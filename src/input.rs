use bevy::{
    ecs::message::MessageReader,
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

use crate::{
    CellInfo, Grid, GridPosition, LastHoveredCell, SimulationSettings, SimulationView,
    ToggleGridVisible, UpdateCellInfoMessage,
    cells::{Cell, CellEnergy, FacingDirection},
    genes::GenomeID,
};

pub struct SimulationInputPlugin;

impl Plugin for SimulationInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                toggle_grid_visibility_system.run_if(resource_exists::<ToggleGridVisible>),
                update_last_hovered_cell_system,
            ),
        )
        .add_systems(Update, (move_camera_system, process_keyboard_system));
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

fn toggle_grid_visibility_system(
    mut commands: Commands,
    mut query: Query<&mut Visibility, With<Grid>>,
) {
    for mut visibility in query.iter_mut() {
        visibility.toggle_visible_hidden();
    }

    commands.remove_resource::<ToggleGridVisible>();
}

fn update_last_hovered_cell_system(
    mut cell_info_events: MessageReader<UpdateCellInfoMessage>,
    mut last_hovered_cell: ResMut<LastHoveredCell>,
) {
    for UpdateCellInfoMessage { cell } in cell_info_events.read() {
        last_hovered_cell.cell_info = cell.clone();
    }
}

pub fn observe_cell_hover(
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
            position: *position,
            cell_type: *cell_type,
            energy: *energy,
            facing: *facing,
            genome_id: *genome_id,
        }),
    });
}

pub fn observe_cell_out(
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
