use bevy::{platform::collections::HashMap, prelude::*};
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use rand::{
    RngExt,
    distr::{Distribution, StandardUniform},
    seq::IndexedRandom,
};

use serde::{Deserialize, Serialize};
use strum::VariantArray;

use crate::{
    GridPosition,
    energy::{Energy, SimulationEnvironment},
    genes::{CellGenomeCommand, Genome, GenomeID, RelativeDirection},
};

#[derive(Reflect, VariantArray, Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Direction {
    North,
    East,
    South,
    West,
}

impl Direction {
    pub fn delta(&self) -> (isize, isize) {
        match self {
            Direction::North => (0, 1),
            Direction::East => (1, 0),
            Direction::South => (0, -1),
            Direction::West => (-1, 0),
        }
    }
}

impl Distribution<Direction> for StandardUniform {
    fn sample<R: RngExt + ?Sized>(&self, rng: &mut R) -> Direction {
        *Direction::VARIANTS.choose(rng).unwrap()
    }
}

#[derive(Component, Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct FacingDirection(pub Direction);

impl Distribution<FacingDirection> for StandardUniform {
    fn sample<R: RngExt + ?Sized>(&self, rng: &mut R) -> FacingDirection {
        FacingDirection(rng.random())
    }
}

impl FacingDirection {
    pub fn relative(&self, relative_direction: RelativeDirection) -> Direction {
        match (self.0, relative_direction) {
            (dir, RelativeDirection::Forward) => dir,
            (Direction::North, RelativeDirection::Left) => Direction::West,
            (Direction::North, RelativeDirection::Right) => Direction::East,
            (Direction::East, RelativeDirection::Left) => Direction::North,
            (Direction::East, RelativeDirection::Right) => Direction::South,
            (Direction::South, RelativeDirection::Left) => Direction::East,
            (Direction::South, RelativeDirection::Right) => Direction::West,
            (Direction::West, RelativeDirection::Left) => Direction::South,
            (Direction::West, RelativeDirection::Right) => Direction::North,
        }
    }

    pub fn opposite(&self) -> Direction {
        match self.0 {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }
}

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

#[derive(Reflect, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeedCell {
    DormantSeed,
    DetachedSeed { is_stationary: bool },
}

impl SeedCell {
    pub const fn is_detached(&self) -> bool {
        matches!(self, SeedCell::DetachedSeed { .. })
    }

    pub const fn is_stationary(&self) -> bool {
        matches!(
            self,
            SeedCell::DormantSeed
                | SeedCell::DetachedSeed {
                    is_stationary: true
                }
        )
    }
}

#[derive(Component, Reflect, Clone, Copy, Debug)]
pub struct CellEnergy(pub u32);

#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cell {
    Leaf,
    Antenna,
    Root,
    Sprout,
    Branch,
    Seed(SeedCell),
}

impl Cell {
    pub fn is_consumable(&self) -> bool {
        matches!(self, Cell::Seed(_) | Cell::Leaf | Cell::Sprout)
    }
}

// fn neighbouring_organic(
//     environment: &ResMut<'_, SimulationEnvironment>,
//     grid_pos: &GridPosition,
//     neighbour_positions: &CellNeighbourPositions,
//     cell_pos3x3: &[(usize, usize)],
// ) -> NeighbouringEnergy {
//     NeighbouringEnergy {
//         forward: environment
//             .organic(neighbour_positions.forward.0, neighbour_positions.forward.1)
//             .unwrap_or(0),
//         left: environment
//             .organic(neighbour_positions.left.0, neighbour_positions.left.1)
//             .unwrap_or(0),
//         right: environment
//             .organic(neighbour_positions.right.0, neighbour_positions.right.1)
//             .unwrap_or(0),
//         center: environment.organic(grid_pos.x, grid_pos.y).unwrap_or(0),
//         total3x3: cell_pos3x3
//             .iter()
//             .map(|&pos| environment.organic(pos.0, pos.1).unwrap_or(0))
//             .sum(),
//     }
// }
//
// fn neighbouring_charge(
//     environment: &SimulationEnvironment,
//     grid_pos: &GridPosition,
//     neighbour_positions: &CellNeighbourPositions,
//     cell_pos3x3: &[(usize, usize)],
// ) -> NeighbouringEnergy {
//     NeighbouringEnergy {
//         forward: environment
//             .charge(neighbour_positions.forward.0, neighbour_positions.forward.1)
//             .unwrap_or(0),
//         left: environment
//             .charge(neighbour_positions.left.0, neighbour_positions.left.1)
//             .unwrap_or(0),
//         right: environment
//             .charge(neighbour_positions.right.0, neighbour_positions.right.1)
//             .unwrap_or(0),
//         center: environment.charge(grid_pos.x, grid_pos.y).unwrap_or(0),
//         total3x3: cell_pos3x3
//             .iter()
//             .map(|&pos| environment.charge(pos.0, pos.1).unwrap_or(0))
//             .sum(),
//     }
// }

pub fn invoke_cell_actions_system(
    _commands: Commands,
    _rng: Single<&mut WyRand, With<GlobalRng>>,
    mut cells: Query<(
        &GridPosition,
        &Cell,
        &mut CellEnergy,
        AnyOf<(&Genome, &mut GenomeID)>,
    )>,
    mut environment: ResMut<SimulationEnvironment>,
    time: Res<Time>,
) {
    for (grid_pos, cell, mut cell_energy, (genome, mut genome_id)) in cells.iter_mut() {
        match *cell {
            Cell::Leaf => {
                let energy = environment.sunlight(time.elapsed().as_secs());
                cell_energy.0 += energy;
            }
            Cell::Root => {
                let energy = environment
                    .collect_organic(grid_pos.x, grid_pos.y)
                    .unwrap_or(0);

                cell_energy.0 += energy;
            }

            Cell::Antenna => {
                let energy = environment
                    .collect_charge(grid_pos.x, grid_pos.y)
                    .unwrap_or(0);

                cell_energy.0 += energy;
            }
            Cell::Branch => {
                // TODO: Remove [transfer_energy_system] and add here to optimize
            }
            Cell::Sprout => {}           // TODO
            Cell::Seed(_seed_cell) => {} // TODO
        };
    }
}

pub fn transfer_energy_system(world: &mut World) {
    let mut transfers: HashMap<Entity, u32> = HashMap::new();

    for (transfer, mut cell_energy) in world
        .query::<(&EnergyTransferer, &mut CellEnergy)>()
        .iter_mut(world)
    {
        let recipients = transfer.transfer_recipients();
        let recipient_count = recipients.len() as u32;

        if cell_energy.0 < recipient_count || recipient_count == 0 {
            continue;
        }

        let amount = cell_energy.0 / recipient_count;
        cell_energy.0 -= amount * recipient_count;

        for recipient in recipients {
            *transfers.entry(recipient).or_insert(0) += amount;
        }
    }

    for (entity, energy) in transfers {
        if energy == 0 {
            continue;
        }

        let Ok(mut cell_entity) = world.get_entity_mut(entity) else {
            info!(
                "Entity {} was removed before energy transfer could be applied",
                entity
            );
            continue;
        };

        let mut cell_energy = cell_entity
            .get_mut::<CellEnergy>()
            .expect("Entity should have CellEnergy component");

        cell_energy.0 += energy;
    }
}
