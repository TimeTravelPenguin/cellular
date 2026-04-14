#![allow(clippy::type_complexity)]

use std::f32;

use avian2d::{self, PhysicsPlugins};
use bevy::{
    dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin},
    platform::collections::HashSet,
    prelude::*,
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use bevy_rand::{
    global::GlobalRng,
    prelude::{EntropyPlugin, WyRand},
};
use clap::Parser;
use rand::RngExt;

use crate::{
    cells::{
        Cell, CellInfo, CellPlugin, FacingDirection, NewCellEvent, SproutCell,
        UpdateCellInfoMessage,
    },
    cli::{Cli, Command},
    config::SimulationConfig,
    energy::{CellEnergy, ChargeEnergyEnvironment, OrganicEnergyEnvironment},
    genes::{Genome, GenomeID},
    input::SimulationInputPlugin,
    simulation::SimulationGrid,
};

mod cells;
mod cli;
mod config;
mod energy;
mod genes;
mod input;
mod simulation;
mod utils;

const TILE_SIZE: f32 = 10.0;
const CELL_GREEN: Color = Color::linear_rgb(23.0 / 255.0, 185.0 / 255.0, 0.0 / 255.0);
const CELL_ORANGE: Color = Color::linear_rgb(235.0 / 255.0, 138.0 / 255.0, 64.0 / 255.0);
const CELL_BLUE: Color = Color::linear_rgb(82.0 / 255.0, 107.0 / 255.0, 1.0);
const CELL_BROWN: Color = Color::linear_rgb(30.0 / 255.0, 20.0 / 255.0, 10.0 / 255.0);

#[derive(Resource, Reflect, Clone, Copy, Debug, Default, PartialEq, Eq, Deref, DerefMut)]
pub struct SimulationStep(pub usize);

#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GridPosition {
    pub x: usize,
    pub y: usize,
}

impl GridPosition {
    pub fn offset(&self, (dx, dy): (isize, isize)) -> Self {
        let new_x = (self.x as isize + dx).max(0) as usize;
        let new_y = (self.y as isize + dy).max(0) as usize;

        Self { x: new_x, y: new_y }
    }
}

#[derive(Component, Reflect, Clone, Debug)]
pub struct Grid;

#[derive(Resource, Reflect, Clone, Debug)]
pub struct ToggleGridVisible;

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
    );

    let charge_energy_env = ChargeEnergyEnvironment::new(
        config.simulation.width,
        config.simulation.height,
        config.environment.initial_charge_energy,
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
            DebugPickingPlugin,
            EntropyPlugin::<WyRand>::default(),
            PhysicsPlugins::default(),
            EguiPlugin::default(),
            SimulationInputPlugin,
            CellPlugin,
        ))
        .insert_resource(DebugPickingMode::Normal)
        .add_systems(
            PreUpdate,
            (|mut mode: ResMut<DebugPickingMode>| {
                *mode = match *mode {
                    DebugPickingMode::Disabled => DebugPickingMode::Normal,
                    DebugPickingMode::Normal => DebugPickingMode::Noisy,
                    DebugPickingMode::Noisy => DebugPickingMode::Disabled,
                }
            })
            .distributive_run_if(bevy::input::common_conditions::input_just_pressed(
                KeyCode::F3,
            )),
        )
        .insert_resource(simulation_settings)
        .insert_resource(organic_energy_env)
        .insert_resource(charge_energy_env)
        .insert_resource(simulation_grid)
        .insert_resource(Time::<Fixed>::from_hz(config.simulation.tick_rate as f64))
        .init_state::<SimulationState>()
        .init_resource::<SimulationStep>()
        .add_message::<UpdateCellInfoMessage>()
        .add_systems(EguiPrimaryContextPass, cell_info_ui_system)
        .add_systems(
            Startup,
            (
                setup_camera_system,
                draw_world_grid_system,
                initialize_sprouts_system,
                // add_test_cells,
            ),
        )
        .add_systems(
            FixedUpdate,
            (shuffle_cells_system,).run_if(in_state(SimulationState::Running)),
        )
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
            ui.label(format!("Type: {:?}", cell_info.cell_type));
            ui.label(format!("Energy: {:?}", cell_info.energy.0));
            ui.label(format!("Facing: {:?}", cell_info.facing));
            ui.label(format!("Genome ID: {:?}", cell_info.genome_id));
        });
    }

    Ok(())
}

fn initialize_sprouts_system(
    settings: Res<SimulationSettings>,
    mut commands: Commands,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
) {
    let genome: Genome = rng.random();

    let height = settings.config.simulation.height;
    let width = settings.config.simulation.width;
    let sprouts = settings.config.simulation.initial_sprout_count;

    let mut positions = HashSet::with_capacity(sprouts);
    while positions.len() < sprouts {
        let x = rng.random_range(0..width);
        let y = rng.random_range(0..height);
        positions.insert((x, y));
    }

    for &(x, y) in &positions {
        let facing_direction = rng.random::<FacingDirection>();
        commands
            .spawn((
                SproutCell,
                CellEnergy(10.0),
                facing_direction,
                GridPosition { x, y },
                rng.random::<GenomeID>(),
                genome.clone(),
            ))
            .trigger(|entity| NewCellEvent {
                entity,
                grid_pos: GridPosition { x, y },
                cell: Cell::Sprout,
                facing_direction,
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

fn draw_world_grid_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    simulation_settings: Res<SimulationSettings>,
) {
    let sim_width = simulation_settings.config.simulation.width;
    let sim_height = simulation_settings.config.simulation.height;

    let grid_width = sim_width as f32 * TILE_SIZE;
    let grid_height = sim_height as f32 * TILE_SIZE;

    let line_color = Color::linear_rgba(1.0, 1.0, 1.0, 0.1);

    // Draw vertical lines
    for x in 0..=sim_width {
        let world_x = (x as f32 - 0.5) * TILE_SIZE;
        let mesh = Segment2d::new(
            Vec2::new(world_x, -0.5 * TILE_SIZE),
            Vec2::new(world_x, grid_height - 0.5 * TILE_SIZE),
        );

        info!("Drawing vertical grid line at x={}", world_x);
        commands.spawn((
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(line_color))),
            Pickable::IGNORE,
            Visibility::Visible,
            Grid,
        ));
    }

    // Draw horizontal lines
    for y in 0..=sim_height {
        let world_y = (y as f32 - 0.5) * TILE_SIZE;
        let mesh = Segment2d::new(
            Vec2::new(-0.5 * TILE_SIZE, world_y),
            Vec2::new(grid_width - 0.5 * TILE_SIZE, world_y),
        );

        info!("Drawing horizontal grid line at y={}", world_y);
        commands.spawn((
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(line_color))),
            Pickable::IGNORE,
            Visibility::Visible,
            Grid,
        ));
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
