use bevy::{ecs::resource::Resource, reflect::Reflect};
use bevy_egui::egui::emath::Numeric;

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

#[derive(Clone, Debug)]
struct SunlightCycle {
    period: f64,
    min_sunlight: f64,
    max_sunlight: f64,
    offset: f64,
    initial_sunlight: f64,
    initial_slope: f64,
    body_coefficient: f64,
}

impl SunlightCycle {
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

#[derive(Resource, Clone, Debug)]
pub struct SimulationEnvironment {
    width: usize,
    height: usize,
    sunlight_cycle: SunlightCycle,
    organic_matter: Vec<u32>,
    charge: Vec<u32>,
}

impl SimulationEnvironment {
    const INITIAL_SUNLIGHT: f64 = 20.0;
    const MIN_SUNLIGHT: f64 = 1.0;
    const MAX_SUNLIGHT: f64 = 10.0;
    const SUNLIGHT_OFFSET: f64 = 50.0;
    const SUNLIGHT_PERIOD: f64 = 30.0;

    pub fn new(
        width: usize,
        height: usize,
        initial_organic_matter: u32,
        initial_charge: u32,
    ) -> Self {
        SimulationEnvironment {
            width,
            height,
            sunlight_cycle: SunlightCycle::new(
                Self::SUNLIGHT_PERIOD,
                Self::MIN_SUNLIGHT,
                Self::MAX_SUNLIGHT,
                Self::SUNLIGHT_OFFSET,
                Self::INITIAL_SUNLIGHT,
            ),
            organic_matter: vec![initial_organic_matter; width * height],
            charge: vec![initial_charge; width * height],
        }
    }

    pub fn collect_organic(&mut self, x: usize, y: usize) -> Option<u32> {
        let amount = self.organic_matter.get_mut(index(self.width, x, y))?;

        if *amount > 0 {
            *amount = amount.saturating_sub(1);
        }

        Some(*amount)
    }

    pub fn collect_charge(&mut self, x: usize, y: usize) -> Option<u32> {
        let amount = self.charge.get_mut(index(self.width, x, y))?;

        if *amount > 0 {
            *amount = amount.saturating_sub(1);
        }

        Some(*amount)
    }

    pub fn sunlight(&self, t: u64) -> u32 {
        self.sunlight_cycle.sunlight(t.to_f64()).round() as u32
    }
}
