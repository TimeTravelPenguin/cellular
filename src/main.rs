#![allow(clippy::type_complexity)]

use std::f32;

use avian2d::{self, PhysicsPlugins};
use bevy::{
    dev_tools::picking_debug::{DebugPickingMode, DebugPickingPlugin},
    ecs::message::MessageReader,
    input::mouse::{MouseScrollUnit, MouseWheel},
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
        Cell, CellEnergy, Direction, FacingDirection, SeedCell, invoke_cell_actions_system,
        transfer_energy_system,
    },
    energy::SimulationEnvironment,
    genes::{Genome, GenomeEntry, GenomeID},
};

mod cells;
mod cli;
mod energy;
mod genes;
mod simulation;

const TILE_SIZE: f32 = 10.0;
const CELL_GREEN: Color = Color::linear_rgb(23.0 / 255.0, 185.0 / 255.0, 0.0 / 255.0);
const CELL_ORANGE: Color = Color::linear_rgb(235.0 / 255.0, 138.0 / 255.0, 64.0 / 255.0);
const CELL_BLUE: Color = Color::linear_rgb(82.0 / 255.0, 107.0 / 255.0, 1.0);

#[derive(Component, Reflect, Clone, Debug, PartialEq, Eq)]
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
    let mut rng = rand::rng();
    let x: GenomeEntry = rng.random();

    // serialize to json to file
    let json = serde_json::to_string_pretty(&x).unwrap();
    std::fs::write("genome_entry.json", json).unwrap();

    let mut simulation_settings = SimulationSettings::default();
    simulation_settings.initial_sprout_count = 1;
    let environment = SimulationEnvironment::new(
        simulation_settings.grid_width,
        simulation_settings.grid_height,
        10,
        10,
    );

    App::new()
        .add_plugins((
            DefaultPlugins,
            MeshPickingPlugin,
            DebugPickingPlugin,
            EntropyPlugin::<WyRand>::default(),
            PhysicsPlugins::default(),
            EguiPlugin::default(),
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
        .insert_resource(environment)
        .init_resource::<LastHoveredCell>()
        .add_message::<UpdateCellInfoMessage>()
        .add_systems(EguiPrimaryContextPass, cell_info_ui_system)
        .add_systems(
            Startup,
            (
                setup_camera_system,
                initialize_sprouts_system,
                draw_world_grid_system,
                // add_cell,
            ),
        )
        .add_systems(
            PreUpdate,
            (
                toggle_grid_visibility_system.run_if(resource_exists::<ToggleGridVisible>),
                update_last_hovered_cell_system,
            ),
        )
        .add_systems(
            Update,
            (
                move_camera_system,
                process_keyboard_system,
                draw_cells_system,
            ),
        )
        .add_systems(
            Update,
            (
                (invoke_cell_actions_system, transfer_energy_system).chain(),
                // shuffle_cells_system,
            ),
        )
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
    egui::Window::new("Closest Cell Info").show(contexts.ctx_mut()?, |ui| {
        if let Some(cell) = last_hovered_cell.cell_info.as_ref() {
            ui.label(format!(
                "Position: ({}, {})",
                cell.position.x, cell.position.y
            ));
            ui.label(format!("Type: {:?}", cell.cell_type));
            ui.label(format!("Energy: {}", cell.energy.0));
            ui.label(format!("Facing: {:?}", cell.facing.0));
            ui.label(format!("Genome ID: {}", cell.genome_id.0));
        }
    });

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

fn process_keyboard_system(
    mut commands: Commands,
    mut simulation_settings: ResMut<SimulationSettings>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if keyboard.just_pressed(KeyCode::KeyG) {
        info!("Switched to Grid view");
        simulation_settings.view = SimulationView::Grid;
    } else if keyboard.just_pressed(KeyCode::KeyO) {
        info!("Switched to Organic Energy view");
        simulation_settings.view = SimulationView::OrganicEnergy;
    } else if keyboard.just_pressed(KeyCode::KeyC) {
        info!("Switched to Charge Energy view");
        simulation_settings.view = SimulationView::ChargeEnergy;
    } else if keyboard.just_pressed(KeyCode::Tab) && !keyboard.pressed(KeyCode::ShiftLeft) {
        info!("Cycled view forward");
        simulation_settings.view = match simulation_settings.view {
            SimulationView::Grid => SimulationView::OrganicEnergy,
            SimulationView::OrganicEnergy => SimulationView::ChargeEnergy,
            SimulationView::ChargeEnergy => SimulationView::Grid,
        };
    } else if keyboard.just_pressed(KeyCode::Tab) && keyboard.pressed(KeyCode::ShiftLeft) {
        info!("Cycled view backward");
        simulation_settings.view = match simulation_settings.view {
            SimulationView::Grid => SimulationView::ChargeEnergy,
            SimulationView::OrganicEnergy => SimulationView::Grid,
            SimulationView::ChargeEnergy => SimulationView::OrganicEnergy,
        };
    }

    if keyboard.just_pressed(KeyCode::KeyH) {
        info!("Toggled grid visibility");
        commands.insert_resource(ToggleGridVisible);
    }

    if keyboard.just_pressed(KeyCode::Equal) {
        simulation_settings.speed_multiplier += 0.5;
        info!(
            "Increased simulation speed to {}x",
            simulation_settings.speed_multiplier
        );
    } else if keyboard.just_pressed(KeyCode::Minus) {
        simulation_settings.speed_multiplier =
            (simulation_settings.speed_multiplier - 0.5).max(0.5);
        info!(
            "Decreased simulation speed to {}x",
            simulation_settings.speed_multiplier
        );
    }
}

fn move_camera_system(
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    mut cursor_events: MessageReader<CursorMoved>,
    mut scroll_events: MessageReader<MouseWheel>,
    mut transform: Single<&mut Transform, With<Camera2d>>,
    mut last_cursor_pos: Local<Option<Vec2>>,
) {
    let scale = transform.scale.x;
    let speed = 500.0 * scale * time.delta_secs();

    // WASD movement
    let mut direction = Vec3::ZERO;
    if keyboard.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    if direction != Vec3::ZERO {
        transform.translation += direction.normalize() * speed;
    }

    // Mouse drag panning
    if mouse_button.pressed(MouseButton::Left) {
        for event in cursor_events.read() {
            if let Some(last_pos) = *last_cursor_pos {
                let delta = event.position - last_pos;
                transform.translation.x -= delta.x * scale;
                transform.translation.y += delta.y * scale;
            }
            *last_cursor_pos = Some(event.position);
        }
    } else {
        for event in cursor_events.read() {
            *last_cursor_pos = Some(event.position);
        }
    }

    // Scroll wheel zoom
    for event in scroll_events.read() {
        let scroll_amount = match event.unit {
            MouseScrollUnit::Line => event.y,
            MouseScrollUnit::Pixel => event.y / 100.0,
        };

        let zoom_factor = 1.0 - scroll_amount * 0.1;
        transform.scale *= Vec3::splat(zoom_factor);
        transform.scale = transform.scale.clamp(Vec3::splat(0.05), Vec3::splat(5.0));
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

        commands.spawn((
            Mesh2d(meshes.add(mesh)),
            MeshMaterial2d(materials.add(ColorMaterial::from_color(line_color))),
            Pickable::IGNORE,
            Visibility::Visible,
            Grid,
        ));
    }
}

fn toggle_grid_visibility_system(
    mut commands: Commands,
    mut query: Query<&mut Visibility, With<Grid>>,
) {
    for mut visibility in query.iter_mut() {
        visibility.toggle_visible_hidden();
    }

    commands.remove_resource::<ToggleGridVisible>();
}

pub fn add_cell(mut commands: Commands) {
    commands.spawn((
        Cell::Leaf,
        CellEnergy(100),
        FacingDirection(Direction::North),
        GridPosition { x: 10, y: 10 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));

    commands.spawn((
        Cell::Antenna,
        CellEnergy(100),
        FacingDirection(Direction::East),
        GridPosition { x: 11, y: 11 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));

    commands.spawn((
        Cell::Sprout,
        CellEnergy(100),
        FacingDirection(Direction::North),
        GridPosition { x: 12, y: 12 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));

    commands.spawn((
        Cell::Root,
        CellEnergy(100),
        FacingDirection(Direction::South),
        GridPosition { x: 13, y: 13 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));

    commands.spawn((
        Cell::Branch,
        CellEnergy(100),
        FacingDirection(Direction::West),
        GridPosition { x: 14, y: 14 },
        rand::rng().random::<Genome>(),
        rand::rng().random::<GenomeID>(),
    ));

    commands.spawn((
        Cell::Seed(SeedCell::DormantSeed),
        CellEnergy(100),
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
    for (entity, grid_pos, facing_direction, cell) in cells.iter() {
        let world_x = grid_pos.x as f32 * TILE_SIZE;
        let world_y = grid_pos.y as f32 * TILE_SIZE;

        let mut transform = Transform::from_translation(Vec3::new(world_x, world_y, 1.0));

        match facing_direction.0 {
            Direction::East => {}
            Direction::South => {
                transform.rotation = Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)
            }
            Direction::West => transform.rotation = Quat::from_rotation_z(std::f32::consts::PI),
            Direction::North => {
                transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)
            }
        }

        info!("Drawing cell at grid ({}, {})", grid_pos.x, grid_pos.y);
        let mut entity_commands = commands.entity(entity);

        match cell {
            Cell::Leaf => {
                let mesh = meshes.add(Ellipse::new(TILE_SIZE / 1.75, TILE_SIZE / 3.0));
                let material = materials.add(ColorMaterial::from_color(CELL_GREEN));

                entity_commands.insert((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    grid_pos.clone(),
                    transform,
                ))
            }
            Cell::Antenna => {
                let mesh = meshes.add(Circle::new(TILE_SIZE / 3.0));
                let material = materials.add(ColorMaterial::from_color(CELL_BLUE));

                entity_commands.insert((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    grid_pos.clone(),
                    transform,
                ))
            }
            Cell::Root => {
                let mesh = meshes.add(Rectangle::new(TILE_SIZE / 1.5, TILE_SIZE / 1.5));
                let material = materials.add(ColorMaterial::from_color(CELL_ORANGE));

                entity_commands.insert((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    grid_pos.clone(),
                    transform,
                ))
            }
            Cell::Sprout => {
                let mesh = meshes.add(Circle::new(TILE_SIZE / 3.0));
                let material = materials.add(ColorMaterial::from_color(Color::WHITE));

                let left_eye = meshes.add(Circle::new(TILE_SIZE / 15.0));
                let right_eye = meshes.add(Circle::new(TILE_SIZE / 15.0));

                entity_commands
                    .insert((
                        Mesh2d(mesh),
                        MeshMaterial2d(material),
                        grid_pos.clone(),
                        transform,
                    ))
                    .with_child((
                        Mesh2d(left_eye),
                        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::BLACK))),
                        Transform::from_translation(Vec3::new(
                            TILE_SIZE / 6.0,
                            TILE_SIZE / 6.0,
                            2.0,
                        )),
                    ))
                    .with_child((
                        Mesh2d(right_eye),
                        MeshMaterial2d(materials.add(ColorMaterial::from_color(Color::BLACK))),
                        Transform::from_translation(Vec3::new(
                            TILE_SIZE / 6.0,
                            -TILE_SIZE / 6.0,
                            2.0,
                        )),
                    ))
            }
            Cell::Branch => {
                let mesh = meshes.add(Rectangle::new(TILE_SIZE / 1.5, TILE_SIZE / 6.0));
                let material = materials.add(ColorMaterial::from_color(Color::WHITE));

                entity_commands.insert((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    grid_pos.clone(),
                    transform,
                ))
            }
            Cell::Seed(_) => {
                let mesh = meshes.add(Circle::new(TILE_SIZE / 6.0));
                let material = materials.add(ColorMaterial::from_color(Color::WHITE));

                entity_commands.insert((
                    Mesh2d(mesh),
                    MeshMaterial2d(material),
                    grid_pos.clone(),
                    transform,
                ))
            }
        };

        entity_commands
            .observe(observe_cell_hover)
            .observe(observe_cell_out);
    }
}

fn update_last_hovered_cell_system(
    mut cell_info_events: MessageReader<UpdateCellInfoMessage>,
    mut last_hovered_cell: ResMut<LastHoveredCell>,
) {
    for UpdateCellInfoMessage { cell } in cell_info_events.read() {
        last_hovered_cell.cell_info = cell.clone();
    }
}

fn observe_cell_hover(
    event: On<Pointer<Over>>,
    mut writer: MessageWriter<UpdateCellInfoMessage>,
    query: Query<(
        &GridPosition,
        &Cell,
        &CellEnergy,
        &FacingDirection,
        &GenomeID,
    )>,
) {
    let Ok((position, cell_type, energy, facing, genome_id)) = query.get(event.entity) else {
        warn!("Received pointer over event for non-cell entity");
        return;
    };

    writer.write(UpdateCellInfoMessage {
        cell: Some(CellInfo {
            position: position.clone(),
            cell_type: *cell_type,
            energy: *energy,
            facing: *facing,
            genome_id: *genome_id,
        }),
    });
}

fn observe_cell_out(
    event: On<Pointer<Out>>,
    mut writer: MessageWriter<UpdateCellInfoMessage>,
    cells: Query<&Cell>,
) {
    let Ok(_) = cells.get(event.entity) else {
        warn!("Received pointer out event for non-cell entity");
        return;
    };

    writer.write(UpdateCellInfoMessage { cell: None });
}
