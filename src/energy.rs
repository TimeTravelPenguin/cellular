use bevy::{ecs::resource::Resource, reflect::Reflect};

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

#[derive(Resource, Reflect, Clone, Debug)]
pub struct OrganicEnergyEnvironment(EnergyEnvironment);

impl OrganicEnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: u32) -> Self {
        OrganicEnergyEnvironment(EnergyEnvironment::new(width, height, initial_energy))
    }
}

#[derive(Resource, Reflect, Clone, Debug)]
pub struct ChargeEnergyEnvironment(EnergyEnvironment);

impl ChargeEnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: u32) -> Self {
        ChargeEnergyEnvironment(EnergyEnvironment::new(width, height, initial_energy))
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
}

pub trait EnergyEnvironmentTrait {
    fn collect(&mut self, x: usize, y: usize) -> Option<u32>;
    fn collect_split(&mut self, x: usize, y: usize, split: usize) -> Option<u32>;
}

impl EnergyEnvironmentTrait for EnergyEnvironment {
    fn collect(&mut self, x: usize, y: usize) -> Option<u32> {
        self.collect(x, y)
    }

    fn collect_split(&mut self, x: usize, y: usize, split: usize) -> Option<u32> {
        self.collect_split(x, y, split)
    }
}

impl EnergyEnvironmentTrait for OrganicEnergyEnvironment {
    fn collect(&mut self, x: usize, y: usize) -> Option<u32> {
        self.0.collect(x, y)
    }

    fn collect_split(&mut self, x: usize, y: usize, split: usize) -> Option<u32> {
        self.0.collect_split(x, y, split)
    }
}

impl EnergyEnvironmentTrait for ChargeEnergyEnvironment {
    fn collect(&mut self, x: usize, y: usize) -> Option<u32> {
        self.0.collect(x, y)
    }

    fn collect_split(&mut self, x: usize, y: usize, split: usize) -> Option<u32> {
        self.0.collect_split(x, y, split)
    }
}
