#![allow(clippy::type_complexity)]

use std::f32;

use bevy::{platform::collections::HashSet, prelude::*};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use bevy_rand::{
    global::GlobalRng,
    prelude::{EntropyPlugin, WyRand},
};
use clap::Parser;
use rand::RngExt;

use crate::{
    cells::{
        Cell, CellInfo, CellPlugin, Direction, FacingDirection, NewCellBundle, OrganismDepth,
        RemainingTicksWithoutEnergy, SpawnCellMessage, UpdateCellInfoMessage,
    },
    cli::{Cli, Command},
    config::SimulationConfig,
    energy::{CellEnergy, ChargeEnergyEnvironment, OrganicEnergyEnvironment},
    genes::{Genome, GenomeID, RelativeDirection},
    grid::GridPlugin,
    input::SimulationInputPlugin,
    simulation::SimulationGrid,
};

mod cells;
mod cli;
mod config;
mod energy;
mod genes;
mod grid;
mod input;
mod simulation;
mod utils;

const TILE_SIZE: f32 = 10.0;

#[derive(Resource, Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Deref, DerefMut)]
pub struct SimulationStep(pub usize);

#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GridPosition {
    pub x: usize,
    pub y: usize,
}

impl GridPosition {
    /// Returns a new `GridPosition` offset from the current position by the specified deltas.
    pub fn offset(&self, (dx, dy): (isize, isize)) -> Self {
        let new_x = (self.x as isize + dx).max(0) as usize;
        let new_y = (self.y as isize + dy).max(0) as usize;

        Self { x: new_x, y: new_y }
    }

    /// Returns the `GridPosition` one step in the specified absolute direction from the current position.
    pub fn position_in_direction(&self, direction: Direction) -> Self {
        let (dx, dy) = match direction {
            Direction::North => (0, 1),
            Direction::East => (1, 0),
            Direction::South => (0, -1),
            Direction::West => (-1, 0),
        };

        self.offset((dx, dy))
    }

    /// Returns the `GridPosition` one step in the direction relative to the
    /// specified facing direction from the current position.
    pub fn position_in_relative_direction(
        &self,
        facing: Direction,
        relative: RelativeDirection,
    ) -> Self {
        let absolute_direction = facing.relative(relative);
        self.position_in_direction(absolute_direction)
    }

    // Returns the valid 3x3 grid of positions centered on this position, ordered from
    // top-left to bottom-right. Out-of-bounds positions will excluded.
    pub fn neighbourhood(&self) -> Vec<Self> {
        let mut neighbors = Vec::with_capacity(9);
        for dy in -1..=1 {
            for dx in -1..=1 {
                let new_x = self.x.saturating_add_signed(dx);
                let new_y = self.y.saturating_add_signed(dy);

                neighbors.push(Self { x: new_x, y: new_y });
            }
        }

        neighbors
    }
}

#[derive(Resource, Reflect, Clone, Copy, Debug)]
pub struct GridVisible(pub bool);

impl Default for GridVisible {
    fn default() -> Self {
        Self(true)
    }
}

#[derive(States, Reflect, Default, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SimulationState {
    #[default]
    Running,
    Paused,
}

#[derive(Default, Reflect, Clone, Debug)]
pub enum SimulationView {
    #[default]
    Grid,
    OrganicEnergy,
    ChargeEnergy,
}

#[derive(Resource, Reflect, Clone, Debug)]
pub struct SimulationSettings {
    pub config: SimulationConfig,
    pub view: SimulationView,
}

#[derive(Resource, Reflect, Default, Clone, Debug, Deref, DerefMut)]
pub struct CellPositions(pub HashSet<GridPosition>);

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Run { config } => {
            let config = if let Some(config_path) = config {
                std::fs::read_to_string(config_path)
                    .ok()
                    .and_then(|contents| toml::from_str(&contents).ok())
                    .unwrap_or_else(|| {
                        eprintln!("Failed to read or parse config file, using default config");
                        SimulationConfig::default()
                    })
            } else {
                SimulationConfig::default()
            };

            run_simulation(config);
        }
        Command::Config { output } => {
            let toml_config = toml::to_string_pretty(&SimulationConfig::default())
                .expect("Failed to serialize default config");

            if let Some(output_path) = output {
                std::fs::write(output_path, toml_config)
                    .expect("Failed to write default config to file");

                return;
            }

            println!("{}", toml_config);
        }
    }
}

fn run_simulation(config: SimulationConfig) {
    let simulation_settings = SimulationSettings {
        config,
        view: SimulationView::Grid,
    };

    let organic_energy_env = OrganicEnergyEnvironment::new(
        config.simulation.width,
        config.simulation.height,
        config.environment.initial_organic_energy,
        config.toxicity_thresholds.organic_toxic_threshold,
    );

    let charge_energy_env = ChargeEnergyEnvironment::new(
        config.simulation.width,
        config.simulation.height,
        config.environment.initial_charge_energy,
        config.toxicity_thresholds.charge_toxic_threshold,
    );

    let simulation_grid = SimulationGrid::new(
        config.simulation.width,
        config.simulation.height,
        simulation::GridBoundary::Fixed,
    );

    App::new()
        .add_plugins((
            DefaultPlugins,
            MeshPickingPlugin,
            EntropyPlugin::<WyRand>::default(),
            EguiPlugin::default(),
            SimulationInputPlugin,
            CellPlugin,
            GridPlugin,
        ))
        .insert_resource(simulation_settings)
        .insert_resource(organic_energy_env)
        .insert_resource(charge_energy_env)
        .insert_resource(simulation_grid)
        .insert_resource(Time::<Fixed>::from_hz(config.simulation.tick_rate as f64))
        .init_state::<SimulationState>()
        .init_resource::<SimulationStep>()
        .init_resource::<CellPositions>()
        .init_resource::<GridVisible>()
        .add_message::<UpdateCellInfoMessage>()
        .add_systems(EguiPrimaryContextPass, cell_info_ui_system)
        .add_systems(
            Startup,
            (
                setup_camera_system,
                initialize_sprouts_system,
                // add_test_cells,
            ),
        )
        .add_systems(
            FixedUpdate,
            (shuffle_cells_system,).run_if(in_state(SimulationState::Running)),
        )
        .add_systems(Update, draw_grid_gizmos_system)
        .add_systems(
            PostUpdate,
            (|mut step: ResMut<SimulationStep>| {
                step.0 += 1;
            })
            .run_if(in_state(SimulationState::Running)),
        )
        .run();
}

fn setup_camera_system(mut commands: Commands, simulation_settings: Res<SimulationSettings>) {
    // Spawn a 2D camera centered on the grid
    let half_w = simulation_settings.config.simulation.width as f32 * 10.0 / 2.0;
    let half_h = simulation_settings.config.simulation.height as f32 * 10.0 / 2.0;
    commands.spawn((
        Camera2d,
        Transform::from_translation(Vec3::new(half_w, half_h, 0.0)),
    ));
}

fn cell_info_ui_system(
    mut contexts: EguiContexts,
    cells: Query<CellInfo>,
    mut reader: MessageReader<UpdateCellInfoMessage>,
    mut last_hovered_cell: Local<Option<Entity>>,
) -> Result {
    for msg in reader.read() {
        if let Some(cell_entity) = msg.cell {
            *last_hovered_cell = Some(cell_entity);
        } else {
            *last_hovered_cell = None;
        }
    }

    if let Some(entity) = last_hovered_cell.as_ref()
        && let Ok(cell_info) = cells.get(*entity)
    {
        egui::Window::new("Cell Info").show(contexts.ctx_mut()?, |ui| {
            ui.label(format!(
                "Position: ({}, {})",
                cell_info.position.x, cell_info.position.y
            ));
            ui.label(format!("Type: {:?}", cell_info.cell));
            ui.label(format!("Energy: {:?}", cell_info.energy.0));
            ui.label(format!("Facing: {:?}", cell_info.facing));
            ui.label(format!("Genome ID: {:?}", cell_info.genome_id));
            ui.label(format!(
                "Genome Spawn: {:?}",
                cell_info
                    .genome
                    .get_entry(*cell_info.genome_id)
                    .spawn
                    .into_iter()
                    .collect::<Vec<_>>()
            ));
        });
    }

    Ok(())
}

fn initialize_sprouts_system(
    settings: Res<SimulationSettings>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    mut spawn_writer: MessageWriter<SpawnCellMessage>,
) {
    let genome: Genome = rng.random();

    let height = settings.config.simulation.height;
    let width = settings.config.simulation.width;
    let sprouts = settings.config.simulation.initial_sprout_count;
    let initial_energy = settings.config.simulation.initial_sprout_energy;
    let remaining_ticks_without_energy =
        RemainingTicksWithoutEnergy(settings.config.cell_defaults.max_ticks_without_energy);

    let mut positions = HashSet::with_capacity(sprouts);
    while positions.len() < sprouts {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);
        positions.insert((x, y));
    }

    for &(x, y) in &positions {
        let facing_direction = rng.random::<FacingDirection>();
        let genome_id = rng.random::<GenomeID>();
        spawn_writer.write(SpawnCellMessage {
            parent: None,
            new_cell: NewCellBundle {
                grid_pos: GridPosition { x, y },
                facing_direction,
                cell: Cell::Sprout,
                cell_energy: CellEnergy(initial_energy),
                genome: genome.clone(),
                genome_id,
                organism_depth: OrganismDepth(0),
                remaining_ticks_without_energy,
            },
        });
    }
}

fn shuffle_cells_system(
    mut query: Query<(&mut GridPosition, &mut Transform), With<Cell>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    settings: Res<SimulationSettings>,
) {
    let height = settings.config.simulation.height;
    let width = settings.config.simulation.width;

    info!("Shuffling cells");
    let mut positions: HashSet<(usize, usize)> = HashSet::new();
    for (mut pos, mut transform) in query.iter_mut() {
        loop {
            let x = rng.random_range(0..width);
            let y = rng.random_range(0..height);

            if !positions.contains(&(x, y)) {
                positions.insert((x, y));

                pos.x = x;
                pos.y = y;

                transform.translation.x = x as f32 * TILE_SIZE;
                transform.translation.y = y as f32 * TILE_SIZE;

                break;
            }
        }
    }
}

// pub fn add_test_cells(mut commands: Commands) {
//     spawn_cell(
//         &mut commands,
//         Cell::Leaf,
//         GridPosition { x: 10, y: 10 },
//         FacingDirection(Direction::North),
//         CellEnergy(5),
//         rand::rng().random::<Genome>(),
//         rand::rng().random::<GenomeID>(),
//     );
//
//     spawn_cell(
//         &mut commands,
//         Cell::Antenna,
//         GridPosition { x: 11, y: 11 },
//         FacingDirection(Direction::East),
//         CellEnergy(5),
//         rand::rng().random::<Genome>(),
//         rand::rng().random::<GenomeID>(),
//     );
//
//     spawn_cell(
//         &mut commands,
//         Cell::Sprout,
//         GridPosition { x: 12, y: 12 },
//         FacingDirection(Direction::North),
//         CellEnergy(5),
//         rand::rng().random::<Genome>(),
//         rand::rng().random::<GenomeID>(),
//     );
//
//     spawn_cell(
//         &mut commands,
//         Cell::Root,
//         GridPosition { x: 13, y: 13 },
//         FacingDirection(Direction::South),
//         CellEnergy(5),
//         rand::rng().random::<Genome>(),
//         rand::rng().random::<GenomeID>(),
//     );
//
//     spawn_cell(
//         &mut commands,
//         Cell::Branch,
//         GridPosition { x: 14, y: 14 },
//         FacingDirection(Direction::West),
//         CellEnergy(5),
//         rand::rng().random::<Genome>(),
//         rand::rng().random::<GenomeID>(),
//     );
//
//     spawn_cell(
//         &mut commands,
//         Cell::Seed(Seed::DormantSeed),
//         GridPosition { x: 15, y: 15 },
//         FacingDirection(Direction::North),
//         CellEnergy(5),
//         rand::rng().random::<Genome>(),
//         rand::rng().random::<GenomeID>(),
//     );
// }
