use bevy::{
    ecs::{
        component::Component,
        query::With,
        resource::Resource,
        system::{Query, ResMut},
    },
    prelude::{Deref, DerefMut},
    reflect::Reflect,
};
use itertools::Itertools;
use log::info;

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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeighbouringEnergy {
    pub forward: u32,
    pub left: u32,
    pub right: u32,
    pub center: u32,
    pub total3x3: u32,
}

// impl NeighbouringEnergy {
//     pub fn new(
//         pos: &GridPosition,
//         facing: &FacingDirection,
//         energy_env: EnergyEnvironment,
//     ) -> Self {
//         let forward_pos = pos.offset(facing.0.delta());
//         let left_pos = pos.offset(facing.left().delta());
//         let right_pos = pos.offset(facing.right().delta());
//
//         let forward = energy_env.peek(forward_pos.x, forward_pos.y).unwrap_or(0);
//         let left = energy_env.peek(left_pos.x, left_pos.y).unwrap_or(0);
//         let right = energy_env.peek(right_pos.x, right_pos.y).unwrap_or(0);
//         let center = energy_env.peek(pos.x, pos.y).unwrap_or(0);
//
//         let tiles3x3 = (-1..=1).cartesian_product(-1..=1).map(|(dx, dy)| {
//             let neighbour_pos = pos.offset((dx, dy));
//             energy_env
//                 .peek(neighbour_pos.x, neighbour_pos.y)
//                 .unwrap_or(0)
//         });
//
//         NeighbouringEnergy {
//             forward,
//             left,
//             right,
//             center,
//             total3x3: tiles3x3.sum(),
//         }
//     }
// }

#[derive(Resource, Reflect, Clone, Debug)]
pub struct OrganicEnergyEnvironment(EnergyEnvironment);

impl OrganicEnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: f32) -> Self {
        OrganicEnergyEnvironment(EnergyEnvironment::new(width, height, initial_energy))
    }
}

#[derive(Resource, Reflect, Clone, Debug)]
pub struct ChargeEnergyEnvironment(EnergyEnvironment);

impl ChargeEnergyEnvironment {
    pub fn new(width: usize, height: usize, initial_energy: f32) -> Self {
        ChargeEnergyEnvironment(EnergyEnvironment::new(width, height, initial_energy))
    }
}

#[derive(Reflect, Clone, Debug)]
struct EnergyEnvironment {
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

#[derive(Component, Reflect, Clone, Copy, Debug)]
pub struct CellRequestSolarEnergy;

#[derive(Component, Reflect, Clone, Copy, Debug)]
pub struct CellRequestOrganicEnergy;

#[derive(Component, Reflect, Clone, Copy, Debug)]
pub struct CellRequestChargeEnergy;

fn process_solar_requests_system(
    mut query: Query<&mut CellEnergy, With<CellRequestSolarEnergy>>,
    mut environment: ResMut<OrganicEnergyEnvironment>,
) {
    // mn = LIGHTENERGY  // 10
    // for each of 8 neighbors:
    //     if neighbor is LEAF → return 0  // complete shading
    //     if neighbor exists (any cell) → mn -= 1
    // return OrganicMap[X][Y] * mn * LIGHTCOEF  // organic * (10 - obstructions) * 0.0008

    for mut energy in query.iter_mut() {
        todo!("Implement solar energy collection system");
    }
}
