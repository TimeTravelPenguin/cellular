use bevy::{
    ecs::system::Res,
    platform::collections::HashSet,
    prelude::{Query, ResMut, With, Without},
};

use crate::{
    GridPosition, SimulationSettings,
    cells::{AntennaCell, LeafCell, RootCell},
    energy::{
        CellEnergy, ChargeEnergyEnvironment, EnergyEnvironmentTrait, OrganicEnergyEnvironment,
    },
};

pub(super) fn leaf_cell_collect_energy_system(
    mut query: Query<
        (&GridPosition, &mut CellEnergy),
        (With<LeafCell>, Without<RootCell>, Without<AntennaCell>),
    >,
    other_cells: Query<&GridPosition, Without<LeafCell>>,
    organic_env: Res<OrganicEnergyEnvironment>,
    settings: Res<SimulationSettings>,
) {
    // mn = LIGHTENERGY
    // for each of 8 neighbors:
    //     if neighbor is LEAF → return 0  // complete shading
    //     if neighbor exists (any cell) → mn -= 1
    // return OrganicMap[X][Y] * mn * LIGHTCOEF  // organic * (10 - obstructions) * 0.0008

    // TODO: Can this be optimised be using `Query<&GridPosition, With<Cell>>` for `other_cells`?
    let leaf_positions: HashSet<_> = query.iter().map(|(pos, _)| (pos.x, pos.y)).collect();
    let other_positions: HashSet<_> = other_cells.iter().map(|pos| (pos.x, pos.y)).collect();

    let coeff = settings.config.environment.light_coef;
    let light_energy = settings.config.environment.light_energy;

    const NEIGHBOR_DELTAS: [(isize, isize); 8] = [
        (-1, -1),
        (-1, 1),
        (-1, 0),
        (1, -1),
        (1, 1),
        (1, 0),
        (0, -1),
        (0, 1),
    ];

    for (grid_pos, mut energy) in query.iter_mut() {
        let neighbors: Vec<(usize, usize)> = NEIGHBOR_DELTAS
            .iter()
            .filter_map(|&(dx, dy)| {
                let nx = grid_pos.x.checked_add_signed(dx)?;
                let ny = grid_pos.y.checked_add_signed(dy)?;
                Some((nx, ny))
            })
            .collect();

        let has_leaf_neighbor = neighbors
            .iter()
            .any(|&(nx, ny)| leaf_positions.contains(&(nx, ny)));

        if has_leaf_neighbor {
            continue; // completely shaded, no energy gain
        }

        let obstruction_count = neighbors
            .iter()
            .filter(|&&(nx, ny)| other_positions.contains(&(nx, ny)))
            .count() as f32;

        let mut env_energy = (light_energy - obstruction_count) * coeff;
        env_energy *= organic_env.peek(grid_pos.x, grid_pos.y).unwrap_or(0.0);
        energy.0 += env_energy.max(0.0);
    }
}

pub(super) fn root_cell_collect_energy_system(
    mut query: Query<
        (&GridPosition, &mut CellEnergy),
        (With<RootCell>, Without<AntennaCell>, Without<LeafCell>),
    >,
    mut organic_env: ResMut<OrganicEnergyEnvironment>,
    settings: Res<SimulationSettings>,
) {
    for (grid_pos, mut energy) in query.iter_mut() {
        let energy_rate = settings.config.extraction_rates.root_extract_rate;
        let env_energy = organic_env
            .take(grid_pos.x, grid_pos.y, energy_rate)
            .unwrap_or(0.0);

        energy.0 += env_energy;
    }
}

pub(super) fn antenna_cell_collect_energy_system(
    mut query: Query<
        (&GridPosition, &mut CellEnergy),
        (With<AntennaCell>, Without<RootCell>, Without<LeafCell>),
    >,
    mut charge_env: ResMut<ChargeEnergyEnvironment>,
    settings: Res<SimulationSettings>,
) {
    for (grid_pos, mut energy) in query.iter_mut() {
        let energy_rate = settings.config.extraction_rates.antenna_extract_rate;
        let env_energy = charge_env
            .take(grid_pos.x, grid_pos.y, energy_rate)
            .unwrap_or(0.0);

        energy.0 += env_energy;
    }
}
