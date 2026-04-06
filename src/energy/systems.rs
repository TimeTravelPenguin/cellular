use bevy::prelude::ResMut;

use crate::ChargeEnergyEnvironment;

pub fn charge_energy_system(mut charge_env: ResMut<ChargeEnergyEnvironment>) {
    charge_env.charge();
}
