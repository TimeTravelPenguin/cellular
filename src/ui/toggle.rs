#![allow(clippy::type_complexity)]

//! An animated toggle button with multiple states.
//!
//! Each toggle is parameterized over a type `S: ToggleState` — typically an
//! enum — that defines the variants and their labels. The widget renders as a
//! horizontal pill with one button per variant; a smaller pill sits behind the
//! selected variant's label and tweens to the newly-chosen label with an
//! easing curve whenever the state changes.
//!
//! # Styling
//!
//! [`ToggleStyle`] holds the defaults applied to every option. Per-state
//! deviations are expressed through two callbacks stored on the style —
//! [`ToggleStyle::on_selected`] and [`ToggleStyle::on_unselected`] — which
//! receive the current state and the style and return an optional
//! [`StateStyle`] whose `Some` fields override the defaults for that option.
//!
//! ```no_run
//! # use bevy::prelude::*;
//! use crate::ui::toggle::{spawn_toggle, StateStyle, Toggle, TogglePlugin, ToggleState, ToggleStyle};
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
//! let style = ToggleStyle::<View>::default().on_selected(|state, _base| {
//!     matches!(state, View::Charge).then(|| StateStyle {
//!         text_color: Some(Color::BLACK),
//!         indicator_color: Some(Color::srgb(1.0, 0.8, 0.2)),
//!         ..default()
//!     })
//! });
//! spawn_toggle::<View>(&mut commands, View::Grid, style);
//! # }
//! ```
//!
//! Add `TogglePlugin::<View>::default()` to the app so the widget's systems run.

use std::marker::PhantomData;
use std::sync::Arc;

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
}

/// Component on the toggle's root node holding the currently-selected state.
#[derive(Component, Debug, Clone, Copy)]
pub struct Toggle<S: ToggleState> {
    pub state: S,
}

/// Per-state style overrides returned from [`ToggleStyle`]'s `on_selected` /
/// `on_unselected` callbacks. Every field is optional; `None` means "use the
/// base value from [`ToggleStyle`]".
#[derive(Default, Clone, Debug)]
pub struct StateStyle {
    /// Replaces the base [`TextFont`] (font handle, size, line height, etc.).
    pub text_font: Option<TextFont>,
    /// Replaces the base text color.
    pub text_color: Option<Color>,
    /// Replaces the indicator pill color (only honored when the state is
    /// currently selected — the pill tweens to this value).
    pub indicator_color: Option<Color>,
}

type StateStyleFn<S> = dyn Fn(&S, &ToggleStyle<S>) -> Option<StateStyle> + Send + Sync + 'static;

/// Visual and motion parameters for a toggle.
#[derive(Component, Setters, Clone)]
pub struct ToggleStyle<S: ToggleState> {
    pub height: f32,
    pub padding: f32,
    pub option_padding_x: f32,
    pub background: Color,
    pub indicator: Color,
    pub text_color: Color,
    pub text_font: TextFont,
    pub duration: f32,
    pub ease: EaseFunction,
    #[setters(skip)]
    on_selected: Option<Arc<StateStyleFn<S>>>,
    #[setters(skip)]
    on_unselected: Option<Arc<StateStyleFn<S>>>,
    #[setters(skip)]
    _marker: PhantomData<fn() -> S>,
}

impl<S: ToggleState> Default for ToggleStyle<S> {
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

        let text_colour = catppuccin::PALETTE.mocha.colors.text;
        let text_color = Color::hsl(
            text_colour.hsl.h as f32,
            text_colour.hsl.s as f32,
            text_colour.hsl.l as f32,
        );

        Self {
            height: 36.0,
            padding: 4.0,
            option_padding_x: 16.0,
            background,
            indicator,
            text_color,
            text_font: TextFont {
                font_size: 14.0,
                ..default()
            },
            duration: 0.22,
            ease: EaseFunction::CubicOut,
            on_selected: None,
            on_unselected: None,
            _marker: PhantomData,
        }
    }
}

impl<S: ToggleState> ToggleStyle<S> {
    /// Registers a callback invoked for the currently-selected option. Return
    /// `Some(StateStyle { ... })` to override base styling for that state, or
    /// `None` to leave it untouched.
    pub fn on_selected<F>(mut self, cb: F) -> Self
    where
        F: Fn(&S, &ToggleStyle<S>) -> Option<StateStyle> + Send + Sync + 'static,
    {
        self.on_selected = Some(Arc::new(cb));
        self
    }

    /// Registers a callback invoked for every non-selected option.
    pub fn on_unselected<F>(mut self, cb: F) -> Self
    where
        F: Fn(&S, &ToggleStyle<S>) -> Option<StateStyle> + Send + Sync + 'static,
    {
        self.on_unselected = Some(Arc::new(cb));
        self
    }

    fn resolve(&self, state: S, selected: S) -> StateStyle {
        let cb = if state == selected {
            self.on_selected.as_ref()
        } else {
            self.on_unselected.as_ref()
        };
        cb.and_then(|cb| cb(&state, self)).unwrap_or_default()
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
    current_x: f32,
    current_w: f32,
    start_color: Color,
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
                update_text_styling::<S>,
                animate_indicator::<S>,
            )
                .chain(),
        );
    }
}

/// Spawns a toggle widget and returns its root entity.
pub fn spawn_toggle<S: ToggleState>(
    commands: &mut Commands,
    initial: S,
    style: ToggleStyle<S>,
) -> Entity {
    let height = style.height;
    let padding = style.padding;
    let option_padding_x = style.option_padding_x;
    let background = style.background;
    let duration = style.duration.max(f32::EPSILON);
    let ease = style.ease;

    let initial_indicator_color = style
        .resolve(initial, initial)
        .indicator_color
        .unwrap_or(style.indicator);

    let per_option: Vec<(S, TextFont, Color)> = S::states()
        .iter()
        .map(|state| {
            let resolved = style.resolve(*state, initial);
            let text_font = resolved
                .text_font
                .unwrap_or_else(|| style.text_font.clone());
            let text_color = resolved.text_color.unwrap_or(style.text_color);
            (*state, text_font, text_color)
        })
        .collect();

    let root = commands
        .spawn((
            Toggle::<S> { state: initial },
            style,
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Stretch,
                padding: UiRect::all(Val::Px(padding)),
                height: Val::Px(height),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(background),
        ))
        .id();

    commands.entity(root).with_children(|parent| {
        parent.spawn((
            ToggleIndicator {
                toggle: root,
                initialized: false,
            },
            ToggleTween {
                start_x: 0.0,
                start_w: 0.0,
                current_x: 0.0,
                current_w: 0.0,
                start_color: initial_indicator_color,
                current_color: initial_indicator_color,
                elapsed: duration,
                duration,
                ease,
            },
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(padding),
                left: Val::Px(padding),
                height: Val::Px((height - 2.0 * padding).max(0.0)),
                width: Val::Px(0.0),
                border_radius: BorderRadius::MAX,
                ..default()
            },
            BackgroundColor(initial_indicator_color),
            Pickable::IGNORE,
        ));

        for (state, text_font, text_color) in per_option {
            parent.spawn((
                Button,
                ToggleOption::<S> {
                    toggle: root,
                    state,
                },
                Node {
                    padding: UiRect::axes(Val::Px(option_padding_x), Val::Px(0.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(Color::NONE),
                children![(
                    Text::new(state.label()),
                    text_font,
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

    let rel_center_x = opt_xform.translation.x - root_xform.translation.x;
    let opt_left_in_root_physical = rel_center_x + (root_node.size.x - opt_node.size.x) / 2.0;
    let scale = root_node.inverse_scale_factor;
    Some((opt_left_in_root_physical * scale, opt_node.size.x * scale))
}

fn retarget_on_state_change<S: ToggleState>(
    changed_toggles: Query<Entity, (Changed<Toggle<S>>, With<Toggle<S>>)>,
    mut indicators: Query<(&ToggleIndicator, &mut ToggleTween)>,
) {
    for toggle_entity in changed_toggles.iter() {
        for (indicator, mut tween) in indicators.iter_mut() {
            if indicator.toggle != toggle_entity {
                continue;
            }
            tween.start_x = tween.current_x;
            tween.start_w = tween.current_w;
            tween.start_color = tween.current_color;
            tween.elapsed = 0.0;
        }
    }
}

fn update_text_styling<S: ToggleState>(
    changed_toggles: Query<(Entity, &Toggle<S>, &ToggleStyle<S>), Changed<Toggle<S>>>,
    options: Query<(&ToggleOption<S>, &Children)>,
    mut texts: Query<(&mut TextColor, &mut TextFont), With<Text>>,
) {
    for (toggle_entity, toggle, style) in changed_toggles.iter() {
        for (option, children) in options.iter() {
            if option.toggle != toggle_entity {
                continue;
            }
            let resolved = style.resolve(option.state, toggle.state);
            let color = resolved.text_color.unwrap_or(style.text_color);
            let font = resolved
                .text_font
                .unwrap_or_else(|| style.text_font.clone());
            for &child in children.iter() {
                if let Ok((mut text_color, mut text_font)) = texts.get_mut(child) {
                    text_color.0 = color;
                    *text_font = font.clone();
                }
            }
        }
    }
}

fn animate_indicator<S: ToggleState>(
    time: Res<Time>,
    toggles: Query<(&Toggle<S>, &ToggleStyle<S>)>,
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
        let Ok((toggle, style)) = toggles.get(indicator.toggle) else {
            continue;
        };

        // Live-measured target so the indicator stays correct even when the
        // selected option's layout changes (e.g. font size swap on selection).
        let Some((target_x, target_w)) =
            selected_option_local_rect::<S>(indicator.toggle, toggle.state, &roots, &options)
        else {
            continue;
        };

        let target_color = style
            .resolve(toggle.state, toggle.state)
            .indicator_color
            .unwrap_or(style.indicator);

        if !indicator.initialized {
            tween.start_x = target_x;
            tween.start_w = target_w;
            tween.current_x = target_x;
            tween.current_w = target_w;
            tween.start_color = target_color;
            tween.current_color = target_color;
            tween.elapsed = tween.duration;
            indicator.initialized = true;
        }

        if tween.elapsed < tween.duration {
            tween.elapsed = (tween.elapsed + time.delta_secs()).min(tween.duration);
            let t = tween.elapsed / tween.duration;
            tween.current_x =
                EasingCurve::new(tween.start_x, target_x, tween.ease).sample_clamped(t);
            tween.current_w =
                EasingCurve::new(tween.start_w, target_w, tween.ease).sample_clamped(t);
            let eased_t = EasingCurve::new(0.0_f32, 1.0, tween.ease).sample_clamped(t);
            tween.current_color = tween.start_color.mix(&target_color, eased_t);
        } else {
            // Keep the indicator locked to the selected option once the tween
            // finishes, so later layout changes (resize, text edits) are picked up.
            tween.current_x = target_x;
            tween.current_w = target_w;
            tween.current_color = target_color;
        }

        node.left = Val::Px(tween.current_x);
        node.width = Val::Px(tween.current_w);
        bg.0 = tween.current_color;
    }
}
