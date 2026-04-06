use bevy::prelude::*;
use rand::{
    RngExt,
    distr::{Distribution, StandardUniform},
    seq::IndexedRandom,
};

use serde::{Deserialize, Serialize};
use strum::VariantArray;

use crate::{
    CELL_BLUE, CELL_BROWN, CELL_GREEN, CELL_ORANGE, GridPosition, TILE_SIZE,
    genes::RelativeDirection,
};

mod systems;

pub use self::systems::*;

#[derive(Component, Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct CellIsDying;

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

#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq)]
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

    /// Returns the visual specification for this cell type, including its
    /// shape, color, and any child visuals.
    pub fn visual_spec(&self) -> CellVisualSpec {
        match self {
            Cell::Leaf => CellVisualSpec {
                shape: ShapeSpec::Ellipse {
                    half_width: TILE_SIZE / 1.75,
                    half_height: TILE_SIZE / 3.0,
                },
                color: CELL_GREEN,
                children: vec![],
            },
            Cell::Antenna => CellVisualSpec {
                shape: ShapeSpec::Circle(TILE_SIZE / 3.0),
                color: CELL_BLUE,
                children: vec![],
            },
            Cell::Root => CellVisualSpec {
                shape: ShapeSpec::Rect {
                    width: TILE_SIZE / 1.5,
                    height: TILE_SIZE / 1.5,
                },
                color: CELL_ORANGE,
                children: vec![],
            },
            Cell::Sprout => CellVisualSpec {
                shape: ShapeSpec::Circle(TILE_SIZE / 3.0),
                color: Color::WHITE,
                children: vec![
                    ChildVisualSpec {
                        shape: ShapeSpec::Circle(TILE_SIZE / 15.0),
                        color: Color::BLACK,
                        transform: Transform::from_translation(Vec3::new(
                            TILE_SIZE / 6.0,
                            TILE_SIZE / 6.0,
                            2.0,
                        )),
                    },
                    ChildVisualSpec {
                        shape: ShapeSpec::Circle(TILE_SIZE / 15.0),
                        color: Color::BLACK,
                        transform: Transform::from_translation(Vec3::new(
                            TILE_SIZE / 6.0,
                            -TILE_SIZE / 6.0,
                            2.0,
                        )),
                    },
                ],
            },
            Cell::Branch => CellVisualSpec {
                shape: ShapeSpec::Rect {
                    width: TILE_SIZE * 1.5,
                    height: TILE_SIZE / 6.0,
                },
                color: CELL_BROWN,
                children: vec![],
            },
            Cell::Seed(_) => CellVisualSpec {
                shape: ShapeSpec::Circle(TILE_SIZE / 6.0),
                color: Color::WHITE,
                children: vec![],
            },
        }
    }
}

/// Bundle for rendering a cell.
#[derive(Bundle)]
pub struct CellRenderBundle {
    mesh: Mesh2d,
    material: MeshMaterial2d<ColorMaterial>,
    transform: Transform,
}

#[derive(Clone, Copy, Debug)]
enum ShapeSpec {
    Circle(f32),
    Ellipse { half_width: f32, half_height: f32 },
    Rect { width: f32, height: f32 },
}

impl ShapeSpec {
    pub fn into_mesh(self, meshes: &mut Assets<Mesh>) -> Mesh2d {
        let handle = match self {
            ShapeSpec::Circle(r) => meshes.add(Circle::new(r)),
            ShapeSpec::Ellipse {
                half_width,
                half_height,
            } => meshes.add(Ellipse::new(half_width, half_height)),
            ShapeSpec::Rect { width, height } => meshes.add(Rectangle::new(width, height)),
        };

        Mesh2d(handle)
    }
}

#[derive(Clone, Copy, Debug)]
struct ChildVisualSpec {
    shape: ShapeSpec,
    color: Color,
    transform: Transform,
}

/// Visual specification for a cell, including its shape, color, and any child
/// visuals (e.g., for details like eyes).
#[derive(Clone, Debug)]
pub struct CellVisualSpec {
    shape: ShapeSpec,
    color: Color,
    children: Vec<ChildVisualSpec>,
}
