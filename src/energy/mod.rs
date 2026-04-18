use std::sync::OnceLock;

use bevy::{
    ecs::{component::Component, resource::Resource},
    prelude::{Deref, DerefMut},
    reflect::Reflect,
};
use itertools::Itertools;

use crate::{GridPosition, cells::Direction, genes::RelativeDirection};

mod dispersal;

pub use dispersal::disperse_on_death;

pub const ORGANIC_TOXICICITY_LEVEL: u32 = 100;
pub const CHARGE_TOXICITY_LEVEL: u32 = 90;
pub const CHARGE_LIMIT: u32 = 100;

#[inline]
pub fn index(width: usize, x: usize, y: usize) -> usize {
    y * width + x
}

#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Deref, DerefMut)]
pub struct CellEnergy(pub f32);

#[derive(Reflect, Clone, Copy, Debug)]
pub enum Energy {
    Solar,
    Organic,
    Charge,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NeighbouringEnergy {
    area_energy: [f32; 9],
    facing_direction: Direction,
    toxic_threshold: f32,
    sum: OnceLock<f32>,
}

impl NeighbouringEnergy {
    pub fn new(pos: &GridPosition, facing: &Direction, energy_env: &EnergyEnvironment) -> Self {
        let area_energy = (-1..=1)
            .cartesian_product(-1..=1)
            .map(|(dx, dy)| {
                let x = pos.x as isize + dx;
                let y = pos.y as isize + dy;

                if x >= 0 && y >= 0 {
                    energy_env.peek(x as usize, y as usize).unwrap_or(0.0)
                } else {
                    0.0
                }
            })
            .collect_vec()
            .try_into()
            .expect("Exactly 9 energy values should be collected for the 3x3 area");

        NeighbouringEnergy {
            area_energy,
            facing_direction: *facing,
            sum: OnceLock::new(),
            toxic_threshold: energy_env.toxic_threshold(),
        }
    }

    pub fn area_energy(&self) -> &[f32; 9] {
        &self.area_energy
    }

    pub fn energy_at_center(&self) -> f32 {
        self.area_energy[4]
    }

    pub fn energy_in_dir(&self, relative_direction: RelativeDirection) -> f32 {
        let at = self.facing_direction.relative(relative_direction);
        let idx = match at {
            Direction::North => 1,
            Direction::East => 5,
            Direction::South => 7,
            Direction::West => 3,
        };

        self.area_energy[idx]
    }

    pub fn total_energy(&self) -> f32 {
        *self.sum.get_or_init(|| self.area_energy.iter().sum())
    }

    pub fn is_toxic_in_dir(&self, relative_direction: RelativeDirection) -> bool {
        self.energy_in_dir(relative_direction) >= self.toxic_threshold
    }
}

#[derive(Resource, Reflect, Clone, Debug, Deref, DerefMut)]
pub struct OrganicEnergyEnvironment(pub EnergyEnvironment);

impl OrganicEnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: f32, toxic_threshold: f32) -> Self {
        OrganicEnergyEnvironment(EnergyEnvironment::new(
            width,
            height,
            initial_energy,
            toxic_threshold,
        ))
    }
}

#[derive(Resource, Reflect, Clone, Debug, Deref, DerefMut)]
pub struct ChargeEnergyEnvironment(pub EnergyEnvironment);

impl ChargeEnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: f32, toxic_threshold: f32) -> Self {
        ChargeEnergyEnvironment(EnergyEnvironment::new(
            width,
            height,
            initial_energy,
            toxic_threshold,
        ))
    }
}

#[derive(Reflect, Clone, Debug)]
pub struct EnergyEnvironment {
    width: usize,
    height: usize,
    energy: Vec<f32>,
    toxic_threshold: f32,
}

impl EnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: f32, toxic_threshold: f32) -> Self {
        EnergyEnvironment {
            width,
            height,
            energy: vec![initial_energy; width * height],
            toxic_threshold,
        }
    }

    pub fn toxic_threshold(&self) -> f32 {
        self.toxic_threshold
    }
}

pub trait EnergyEnvironmentTrait {
    fn peek(&self, x: usize, y: usize) -> Option<f32>;
    fn take(&mut self, x: usize, y: usize, amount: f32) -> Option<f32>;
    fn add(&mut self, x: usize, y: usize, amount: f32);
}

impl EnergyEnvironmentTrait for EnergyEnvironment {
    fn peek(&self, x: usize, y: usize) -> Option<f32> {
        if x < self.width && y < self.height {
            let idx = index(self.width, x, y);
            Some(self.energy[idx])
        } else {
            None
        }
    }

    fn take(&mut self, x: usize, y: usize, amount: f32) -> Option<f32> {
        if x < self.width && y < self.height {
            let idx = index(self.width, x, y);
            let available = self.energy[idx];
            let taken = available.min(amount);
            self.energy[idx] -= taken;
            Some(taken)
        } else {
            None
        }
    }

    fn add(&mut self, x: usize, y: usize, amount: f32) {
        if x < self.width && y < self.height {
            let idx = index(self.width, x, y);
            self.energy[idx] += amount;
        }
    }
}
