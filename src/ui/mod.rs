use bevy::prelude::*;

use crate::{SimulationSettings, SimulationState};

pub mod toggle;

#[derive(Debug, Copy, Clone)]
pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, init_ui_system).add_systems(
            Update,
            (
                update_paused_text_system
                    .run_if(state_exists::<SimulationState>)
                    .run_if(state_changed::<SimulationState>),
                update_speed_text_system
                    .run_if(resource_exists::<SimulationSettings>)
                    .run_if(resource_changed::<SimulationSettings>),
            ),
        );
    }
}

#[derive(Component)]
struct SimulationPausedText;

#[derive(Component)]
struct SimulationSpeedText;

#[derive(Component)]
struct SimulationZoomText;

#[derive(Component)]
struct GridVisibleText;

fn paused_text_string(paused: bool) -> String {
    if paused {
        "Paused".to_string()
    } else {
        String::new()
    }
}

fn init_ui_system(mut commands: Commands) {
    // Simulation Paused Text
    commands.spawn((
        Text::default(),
        TextLayout::new_with_justify(Justify::Right),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(10),
            right: px(10),
            ..default()
        },
        TextBackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
        SimulationPausedText,
    ));

    // Simulation Speed Text
    commands
        .spawn((
            Text::new("Ticks/s: "),
            TextLayout::new_with_justify(Justify::Left),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(10),
                left: px(10),
                ..default()
            },
        ))
        .with_child((TextSpan::default(), SimulationSpeedText));
}

fn update_paused_text_system(
    simulation_state: Res<State<SimulationState>>,
    mut simulation_paused_text: Single<&mut Text, With<SimulationPausedText>>,
) {
    let paused = matches!(simulation_state.get(), SimulationState::Paused);
    let paused_text = paused_text_string(paused);

    **simulation_paused_text = Text::from(paused_text);
}

fn update_speed_text_system(
    simulation_settings: Res<SimulationSettings>,
    mut simulation_speed_text: Single<&mut TextSpan, With<SimulationSpeedText>>,
) {
    let speed_text = simulation_settings.config.simulation.tick_rate.to_string();
    **simulation_speed_text = TextSpan::from(speed_text);
}
