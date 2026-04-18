//! An animated toggle button with multiple states.
//!
//! Each toggle is parameterized over a type `S: ToggleState` — typically an
//! enum — that defines the variants and their labels. The widget renders as a
//! horizontal pill with one button per variant; a smaller pill sits behind the
//! selected variant's label and tweens to the newly-chosen label with an
//! easing curve whenever the state changes.
//!
//! # Usage
//!
//! ```no_run
//! # use bevy::prelude::*;
//! use crate::ui::toggle::{spawn_toggle, Toggle, TogglePlugin, ToggleState, ToggleStyle};
//!
//! #[derive(Clone, Copy, PartialEq, Eq)]
//! enum View { Grid, Organic, Charge }
//!
//! impl ToggleState for View {
//!     fn label(&self) -> &'static str {
//!         match self { Self::Grid => "Grid", Self::Organic => "Organic", Self::Charge => "Charge" }
//!     }
//!     fn states() -> &'static [Self] { &[Self::Grid, Self::Organic, Self::Charge] }
//! }
//!
//! # fn setup(mut commands: Commands) {
//! spawn_toggle::<View>(&mut commands, View::Grid, ToggleStyle::default());
//! # }
//! ```
//!
//! Add `TogglePlugin::<View>::default()` to the app so the widget's systems run.

use std::marker::PhantomData;

use bevy::color::Mix;
use bevy::prelude::{
    AlignItems, App, BackgroundColor, BorderRadius, Button, Changed, Children, Color, Commands,
    Component, ComputedNode, Curve, Display, EaseFunction, EasingCurve, Entity, FlexDirection,
    Interaction, IntoScheduleConfigs, JustifyContent, Node, Pickable, Plugin, PositionType, Query,
    Res, Text, TextColor, TextFont, Time, UiGlobalTransform, UiRect, Update, Val, With, children,
    default,
};
use derive_setters::Setters;

/// Trait implemented by the state type a [`Toggle`] cycles through.
///
/// `S: ToggleState` is usually a `Copy` enum. `states()` defines the display
/// order of the buttons; `label()` supplies the text drawn on each one.
pub trait ToggleState: Copy + Clone + PartialEq + Send + Sync + 'static {
    fn label(&self) -> &'static str;
    fn states() -> &'static [Self];

    /// Text color when this state is the currently-selected one. Falls back to
    /// [`ToggleStyle::text_color_selected`] when `None`.
    fn text_color_selected(&self) -> Option<Color> {
        None
    }

    /// Text color when this state is *not* currently selected. Falls back to
    /// [`ToggleStyle::text_color_unselected`] when `None`.
    fn text_color_unselected(&self) -> Option<Color> {
        None
    }

    /// Per-state indicator pill color override. Falls back to [`ToggleStyle::indicator`] when `None`.
    /// The indicator tweens between consecutive states' colors when it moves.
    fn indicator_color(&self) -> Option<Color> {
        None
    }
}

/// Component on the toggle's root node holding the currently-selected state.
#[derive(Component, Debug, Clone, Copy)]
pub struct Toggle<S: ToggleState> {
    pub state: S,
}

/// Visual and motion parameters for a toggle.
#[derive(Component, Setters, Debug, Clone, Copy)]
pub struct ToggleStyle {
    pub height: f32,
    pub padding: f32,
    pub option_padding_x: f32,
    pub background: Color,
    pub indicator: Color,
    pub text_color_selected: Color,
    pub text_color_unselected: Color,
    pub font_size: f32,
    pub duration: f32,
    pub ease: EaseFunction,
}

impl Default for ToggleStyle {
    fn default() -> Self {
        let background = catppuccin::PALETTE.mocha.colors.crust;
        let background = Color::hsl(
            background.hsl.h as f32,
            background.hsl.s as f32,
            background.hsl.l as f32,
        );

        let indicator = catppuccin::PALETTE.mocha.colors.surface0;
        let indicator = Color::hsl(
            indicator.hsl.h as f32,
            indicator.hsl.s as f32,
            indicator.hsl.l as f32,
        );

        let text_selected = catppuccin::PALETTE.mocha.colors.text;
        let text_color_selected = Color::hsl(
            text_selected.hsl.h as f32,
            text_selected.hsl.s as f32,
            text_selected.hsl.l as f32,
        );

        let text_unselected = catppuccin::PALETTE.mocha.colors.subtext0;
        let text_color_unselected = Color::hsl(
            text_unselected.hsl.h as f32,
            text_unselected.hsl.s as f32,
            text_unselected.hsl.l as f32,
        );

        Self {
            height: 36.0,
            padding: 4.0,
            option_padding_x: 16.0,
            background,
            indicator,
            text_color_selected,
            text_color_unselected,
            font_size: 14.0,
            duration: 0.22,
            ease: EaseFunction::CubicOut,
        }
    }
}

#[derive(Component)]
struct ToggleIndicator {
    toggle: Entity,
    initialized: bool,
}

#[derive(Component)]
struct ToggleTween {
    start_x: f32,
    start_w: f32,
    target_x: f32,
    target_w: f32,
    current_x: f32,
    current_w: f32,
    start_color: Color,
    target_color: Color,
    current_color: Color,
    elapsed: f32,
    duration: f32,
    ease: EaseFunction,
}

#[derive(Component)]
struct ToggleOption<S: ToggleState> {
    toggle: Entity,
    state: S,
}

/// Registers the systems that drive toggle widgets of a given state type.
pub struct TogglePlugin<S: ToggleState>(PhantomData<fn() -> S>);

impl<S: ToggleState> Default for TogglePlugin<S> {
    fn default() -> Self {
        Self(PhantomData)
    }
}

impl<S: ToggleState> Plugin for TogglePlugin<S> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_option_clicks::<S>,
                retarget_on_state_change::<S>,
                update_text_colors::<S>,
                animate_indicator::<S>,
            )
                .chain(),
        );
    }
}

fn option_text_color<S: ToggleState>(state: S, selected: S, style: &ToggleStyle) -> Color {
    if state == selected {
        state
            .text_color_selected()
            .unwrap_or(style.text_color_selected)
    } else {
        state
            .text_color_unselected()
            .unwrap_or(style.text_color_unselected)
    }
}

/// Spawns a toggle widget and returns its root entity.
pub fn spawn_toggle<S: ToggleState>(
    commands: &mut Commands,
    initial: S,
    style: ToggleStyle,
) -> Entity {
    let root = commands
        .spawn((
            Toggle::<S> { state: initial },
            style,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                padding: UiRect::all(Val::Px(style.padding)),
                height: Val::Px(style.height),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(style.background),
        ))
        .id();

    let initial_indicator_color = initial.indicator_color().unwrap_or(style.indicator);

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            ToggleIndicator {
                toggle: root,
                initialized: false,
            },
            ToggleTween {
                start_x: 0.0,
                start_w: 0.0,
                target_x: 0.0,
                target_w: 0.0,
                current_x: 0.0,
                current_w: 0.0,
                start_color: initial_indicator_color,
                target_color: initial_indicator_color,
                current_color: initial_indicator_color,
                elapsed: style.duration,
                duration: style.duration.max(f32::EPSILON),
                ease: style.ease,
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(style.padding),
                left: Val::Px(style.padding),
                height: Val::Px((style.height - 2.0 * style.padding).max(0.0)),
                width: Val::Px(0.0),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(initial_indicator_color),
            Pickable::IGNORE,
        ));

        for state in S::states() {
            let text_color = option_text_color(*state, initial, &style);
            parent.spawn((
                Button,
                ToggleOption::<S> {
                    toggle: root,
                    state: *state,
                },
                Node {
                    padding: UiRect::axes(Val::Px(style.option_padding_x), Val::Px(0.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                children![(
                    Text::new(state.label()),
                    TextFont {
                        font_size: style.font_size,
                        ..default()
                    },
                    TextColor(text_color),
                    Pickable::IGNORE,
                )],
            ));
        }
    });

    root
}

fn handle_option_clicks<S: ToggleState>(
    options: Query<(&Interaction, &ToggleOption<S>), (Changed<Interaction>, With<Button>)>,
    mut toggles: Query<&mut Toggle<S>>,
) {
    for (interaction, option) in options.iter() {
        if matches!(*interaction, Interaction::Pressed)
            && let Ok(mut toggle) = toggles.get_mut(option.toggle)
            && toggle.state != option.state
        {
            toggle.state = option.state;
        }
    }
}

// Returns (left, width) of the option for `state` in the toggle's local logical pixel coords,
// or `None` if layout hasn't produced sizes yet.
fn selected_option_local_rect<S: ToggleState>(
    toggle_entity: Entity,
    selected_state: S,
    roots: &Query<(&ComputedNode, &UiGlobalTransform), With<Toggle<S>>>,
    options: &Query<(&ToggleOption<S>, &ComputedNode, &UiGlobalTransform)>,
) -> Option<(f32, f32)> {
    let (root_node, root_xform) = roots.get(toggle_entity).ok()?;
    if root_node.size.x <= 0.0 {
        return None;
    }

    let (_, opt_node, opt_xform) = options.iter().find(|(opt, node, _)| {
        opt.toggle == toggle_entity && opt.state == selected_state && node.size.x > 0.0
    })?;

    // Global transforms are in physical pixels; convert the option's physical
    // left-edge-relative-to-root into logical pixels for Val::Px.
    let rel_center_x = opt_xform.translation.x - root_xform.translation.x;
    let opt_left_in_root_physical = rel_center_x + (root_node.size.x - opt_node.size.x) / 2.0;
    let scale = root_node.inverse_scale_factor;
    Some((opt_left_in_root_physical * scale, opt_node.size.x * scale))
}

fn retarget_on_state_change<S: ToggleState>(
    changed_toggles: Query<(Entity, &Toggle<S>, &ToggleStyle), Changed<Toggle<S>>>,
    roots: Query<(&ComputedNode, &UiGlobalTransform), With<Toggle<S>>>,
    options: Query<(&ToggleOption<S>, &ComputedNode, &UiGlobalTransform)>,
    mut indicators: Query<(&ToggleIndicator, &mut ToggleTween)>,
) {
    for (toggle_entity, toggle, style) in changed_toggles.iter() {
        let Some((target_x, target_w)) =
            selected_option_local_rect::<S>(toggle_entity, toggle.state, &roots, &options)
        else {
            continue;
        };

        let target_color = toggle.state.indicator_color().unwrap_or(style.indicator);

        for (indicator, mut tween) in indicators.iter_mut() {
            if indicator.toggle != toggle_entity {
                continue;
            }
            tween.start_x = tween.current_x;
            tween.start_w = tween.current_w;
            tween.start_color = tween.current_color;
            tween.target_x = target_x;
            tween.target_w = target_w;
            tween.target_color = target_color;
            tween.elapsed = 0.0;
        }
    }
}

fn update_text_colors<S: ToggleState>(
    changed_toggles: Query<(Entity, &Toggle<S>, &ToggleStyle), Changed<Toggle<S>>>,
    options: Query<(&ToggleOption<S>, &Children)>,
    mut texts: Query<&mut TextColor, With<Text>>,
) {
    for (toggle_entity, toggle, style) in changed_toggles.iter() {
        for (option, children) in options.iter() {
            if option.toggle != toggle_entity {
                continue;
            }
            let color = option_text_color(option.state, toggle.state, style);
            for &child in children.iter() {
                if let Ok(mut text_color) = texts.get_mut(child) {
                    text_color.0 = color;
                }
            }
        }
    }
}

fn animate_indicator<S: ToggleState>(
    time: Res<Time>,
    toggles: Query<&Toggle<S>>,
    roots: Query<(&ComputedNode, &UiGlobalTransform), With<Toggle<S>>>,
    options: Query<(&ToggleOption<S>, &ComputedNode, &UiGlobalTransform)>,
    mut indicators: Query<(
        &mut ToggleIndicator,
        &mut ToggleTween,
        &mut Node,
        &mut BackgroundColor,
    )>,
) {
    for (mut indicator, mut tween, mut node, mut bg) in indicators.iter_mut() {
        let Ok(toggle) = toggles.get(indicator.toggle) else {
            continue;
        };

        if !indicator.initialized {
            let Some((x, w)) =
                selected_option_local_rect::<S>(indicator.toggle, toggle.state, &roots, &options)
            else {
                continue;
            };

            tween.start_x = x;
            tween.start_w = w;
            tween.target_x = x;
            tween.target_w = w;
            tween.current_x = x;
            tween.current_w = w;
            tween.elapsed = tween.duration;
            indicator.initialized = true;
        }

        if tween.elapsed < tween.duration {
            tween.elapsed = (tween.elapsed + time.delta_secs()).min(tween.duration);
            let t = tween.elapsed / tween.duration;
            tween.current_x =
                EasingCurve::new(tween.start_x, tween.target_x, tween.ease).sample_clamped(t);
            tween.current_w =
                EasingCurve::new(tween.start_w, tween.target_w, tween.ease).sample_clamped(t);
            let eased_t = EasingCurve::new(0.0_f32, 1.0, tween.ease).sample_clamped(t);
            tween.current_color = tween.start_color.mix(&tween.target_color, eased_t);
        }

        node.left = Val::Px(tween.current_x);
        node.width = Val::Px(tween.current_w);
        bg.0 = tween.current_color;
    }
}
