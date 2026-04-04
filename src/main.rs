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
use rand::RngExt;

use crate::{
    cells::{
        Cell, CellEnergy, Direction, FacingDirection, SeedCell, cell_collect_charge_energy_system,
        cell_collect_organic_energy_system, cell_collect_solar_energy_system,
        cell_request_energy_system, cell_transform, insert_cell_visual,
        invoke_cell_genome_actions_system, kill_toxic_cells_system, transfer_energy_system,
    },
    energy::{
        ChargeEnergyEnvironment, OrganicEnergyEnvironment, SunlightCycle, charge_energy_system,
    },
    genes::{Genome, GenomeID},
    input::{SimulationInputPlugin, observe_cell_hover, observe_cell_out},
};

mod cells;
mod cli;
mod energy;
mod genes;
mod input;
mod simulation;

const TILE_SIZE: f32 = 10.0;
const CELL_GREEN: Color = Color::linear_rgb(23.0 / 255.0, 185.0 / 255.0, 0.0 / 255.0);
const CELL_ORANGE: Color = Color::linear_rgb(235.0 / 255.0, 138.0 / 255.0, 64.0 / 255.0);
const CELL_BLUE: Color = Color::linear_rgb(82.0 / 255.0, 107.0 / 255.0, 1.0);
const CELL_BROWN: Color = Color::linear_rgb(30.0 / 255.0, 20.0 / 255.0, 10.0 / 255.0);

#[derive(Resource, Reflect, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SimulationStep(usize);

#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GridPosition {
    pub x: usize,
    pub y: usize,
}

#[derive(Component, Reflect, Clone, Debug)]
pub struct Grid;

#[derive(Resource, Reflect, Clone, Debug)]
pub struct ToggleGridVisible;

#[derive(Default, Reflect, Clone, Debug)]
pub enum SimulationView {
    #[default]
    Grid,
    OrganicEnergy,
    ChargeEnergy,
}

#[derive(Message)]
pub struct UpdateCellInfoMessage {
    pub cell: Option<CellInfo>,
}

#[derive(Component, Reflect, Clone, Debug)]
pub struct CellInfo {
    position: GridPosition,
    cell_type: Cell,
    energy: CellEnergy,
    facing: FacingDirection,
    genome_id: GenomeID,
}

#[derive(Resource, Reflect, Default, Clone, Debug)]
pub struct LastHoveredCell {
    pub cell_info: Option<CellInfo>,
}

#[derive(Resource, Reflect, Clone, Debug)]
pub struct SimulationSettings {
    pub speed_multiplier: f32,
    pub grid_height: usize,
    pub grid_width: usize,
    pub initial_sprout_count: usize,
    pub view: SimulationView,
}

impl Default for SimulationSettings {
    fn default() -> Self {
        let grid_height = 50;
        let grid_width = 100;
        Self {
            grid_height,
            grid_width,
            speed_multiplier: 10.0,
            initial_sprout_count: grid_height * grid_width / 20,
            view: SimulationView::Grid,
        }
    }
}

// Simulation Overview:
// Each cell is one of the following types:
// - Single Cellular: This consists only of a seed that may or may not be
//     attached to a parent cell. This cell executes the genome and can perform
//     actions.
// - Multi Cellular:
//   - Sprout: This cell executes the genome and can perform actions, such as
//       producing new child cells in adjacent grid spaces.
//   - Leaf: Collects energy from sunlight.
//   - Antenna: Collects energy from charge in the environment.
//   - Root: Collects energy from organic matter in the environment.
//   - Branch: A cell that connects other cells together, transferring energy
//       between all connected cells.

// 1. Step through each gene-executing cell and invoke their genome to get a
//    command. Store this command as a component on the cell entity.
// 2. Process each genome command component, applying its effects to the cell
//    and/or environment, and then remove the command component.
// 3. Perform actions of non-genome-command cells.
// 4. Perform a step in energy transfer between connected cells.

fn main() {
    let simulation_settings = SimulationSettings::default();

    let organic_energy_env = OrganicEnergyEnvironment::new(
        simulation_settings.grid_width,
        simulation_settings.grid_height,
        50,
    );

    let charge_energy_env = ChargeEnergyEnvironment::new(
        simulation_settings.grid_width,
        simulation_settings.grid_height,
        20,
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
        .init_resource::<SunlightCycle>()
        .init_resource::<SimulationStep>()
        .init_resource::<LastHoveredCell>()
        .add_message::<UpdateCellInfoMessage>()
        .add_systems(EguiPrimaryContextPass, cell_info_ui_system)
        .add_systems(
            Startup,
            (
                setup_camera_system,
                draw_world_grid_system,
                // initialize_sprouts_system,
                add_test_cells,
            ),
        )
        .add_systems(Update, (draw_cells_system,))
        .add_systems(
            Update,
            (
                (
                    invoke_cell_genome_actions_system,
                    transfer_energy_system,
                    cell_request_energy_system,
                    cell_collect_solar_energy_system,
                    cell_collect_organic_energy_system,
                    cell_collect_charge_energy_system,
                    kill_toxic_cells_system,
                )
                    .chain(),
                charge_energy_system,
                // shuffle_cells_system,
            ),
        )
        .add_systems(PostUpdate, |mut step: ResMut<SimulationStep>| {
            step.0 += 1;
        })
        .run();
}

fn setup_camera_system(mut commands: Commands, simulation_settings: Res<SimulationSettings>) {
    // Spawn a 2D camera centered on the grid
    let half_w = simulation_settings.grid_width as f32 * 10.0 / 2.0;
    let half_h = simulation_settings.grid_height as f32 * 10.0 / 2.0;
    commands.spawn((
        Camera2d,
        Transform::from_translation(Vec3::new(half_w, half_h, 0.0)),
    ));
}

fn cell_info_ui_system(
    mut contexts: EguiContexts,
    last_hovered_cell: Res<LastHoveredCell>,
) -> Result {
    if let Some(cell) = last_hovered_cell.cell_info.as_ref() {
        egui::Window::new("Cell Info").show(contexts.ctx_mut()?, |ui| {
            ui.label(format!(
                "Position: ({}, {})",
                cell.position.x, cell.position.y
            ));
            ui.label(format!("Type: {:?}", cell.cell_type));
            ui.label(format!("Energy: {}", cell.energy.0));
            ui.label(format!("Facing: {:?}", cell.facing.0));
            ui.label(format!("Genome ID: {}", cell.genome_id.0));
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

    let mut positions = HashSet::new();
    while positions.len() < settings.initial_sprout_count {
        let x = rng.random_range(0..settings.grid_width);
        let y = rng.random_range(0..settings.grid_height);
        positions.insert((x, y));
    }

    for (x, y) in positions {
        info!("Spawning initial sprout at ({}, {})", x, y);
        commands.spawn((
            Cell::Sprout,
            CellEnergy(10),
            rng.random::<FacingDirection>(),
            GridPosition { x, y },
            rng.random::<GenomeID>(),
            genome.clone(),
        ));
    }
}

fn shuffle_cells_system(
    mut query: Query<(&mut GridPosition, &mut Transform), With<Cell>>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    settings: Res<SimulationSettings>,
) {
    info!("Shuffling cells");
    let mut positions: HashSet<(usize, usize)> = HashSet::new();
    for (mut pos, mut transform) in query.iter_mut() {
        loop {
            let x = rng.random_range(0..settings.grid_width);
            let y = rng.random_range(0..settings.grid_height);

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
    let grid_width = simulation_settings.grid_width as f32 * TILE_SIZE;
    let grid_height = simulation_settings.grid_height as f32 * TILE_SIZE;

    let line_color = Color::linear_rgba(1.0, 1.0, 1.0, 0.1);

    // Draw vertical lines
    for x in 0..=simulation_settings.grid_width {
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
    for y in 0..=simulation_settings.grid_height {
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

pub fn add_test_cells(mut commands: Commands) {
    info!("Spawning test Leaf cell");
    commands.spawn((
        Cell::Leaf,
        CellEnergy(5),
        FacingDirection(Direction::North),
        GridPosition { x: 10, y: 10 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));

    info!("Spawning test Antenna cell");
    commands.spawn((
        Cell::Antenna,
        CellEnergy(5),
        FacingDirection(Direction::East),
        GridPosition { x: 11, y: 11 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));

    info!("Spawning test Sprout cell");
    commands.spawn((
        Cell::Sprout,
        CellEnergy(5),
        FacingDirection(Direction::North),
        GridPosition { x: 12, y: 12 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));

    info!("Spawning test Root cell");
    commands.spawn((
        Cell::Root,
        CellEnergy(5),
        FacingDirection(Direction::South),
        GridPosition { x: 13, y: 13 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));

    info!("Spawning test Branch cell");
    commands.spawn((
        Cell::Branch,
        CellEnergy(5),
        FacingDirection(Direction::West),
        GridPosition { x: 14, y: 14 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));

    info!("Spawning test Dormant Seed cell");
    commands.spawn((
        Cell::Seed(SeedCell::DormantSeed),
        CellEnergy(5),
        FacingDirection(Direction::North),
        GridPosition { x: 15, y: 15 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));
}

pub fn draw_cells_system(
    mut commands: Commands,
    cells: Query<(Entity, &GridPosition, &FacingDirection, &Cell), Without<Mesh2d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, grid_pos, facing_direction, cell) in &cells {
        let transform = cell_transform(grid_pos, facing_direction.0);
        let spec = cell.visual_spec();

        info!(
            "Spawning cell at ({}, {}) of type {:?}",
            grid_pos.x, grid_pos.y, cell,
        );

        let mut entity_commands = commands.entity(entity);
        insert_cell_visual(
            &mut entity_commands,
            spec,
            transform,
            *grid_pos,
            &mut meshes,
            &mut materials,
        );

        entity_commands
            .observe(observe_cell_hover)
            .observe(observe_cell_out);
    }
}
