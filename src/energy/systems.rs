use bevy::{
    platform::collections::HashMap,
    prelude::{Commands, Entity, Mut, Query, Res, ResMut, Resource, With, Without, World, info},
};
use itertools::Itertools;

use crate::{
    SimulationStep,
    cells::{Cell, CellIsDying},
    energy::{
        CHARGE_TOXICITY_LEVEL, CellEnergy, CellRequestChargeEnergy, CellRequestOrganicEnergy,
        CellRequestSolarEnergy, ChargeEnergyEnvironment, EnergyEnvironmentTrait, EnergyTransferer,
        GridPosition, ORGANIC_TOXICICITY_LEVEL, OrganicEnergyEnvironment, SunlightCycle,
    },
};

pub fn charge_energy_system(mut charge_env: ResMut<ChargeEnergyEnvironment>) {
    charge_env.charge();
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
            charge_energy > CHARGE_TOXICITY_LEVEL && !matches!(cell, Cell::Antenna);

        if organic_is_toxic || charge_is_toxic {
            commands.entity(entity).insert(CellIsDying);
        }
    }
}

/// Adds energy request components to cells based on their type. Leaf cells
/// request solar energy, antenna cells request charge energy, and root cells
/// request organic energy.
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

/// Distributes energy from the environment to the cells at the given grid
/// position, splitting it evenly among the cells. If the environment doesn't
/// have enough energy to give each cell at least
/// 1 unit, no energy is distributed.
pub fn distribute_energy<'a, T: Resource + EnergyEnvironmentTrait>(
    environment: &mut ResMut<T>,
    energies: &mut [&mut Mut<'a, CellEnergy>],
    grid_position: &GridPosition,
) {
    let energy_per_cell = environment
        .collect_split(grid_position.x, grid_position.y, energies.len())
        .unwrap_or(0);

    if energy_per_cell == 0 {
        return;
    }

    for energy in energies {
        energy.0 += energy_per_cell;
    }
}

/// Collects energy from the environment for cells that have requested organic
/// energy, splitting it evenly among the cells at the same grid position.
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

/// Collects energy from the environment for cells that have requested charge
/// energy, splitting it evenly among the cells at the same grid position.
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

/// Transfers energy from cells with `EnergyTransferer` component to their
/// specified recipients. The energy is split evenly among the recipients, and
/// if the transferer doesn't have enough energy to give each recipient at least
/// 1 unit, no energy is transferred.
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
