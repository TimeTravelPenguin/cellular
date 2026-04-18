use crate::{
    GridPosition,
    cells::Cell,
    config::CellDeathEnergyRedistributionConfig,
    energy::{
        ChargeEnergyEnvironment, EnergyEnvironmentTrait, OrganicEnergyEnvironment,
    },
};

/// Deposits the energy of a dying cell back into the environment: a
/// cell-type-specific share goes to organic soil, the rest becomes charge.
pub fn disperse_on_death(
    cell: Cell,
    pos: GridPosition,
    total_energy: f32,
    organic_env: &mut OrganicEnergyEnvironment,
    charge_env: &mut ChargeEnergyEnvironment,
    config: &CellDeathEnergyRedistributionConfig,
) {
    let organic_released = match cell {
        Cell::Leaf => config.leaf_organic_death_energy,
        Cell::Root => config.root_organic_death_energy,
        Cell::Antenna => config.antenna_organic_death_energy,
        Cell::Branch => config.branch_organic_death_energy,
        Cell::Sprout => config.sprout_organic_death_energy,
        Cell::Seed => config.seed_organic_death_energy,
    };

    organic_env.add(pos.x, pos.y, organic_released);

    let remaining_energy = total_energy - organic_released;
    if remaining_energy > 0.0 {
        charge_env.add(pos.x, pos.y, remaining_energy);
    }
}
