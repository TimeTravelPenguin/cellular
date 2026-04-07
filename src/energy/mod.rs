use bevy::{
    color::Color,
    ecs::{component::Component, entity::Entity, resource::Resource},
    prelude::{Deref, DerefMut},
    reflect::Reflect,
};
use itertools::Itertools;
use log::info;
use serde::{Deserialize, Serialize};

mod systems;

use crate::{GridPosition, cells::FacingDirection};

pub use self::systems::*;

pub const ORGANIC_TOXICICITY_LEVEL: u32 = 100;
pub const CHARGE_TOXICITY_LEVEL: u32 = 90;
pub const CHARGE_LIMIT: u32 = 100;

#[inline]
pub fn index(width: usize, x: usize, y: usize) -> usize {
    y * width + x
}

#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellEnergy(pub u32);

#[derive(Reflect, Clone, Copy, Debug)]
pub enum Energy {
    Solar,
    Organic,
    Charge,
}

#[derive(Component, Reflect, Clone, Copy, Debug)]
pub struct RenderedEnergyTile {
    pub solar: f64,
    pub organic: f64,
    pub charge: f64,
}

impl RenderedEnergyTile {
    pub const MAX_SOLAR_ILLUMINOCITY: f64 = SunlightCycle::MAX_SUNLIGHT;
    pub const MAX_ORGANIC_ILLUMINOCITY: f64 = ORGANIC_TOXICICITY_LEVEL as f64;
    pub const MAX_CHARGE_ILLUMINOCITY: f64 = CHARGE_LIMIT as f64;

    pub fn set_solar(&mut self, solar: f64) {
        self.solar = solar;
    }

    pub fn set_organic(&mut self, organic: f64) {
        self.organic = organic;
    }

    pub fn set_charge(&mut self, charge: f64) {
        self.charge = charge;
    }

    pub fn solar_color(&self) -> Color {
        let intensity = (self.solar as f64 / Self::MAX_SOLAR_ILLUMINOCITY).min(1.0);
        Color::hsv(60.0, 1.0, intensity as f32)
    }

    pub fn organic_color(&self) -> Color {
        let intensity = (self.organic as f64 / Self::MAX_ORGANIC_ILLUMINOCITY).min(1.0);
        Color::hsv(0.0, 1.0, intensity as f32)
    }

    pub fn charge_color(&self) -> Color {
        let intensity = (self.charge as f64 / Self::MAX_CHARGE_ILLUMINOCITY).min(1.0);
        Color::hsv(240.0, 1.0, intensity as f32)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeighbouringEnergy {
    pub forward: u32,
    pub left: u32,
    pub right: u32,
    pub center: u32,
    pub total3x3: u32,
}

impl NeighbouringEnergy {
    pub fn new(
        pos: &GridPosition,
        facing: &FacingDirection,
        energy_env: &impl EnergyEnvironmentTrait,
    ) -> Self {
        let forward_pos = pos.offset(facing.0.delta());
        let left_pos = pos.offset(facing.left().delta());
        let right_pos = pos.offset(facing.right().delta());

        let forward = energy_env.peek(forward_pos.x, forward_pos.y).unwrap_or(0);
        let left = energy_env.peek(left_pos.x, left_pos.y).unwrap_or(0);
        let right = energy_env.peek(right_pos.x, right_pos.y).unwrap_or(0);
        let center = energy_env.peek(pos.x, pos.y).unwrap_or(0);

        let tiles3x3 = (-1..=1).cartesian_product(-1..=1).map(|(dx, dy)| {
            let neighbour_pos = pos.offset((dx, dy));
            energy_env
                .peek(neighbour_pos.x, neighbour_pos.y)
                .unwrap_or(0)
        });

        NeighbouringEnergy {
            forward,
            left,
            right,
            center,
            total3x3: tiles3x3.sum(),
        }
    }
}

#[derive(Resource, Reflect, Clone, Debug)]
pub struct SunlightCycle {
    period: f64,
    min_sunlight: f64,
    max_sunlight: f64,
    offset: f64,
    initial_sunlight: f64,
    initial_slope: f64,
    body_coefficient: f64,
}

impl Default for SunlightCycle {
    fn default() -> Self {
        SunlightCycle::new(
            Self::SUNLIGHT_PERIOD,
            Self::MIN_SUNLIGHT,
            Self::MAX_SUNLIGHT,
            Self::SUNLIGHT_OFFSET,
            Self::INITIAL_SUNLIGHT,
        )
    }
}

impl SunlightCycle {
    pub const INITIAL_SUNLIGHT: f64 = 20.0;
    pub const MIN_SUNLIGHT: f64 = 1.0;
    pub const MAX_SUNLIGHT: f64 = 10.0;
    pub const SUNLIGHT_OFFSET: f64 = 50.0;
    pub const SUNLIGHT_PERIOD: f64 = 30.0;

    pub fn new(
        period: f64,
        min_sunlight: f64,
        max_sunlight: f64,
        offset: f64,
        initial_sunlight: f64,
    ) -> Self {
        let initial_slope = (initial_sunlight - min_sunlight) / offset;
        let body_coefficient = 2.0 * (max_sunlight - min_sunlight);

        SunlightCycle {
            period,
            min_sunlight,
            max_sunlight,
            offset,
            initial_sunlight,
            initial_slope,
            body_coefficient,
        }
    }

    fn initial_curve(&self, t: f64) -> f64 {
        self.initial_sunlight - self.initial_slope * t
    }

    fn curve_body(&self, t: f64) -> f64 {
        let x = (t - self.offset) / self.period;
        self.min_sunlight + self.body_coefficient * (x - (x - 0.5).floor()).abs()
    }

    pub fn sunlight(&self, t: f64) -> f64 {
        if t < self.offset {
            self.initial_curve(t)
        } else {
            self.curve_body(t)
        }
    }
}

#[derive(Resource, Reflect, Clone, Debug, Deref, DerefMut)]
pub struct OrganicEnergyEnvironment(EnergyEnvironment);

impl OrganicEnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: u32) -> Self {
        OrganicEnergyEnvironment(EnergyEnvironment::new(width, height, initial_energy))
    }
}

#[derive(Resource, Reflect, Clone, Debug, Deref, DerefMut)]
pub struct ChargeEnergyEnvironment(EnergyEnvironment);

#[inline]
fn euler_ode_approx(
    f: impl Fn(f64, f64) -> f64,
    x0: f64,
    y0: f64,
    dt: f64,
    steps: usize,
) -> (f64, f64) {
    let mut x = x0;
    let mut y = y0;

    for n in 0..steps {
        y += f(x, y) * dt;
        x = x0 + n as f64 * dt;
    }

    (x, y)
}

#[inline]
fn logistic_growth(_t: f64, y: f64) -> f64 {
    const R: f64 = 0.5;
    R * y * (1.0 - y / CHARGE_LIMIT as f64)
}

impl ChargeEnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: u32) -> Self {
        ChargeEnergyEnvironment(EnergyEnvironment::new(width, height, initial_energy))
    }

    pub fn charge(&mut self) -> &mut Self {
        // Charge follows y' = r * y * (1 - y / CHARGE_LIMIT)

        info!("Charging environment");
        for e in self.0.energy.iter_mut() {
            let (_, new_energy) =
                euler_ode_approx(logistic_growth, 0.0, (*e as f64).max(1.0), 0.5, 1);
            *e = new_energy.round() as u32;
        }

        self
    }
}

#[derive(Reflect, Clone, Debug)]
pub struct EnergyEnvironment {
    width: usize,
    height: usize,
    energy: Vec<u32>,
}

impl EnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: u32) -> Self {
        EnergyEnvironment {
            width,
            height,
            energy: vec![initial_energy; width * height],
        }
    }

    pub fn collect(&mut self, x: usize, y: usize) -> Option<u32> {
        let idx = index(self.width, x, y);
        let amount = self.energy.get_mut(idx)?;
        let taken = *amount;
        *amount = 0;

        Some(taken)
    }

    pub fn collect_split(&mut self, x: usize, y: usize, split: usize) -> Option<u32> {
        let idx = index(self.width, x, y);
        let amount = self.energy.get_mut(idx)?;
        let collected = *amount / split as u32;
        *amount -= collected * split as u32;

        Some(collected)
    }

    pub fn peek(&self, x: usize, y: usize) -> Option<u32> {
        let idx = index(self.width, x, y);
        self.energy.get(idx).copied()
    }
}

pub trait EnergyEnvironmentTrait {
    /// Collects all energy from the specified cell, returning the amount collected.
    fn collect(&mut self, x: usize, y: usize) -> Option<u32>;

    /// Collects energy from the specified cell, splitting it evenly among `split` collectors.
    fn collect_split(&mut self, x: usize, y: usize, split: usize) -> Option<u32>;

    /// Peeks at the amount of energy available in the specified cell without modifying it.
    fn peek(&self, x: usize, y: usize) -> Option<u32>;
}

impl EnergyEnvironmentTrait for EnergyEnvironment {
    fn collect(&mut self, x: usize, y: usize) -> Option<u32> {
        self.collect(x, y)
    }

    fn collect_split(&mut self, x: usize, y: usize, split: usize) -> Option<u32> {
        self.collect_split(x, y, split)
    }

    fn peek(&self, x: usize, y: usize) -> Option<u32> {
        self.peek(x, y)
    }
}

impl EnergyEnvironmentTrait for OrganicEnergyEnvironment {
    fn collect(&mut self, x: usize, y: usize) -> Option<u32> {
        self.0.collect(x, y)
    }

    fn collect_split(&mut self, x: usize, y: usize, split: usize) -> Option<u32> {
        self.0.collect_split(x, y, split)
    }

    fn peek(&self, x: usize, y: usize) -> Option<u32> {
        self.0.peek(x, y)
    }
}

impl EnergyEnvironmentTrait for ChargeEnergyEnvironment {
    fn collect(&mut self, x: usize, y: usize) -> Option<u32> {
        self.0.collect(x, y)
    }

    fn collect_split(&mut self, x: usize, y: usize, split: usize) -> Option<u32> {
        self.0.collect_split(x, y, split)
    }

    fn peek(&self, x: usize, y: usize) -> Option<u32> {
        self.0.peek(x, y)
    }
}

#[derive(Component, Reflect, Clone, Copy, Debug)]
pub struct CellRequestSolarEnergy;

#[derive(Component, Reflect, Clone, Copy, Debug)]
pub struct CellRequestOrganicEnergy(GridPosition);

#[derive(Component, Reflect, Clone, Copy, Debug)]
pub struct CellRequestChargeEnergy(GridPosition);

#[derive(Component, Reflect, Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct EnergyTransferer {
    pub north: Option<Entity>,
    pub east: Option<Entity>,
    pub south: Option<Entity>,
    pub west: Option<Entity>,
}

impl EnergyTransferer {
    pub fn transfer_recipients(&self) -> Vec<Entity> {
        [self.north, self.east, self.south, self.west]
            .iter()
            .filter_map(|&opt| opt)
            .collect()
    }
}
