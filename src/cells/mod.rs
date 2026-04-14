use bevy::{
    ecs::{event::EntityEvent, query::QueryData},
    prelude::{
        Assets, Bundle, Circle, Color, ColorMaterial, Component, Deref, DerefMut, Ellipse, Entity,
        Mesh, Mesh2d, MeshMaterial2d, Message, Rectangle, Reflect, Transform, Vec, Vec3, vec,
    },
};
use rand::{
    RngExt,
    distr::{Distribution, StandardUniform},
    seq::IndexedRandom,
};

use serde::{Deserialize, Serialize};
use strum::VariantArray;

use crate::{
    CELL_BLUE, CELL_BROWN, CELL_GREEN, CELL_ORANGE, GridPosition, TILE_SIZE,
    energy::CellEnergy,
    genes::{Genome, GenomeID, RelativeDirection},
};

mod systems;

pub use self::systems::*;

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

#[derive(EntityEvent, Debug, Clone)]
pub struct NewCellEvent {
    pub entity: Entity,
    pub grid_pos: GridPosition,
    pub cell: Cell,
    pub facing_direction: FacingDirection,
}

#[derive(Component, Reflect, Clone, Copy, Debug, Serialize, Deserialize)]
pub struct RequestDeath;

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

#[derive(Component, Reflect, Clone, Copy, Debug, Deref, DerefMut, Serialize, Deserialize)]
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
        matches!(self, Cell::Seed | Cell::Leaf | Cell::Sprout)
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
            Cell::Seed => CellVisualSpec {
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

#[derive(Message, Clone, Debug)]
pub struct SpawnChildCellMessage {
    pub parent: Entity,
    pub child_cell: Cell,
    pub child_genome: Genome,
}

#[derive(Component, Reflect, Clone, Debug)]
pub struct CellRelation {
    pub parent: Option<Entity>,
    pub children: Vec<Entity>,
}

#[derive(Message)]
pub struct UpdateCellInfoMessage {
    pub cell: Option<Entity>,
}

#[derive(QueryData)]
pub struct CellInfo {
    pub position: &'static GridPosition,
    pub cell_type: &'static Cell,
    pub energy: &'static CellEnergy,
    pub facing: &'static FacingDirection,
    pub genome_id: &'static GenomeID,
}
