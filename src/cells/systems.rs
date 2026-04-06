use bevy::{platform::collections::HashMap, prelude::*};
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use itertools::Itertools;

use crate::{
    GridPosition, SimulationStep, TILE_SIZE,
    cells::*,
    energy::{
        ChargeEnergyEnvironment, EnergyEnvironmentTrait, ORGANIC_TOXICICITY_LEVEL,
        OrganicEnergyEnvironment, SunlightCycle,
    },
    genes::{Genome, GenomeID},
};

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

pub fn kill_toxic_cells_system(
    mut commands: Commands,
    query: Query<(Entity, &GridPosition, &Cell), Without<CellIsDying>>,
    organic_energy_env: ResMut<OrganicEnergyEnvironment>,
    charge_energy_env: Res<ChargeEnergyEnvironment>,
) {
    for (entity, grid_pos, cell) in query.iter() {
        let organic_energy = organic_energy_env.peek(grid_pos.x, grid_pos.y).unwrap_or(0);
        let charge_energy = charge_energy_env.peek(grid_pos.x, grid_pos.y).unwrap_or(0);

        let organic_is_toxic =
            organic_energy > ORGANIC_TOXICICITY_LEVEL && !matches!(cell, Cell::Root);
        let charge_is_toxic =
            charge_energy > ORGANIC_TOXICICITY_LEVEL && !matches!(cell, Cell::Antenna);

        if organic_is_toxic || charge_is_toxic {
            commands.entity(entity).insert(CellIsDying);
        }
    }
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
            _ => continue,
        };
    }
}

pub fn cell_collect_solar_energy_system(
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
        debug_assert!(
            matches!(_cell, Cell::Sprout | Cell::Seed(_)),
            "Only Sprout and Seed cells should have genomes"
        );
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

#[derive(Message, Clone, Debug)]
pub struct SpawnChildCellMessage {
    pub parent: Entity,
    pub child_cell: Cell,
    pub child_genome: Genome,
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
