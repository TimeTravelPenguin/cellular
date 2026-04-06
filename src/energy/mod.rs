use bevy::{
    ecs::resource::Resource,
    prelude::{Deref, DerefMut},
    reflect::Reflect,
};
use log::info;

mod systems;

pub use self::systems::*;

pub const ORGANIC_TOXICICITY_LEVEL: u32 = 100;
pub const CHARGE_TOXICITY_LEVEL: u32 = 90;
pub const CHARGE_LIMIT: u32 = 100;

#[inline]
pub fn index(width: usize, x: usize, y: usize) -> usize {
    y * width + x
}

#[derive(Reflect, Clone, Copy, Debug)]
pub enum Energy {
    Solar,
    Organic,
    Charge,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeighbouringEnergy {
    pub forward: u32,
    pub left: u32,
    pub right: u32,
    pub center: u32,
    pub total3x3: u32,
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
    const INITIAL_SUNLIGHT: f64 = 20.0;
    const MIN_SUNLIGHT: f64 = 1.0;
    const MAX_SUNLIGHT: f64 = 10.0;
    const SUNLIGHT_OFFSET: f64 = 50.0;
    const SUNLIGHT_PERIOD: f64 = 30.0;

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
    fn collect(&mut self, x: usize, y: usize) -> Option<u32>;
    fn collect_split(&mut self, x: usize, y: usize, split: usize) -> Option<u32>;
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
