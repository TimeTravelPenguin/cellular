use bevy::{
    ecs::{component::Component, resource::Resource},
    prelude::{Deref, DerefMut},
    reflect::Reflect,
};
use itertools::Itertools;

use crate::{GridPosition, cells::FacingDirection};

mod systems;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeighbouringEnergy {
    pub forward: f32,
    pub left: f32,
    pub right: f32,
    pub center: f32,
    pub total3x3: f32,
}

impl NeighbouringEnergy {
    pub fn new(
        pos: &GridPosition,
        facing: &FacingDirection,
        energy_env: &EnergyEnvironment,
    ) -> Self {
        let forward_pos = pos.offset(facing.0.delta());
        let left_pos = pos.offset(facing.left().delta());
        let right_pos = pos.offset(facing.right().delta());

        let forward = energy_env.peek(forward_pos.x, forward_pos.y).unwrap_or(0.0);
        let left = energy_env.peek(left_pos.x, left_pos.y).unwrap_or(0.0);
        let right = energy_env.peek(right_pos.x, right_pos.y).unwrap_or(0.0);
        let center = energy_env.peek(pos.x, pos.y).unwrap_or(0.0);

        let tiles3x3 = (-1..=1).cartesian_product(-1..=1).map(|(dx, dy)| {
            let neighbour_pos = pos.offset((dx, dy));
            energy_env
                .peek(neighbour_pos.x, neighbour_pos.y)
                .unwrap_or(0.0)
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

#[derive(Resource, Reflect, Clone, Debug, Deref, DerefMut)]
pub struct OrganicEnergyEnvironment(pub EnergyEnvironment);

impl OrganicEnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: f32) -> Self {
        OrganicEnergyEnvironment(EnergyEnvironment::new(width, height, initial_energy))
    }
}

#[derive(Resource, Reflect, Clone, Debug, Deref, DerefMut)]
pub struct ChargeEnergyEnvironment(pub EnergyEnvironment);

impl ChargeEnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: f32) -> Self {
        ChargeEnergyEnvironment(EnergyEnvironment::new(width, height, initial_energy))
    }
}

#[derive(Reflect, Clone, Debug)]
pub struct EnergyEnvironment {
    width: usize,
    height: usize,
    energy: Vec<f32>,
}

impl EnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: f32) -> Self {
        EnergyEnvironment {
            width,
            height,
            energy: vec![initial_energy; width * height],
        }
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
