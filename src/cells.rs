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
    CELL_BLUE, CELL_GREEN, CELL_ORANGE, GridPosition, TILE_SIZE,
    energy::SimulationEnvironment,
    genes::{Genome, GenomeID, RelativeDirection},
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
                color: Color::linear_rgb(30.0 / 255.0, 20.0 / 255.0, 10.0 / 255.0),
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

fn facing_rotation(direction: Direction) -> Quat {
    match direction {
        Direction::East => Quat::IDENTITY,
        Direction::South => Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
        Direction::West => Quat::from_rotation_z(std::f32::consts::PI),
        Direction::North => Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
    }
}

#[inline]
const fn grid_pos_to_world_pos(grid_pos: &GridPosition) -> Vec3 {
    let world_x = grid_pos.x as f32 * TILE_SIZE;
    let world_y = grid_pos.y as f32 * TILE_SIZE;

    Vec3::new(world_x, world_y, 1.0)
}

/// Computes the world transform for a cell based on its grid position and facing direction.
pub fn cell_transform(grid_pos: &GridPosition, facing: Direction) -> Transform {
    let translation = grid_pos_to_world_pos(grid_pos);

    Transform {
        translation,
        rotation: facing_rotation(facing),
        ..default()
    }
}

/// Inserts the necessary components to render a cell based on its visual specification.
pub fn insert_cell_visual(
    entity_commands: &mut EntityCommands,
    spec: CellVisualSpec,
    transform: Transform,
    grid_pos: GridPosition,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    let mesh = spec.shape.into_mesh(meshes);
    let material = MeshMaterial2d(materials.add(ColorMaterial::from_color(spec.color)));

    entity_commands.insert((
        CellRenderBundle {
            mesh,
            material,
            transform,
        },
        grid_pos,
    ));

    entity_commands.with_children(|parent| {
        for child in spec.children {
            parent.spawn(CellRenderBundle {
                mesh: child.shape.into_mesh(meshes),
                material: MeshMaterial2d(materials.add(ColorMaterial::from_color(child.color))),
                transform: child.transform,
            });
        }
    });
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
    for (grid_pos, cell, mut cell_energy, (_genome, _genome_id)) in cells.iter_mut() {
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
