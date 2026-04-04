use bevy::{platform::collections::HashMap, prelude::*};
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use itertools::Itertools;
use rand::{
    RngExt,
    distr::{Distribution, StandardUniform},
    seq::IndexedRandom,
};

use serde::{Deserialize, Serialize};
use strum::VariantArray;

use crate::{
    CELL_BLUE, CELL_BROWN, CELL_GREEN, CELL_ORANGE, GridPosition, SimulationStep, TILE_SIZE,
    energy::{
        ChargeEnergyEnvironment, EnergyEnvironmentTrait, OrganicEnergyEnvironment, SunlightCycle,
    },
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

pub fn cell_request_energy_system(
    mut commands: Commands,
    cells: Query<(Entity, &GridPosition, &Cell)>,
) {
    for (entity, grid_pos, cell) in cells.iter() {
        match cell {
            Cell::Leaf => commands.entity(entity).insert(CellRequestSolarEnergy),
            Cell::Antenna => commands
                .entity(entity)
                .insert(CellRequestChargeEnergy(*grid_pos)),
            Cell::Root => commands
                .entity(entity)
                .insert(CellRequestOrganicEnergy(*grid_pos)),
            _ => unimplemented!("Energy requests for cell type {:?} not implemented", cell),
        };
    }
}

pub fn cell_collect_solar_energy(
    mut query: Query<&mut CellEnergy, With<CellRequestSolarEnergy>>,
    environment: Res<SunlightCycle>,
    simulation_step: Res<SimulationStep>,
) {
    let sunlight = environment.sunlight(simulation_step.0 as f64);
    for mut cell_energy in query.iter_mut() {
        cell_energy.0 += sunlight as u32;
    }
}

pub fn distribute_energy<'a, T: Resource + EnergyEnvironmentTrait>(
    environment: &mut ResMut<T>,
    energies: &mut [&mut Mut<'a, CellEnergy>],
    grid_positions: &GridPosition,
) {
    let energy_per_cell = environment
        .collect_split(grid_positions.x, grid_positions.y, energies.len())
        .unwrap_or(0);

    if energy_per_cell == 0 {
        return;
    }

    for energy in energies {
        energy.0 += energy_per_cell;
    }
}

pub fn cell_collect_organic_energy_system(
    mut query: Query<(&mut CellEnergy, &GridPosition), With<CellRequestOrganicEnergy>>,
    mut environment: ResMut<OrganicEnergyEnvironment>,
) {
    for (grid_pos, mut energies) in query
        .iter_mut()
        .into_group_map_by(|(_, grid_pos)| **grid_pos)
    {
        let mut energy_refs = energies.iter_mut().map(|(energy, _)| energy).collect_vec();
        distribute_energy(&mut environment, &mut energy_refs, &grid_pos);
    }
}

pub fn cell_collect_charge_energy_system(
    mut query: Query<(&mut CellEnergy, &GridPosition), With<CellRequestChargeEnergy>>,
    mut environment: ResMut<ChargeEnergyEnvironment>,
) {
    for (grid_pos, mut energies) in query
        .iter_mut()
        .into_group_map_by(|(_, grid_pos)| **grid_pos)
    {
        let mut energy_refs = energies.iter_mut().map(|(energy, _)| energy).collect_vec();
        distribute_energy(&mut environment, &mut energy_refs, &grid_pos);
    }
}

pub fn invoke_cell_genome_actions_system(
    _commands: Commands,
    _rng: Single<&mut WyRand, With<GlobalRng>>,
    mut cells: Query<(
        &GridPosition,
        &Cell,
        &mut CellEnergy,
        &Genome,
        &mut GenomeID,
    )>,
) {
    for (_grid_pos, _cell, _cell_energy, _genome, _genome_id) in cells.iter_mut() {
        // TODO: Implement
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cell_collect_organic_energy_system() {
        let mut world = World::new();

        let initial_env = OrganicEnergyEnvironment::new(10, 10, 100);
        world.insert_resource(initial_env);

        let grid_pos = GridPosition { x: 5, y: 5 };
        let cell_entity = world
            .spawn((
                Cell::Root,
                grid_pos,
                CellEnergy(0),
                CellRequestOrganicEnergy(grid_pos),
            ))
            .id();

        let system = world.register_system(cell_collect_organic_energy_system);
        _ = world.run_system(system);

        let cell_energy = *world
            .entity(cell_entity)
            .get::<CellEnergy>()
            .expect("Cell should have CellEnergy component");

        let mut updated_env = world
            .get_resource_mut::<OrganicEnergyEnvironment>()
            .expect("Should have OrganicEnergyEnvironment resource");

        assert_eq!(cell_energy.0, 100);
        assert_eq!(updated_env.collect(grid_pos.x, grid_pos.y), Some(0));
    }

    #[test]
    fn test_cell_collect_organic_energy_split_system() {
        let mut world = World::new();

        let initial_env = OrganicEnergyEnvironment::new(10, 10, 113);
        world.insert_resource(initial_env);

        let grid_pos = GridPosition { x: 5, y: 5 };
        let cell_entity_first = world
            .spawn((
                Cell::Root,
                grid_pos,
                CellEnergy(0),
                CellRequestOrganicEnergy(grid_pos),
            ))
            .id();

        let cell_entity_second = world
            .spawn((
                Cell::Root,
                grid_pos,
                CellEnergy(3),
                CellRequestOrganicEnergy(grid_pos),
            ))
            .id();

        let system = world.register_system(cell_collect_organic_energy_system);
        _ = world.run_system(system);

        let first_cell_energy = *world
            .entity(cell_entity_first)
            .get::<CellEnergy>()
            .expect("Cell should have CellEnergy component");

        let second_cell_energy = *world
            .entity(cell_entity_second)
            .get::<CellEnergy>()
            .expect("Cell should have CellEnergy component");

        let mut updated_env = world
            .get_resource_mut::<OrganicEnergyEnvironment>()
            .expect("Should have OrganicEnergyEnvironment resource");

        assert_eq!(first_cell_energy.0, 56);
        assert_eq!(second_cell_energy.0, 59);
        assert_eq!(updated_env.collect(grid_pos.x, grid_pos.y), Some(1));
        assert_eq!(updated_env.collect(grid_pos.x, grid_pos.y), Some(0));
    }
}
