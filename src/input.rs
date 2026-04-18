use bevy::{
    ecs::message::MessageReader,
    input::mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};

use crate::{
    GridVisible, SimulationSettings, SimulationState, SimulationView, UpdateCellInfoMessage,
    cells::{Cell, NewCellEvent},
};

#[derive(Message)]
struct Pause(bool);

pub struct SimulationInputPlugin;

impl Plugin for SimulationInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Pause>()
            .add_systems(PreUpdate, pause_system)
            .add_systems(Update, (move_camera_system, process_keyboard_system))
            .add_observer(|event: On<NewCellEvent>, mut commands: Commands| {
                commands
                    .entity(event.0)
                    .observe(observe_cell_hover)
                    .observe(observe_cell_out);
            });
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
    mut simulation_settings: ResMut<SimulationSettings>,
    mut grid_visible: ResMut<GridVisible>,
    mut time: ResMut<Time<Fixed>>,
    simulation_state: Res<State<SimulationState>>,
    mut pause_writer: MessageWriter<Pause>,
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
        grid_visible.0 = !grid_visible.0;
        info!("Toggled grid visibility to {}", grid_visible.0);
    }

    if keyboard.just_pressed(KeyCode::Space) {
        match simulation_state.get() {
            SimulationState::Running => {
                pause_writer.write(Pause(true));
            }
            SimulationState::Paused => {
                pause_writer.write(Pause(false));
            }
        }
    }

    let tick_rate = &mut simulation_settings.config.simulation.tick_rate;
    if keyboard.just_pressed(KeyCode::Equal) {
        *tick_rate = (*tick_rate).saturating_mul(2).min(2u32.pow(10));
        time.set_timestep_hz(*tick_rate as f64);

        info!(
            "Increased simulation speed to {} ticks per second",
            tick_rate
        );
    } else if keyboard.just_pressed(KeyCode::Minus) {
        *tick_rate = (*tick_rate).saturating_div(2).max(1);
        time.set_timestep_hz(*tick_rate as f64);

        info!(
            "Decreased simulation speed to {} ticks per second",
            tick_rate
        );
    }
}

fn pause_system(
    mut time: ResMut<Time<Virtual>>,
    mut next_state: ResMut<NextState<SimulationState>>,
    mut pause_reader: MessageReader<Pause>,
) {
    for pause in pause_reader.read() {
        if pause.0 {
            time.pause();
            next_state.set(SimulationState::Paused);
            info!("Simulation paused");
        } else {
            time.unpause();
            next_state.set(SimulationState::Running);
            info!("Simulation resumed");
        }
    }
}

pub fn observe_cell_hover(
    event: On<Pointer<Over>>,
    mut writer: MessageWriter<UpdateCellInfoMessage>,
) {
    writer.write(UpdateCellInfoMessage {
        cell: Some(event.entity),
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
