//! Demo for the reusable toggle widget.
//!
//! Run with: `cargo run --example toggle`

use bevy::prelude::*;

#[path = "../src/ui/toggle.rs"]
mod toggle;

use toggle::{Toggle, TogglePlugin, ToggleState, ToggleStyle, spawn_toggle};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Size {
    Small,
    Medium,
    Large,
    XLarge,
    SuperUltraMegaLargeAndLong,
}

impl ToggleState for Size {
    fn label(&self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
            Self::XLarge => "X-Large",
            Self::SuperUltraMegaLargeAndLong => "Super Ultra Mega Large and Long",
        }
    }

    fn states() -> &'static [Self] {
        &[
            Self::Small,
            Self::Medium,
            Self::Large,
            Self::XLarge,
            Self::SuperUltraMegaLargeAndLong,
        ]
    }

    fn text_color_selected(&self) -> Option<Color> {
        if *self == Self::SuperUltraMegaLargeAndLong {
            Some(Color::BLACK)
        } else {
            None
        }
    }

    fn text_color_unselected(&self) -> Option<Color> {
        if *self == Self::SuperUltraMegaLargeAndLong {
            Some(Color::srgb(0.6, 0.2, 0.2))
        } else {
            None
        }
    }

    fn indicator_color(&self) -> Option<Color> {
        if *self == Self::SuperUltraMegaLargeAndLong {
            Some(Color::srgb(1.0, 0.0, 0.0))
        } else {
            None
        }
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(TogglePlugin::<Size>::default())
        .add_systems(Startup, setup)
        .add_systems(Update, log_state_changes)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let container = commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .id();

    let toggle = spawn_toggle::<Size>(&mut commands, Size::Medium, ToggleStyle::default());
    commands.entity(toggle).insert(ChildOf(container));
}

fn log_state_changes(
    toggles: Query<&Toggle<Size>, Changed<Toggle<Size>>>,
    mut last: Local<Option<Size>>,
) {
    for toggle in toggles.iter() {
        if last.as_ref() != Some(&toggle.state) {
            println!("Selected: {:?}", toggle.state);
            *last = Some(toggle.state);
        }
    }
}
