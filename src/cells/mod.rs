use std::fmt::Debug;

use bevy::{
    ecs::query::QueryData,
    platform::collections::{HashMap, HashSet},
    prelude::{Component, Deref, DerefMut, Entity, EntityEvent, Message, Reflect},
};
use rand::{
    RngExt,
    distr::{Distribution, StandardUniform},
    seq::IndexedRandom,
};

use serde::{Deserialize, Serialize};
use strum::VariantArray;

use crate::{
    GridPosition,
    energy::CellEnergy,
    genes::{Genome, GenomeID, RelativeDirection},
};

mod render;
mod spawn;
mod systems;

pub use self::systems::*;
pub use spawn::spawn_cell;

#[derive(Component, Reflect, Clone, Copy, Debug, Deref, DerefMut)]
pub struct RemainingTicksWithoutEnergy(pub u32);

#[derive(Component, Reflect, Default, Clone, Copy, Debug)]
pub struct ProducerCell;

#[derive(Component, Reflect, Default, Clone, Copy, Debug)]
pub struct EnergyTransferCell;

#[derive(Component, Reflect, Clone, Copy, Debug)]
#[require(Cell::Sprout)]
pub struct SproutCell;

#[derive(Component, Reflect, Clone, Copy, Debug)]
#[require(Cell::Leaf, ProducerCell, EnergyTransferCell)]
pub struct LeafCell;

#[derive(Component, Reflect, Clone, Copy, Debug)]
#[require(Cell::Antenna, ProducerCell, EnergyTransferCell)]
pub struct AntennaCell;

#[derive(Component, Reflect, Clone, Copy, Debug)]
#[require(Cell::Root, ProducerCell, EnergyTransferCell)]
pub struct RootCell;

#[derive(Component, Reflect, Clone, Copy, Debug)]
#[require(Cell::Branch, EnergyTransferCell)]
pub struct BranchCell;

#[derive(Component, Reflect, Clone, Copy, Debug)]
#[require(Cell::Seed)]
pub struct SeedCell;

#[derive(Component, Reflect, Clone, Copy, Debug, Deref, DerefMut)]
pub struct OrganismDepth(pub usize);

#[derive(Component, Reflect, Clone, Copy, Debug, Deref, DerefMut)]
pub struct PreviousEnergy(pub f32);

#[derive(Component, Reflect, Clone, Copy, Debug)]
pub struct EnergyTransferer;

#[derive(EntityEvent, Debug, Clone)]
pub struct NewCellEvent {
    pub entity: Entity,
    pub grid_pos: GridPosition,
    pub cell: Cell,
    pub facing_direction: FacingDirection,
}

#[derive(Message, Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RequestDeathMessage {
    pub entity: Entity,
}

#[derive(Reflect, VariantArray, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(
    Component, Reflect, Clone, Copy, Debug, Eq, PartialEq, Deref, DerefMut, Serialize, Deserialize,
)]
pub struct FacingDirection(pub Direction);

impl Distribution<FacingDirection> for StandardUniform {
    fn sample<R: RngExt + ?Sized>(&self, rng: &mut R) -> FacingDirection {
        FacingDirection(rng.random())
    }
}

impl Direction {
    pub fn relative(&self, relative_direction: RelativeDirection) -> Direction {
        match (*self, relative_direction) {
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
        match self {
            Direction::North => Direction::South,
            Direction::East => Direction::West,
            Direction::South => Direction::North,
            Direction::West => Direction::East,
        }
    }

    pub fn left(&self) -> Direction {
        self.relative(RelativeDirection::Left)
    }

    pub fn right(&self) -> Direction {
        self.relative(RelativeDirection::Right)
    }

    pub fn forward(&self) -> Direction {
        self.relative(RelativeDirection::Forward)
    }
}

#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cell {
    Leaf,
    Antenna,
    Root,
    Sprout,
    Branch,
    Seed,
}

impl Cell {
    pub fn is_consumable(&self) -> bool {
        matches!(self, Cell::Leaf | Cell::Antenna | Cell::Root)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NeighbouringCells {
    pub cells: [Option<Cell>; 9],
    facing_direction: Direction,
}

impl NeighbouringCells {
    pub fn new(
        center: GridPosition,
        facing_direction: Direction,
        cells: &HashMap<GridPosition, Cell>,
    ) -> Self {
        let mut cell_array = [None; 9];

        for (i, dy) in (-1..=1).enumerate() {
            for (j, dx) in (-1..=1).enumerate() {
                let pos = GridPosition {
                    x: center.x.saturating_add_signed(dx),
                    y: center.y.saturating_add_signed(dy),
                };

                cell_array[i * 3 + j] = cells.get(&pos).cloned();
            }
        }

        NeighbouringCells {
            cells: cell_array,
            facing_direction,
        }
    }

    pub fn cell_in_dir(&self, relative_direction: RelativeDirection) -> Option<Cell> {
        let at = self.facing_direction.relative(relative_direction);
        let idx = match at {
            Direction::North => 1,
            Direction::East => 5,
            Direction::South => 7,
            Direction::West => 3,
        };

        self.cells[idx]
    }
}

#[derive(Message, Clone, Debug)]
pub struct SpawnChildCellMessage {
    pub parent: Entity,
    pub child_cell: Cell,
    pub child_genome: Genome,
}

#[derive(Component, Reflect, Clone, Debug)]
pub struct CellRelation {
    pub parent: Option<Entity>,
    pub children: HashSet<Entity>,
}

#[derive(Message)]
pub struct UpdateCellInfoMessage {
    pub cell: Option<Entity>,
}

#[derive(QueryData)]
pub struct CellInfo {
    pub cell: &'static Cell,
    pub position: &'static GridPosition,
    pub energy: &'static CellEnergy,
    pub facing: &'static FacingDirection,
    pub genome: &'static Genome,
    pub genome_id: &'static GenomeID,
}

#[derive(Message, Clone, Debug)]
pub struct CellEnergyTransferMessage {
    pub from: Entity,
    pub to: Entity,
    pub amount: f32,
}

#[derive(Message, Clone, Debug)]
pub struct RemoveChildCellMessage {
    pub parent: Entity,
    pub child: Entity,
}
