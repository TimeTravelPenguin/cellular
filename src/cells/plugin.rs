use bevy::{
    app::{App, FixedFirst, FixedPostUpdate, FixedUpdate, Plugin},
    ecs::schedule::{IntoScheduleConfigs, SystemSet},
};

use crate::cells::{
    CellEnergyTransferMessage, RemoveChildCellMessage, RequestDeathMessage,
    collection::{
        antenna_cell_collect_energy_system, leaf_cell_collect_energy_system,
        root_cell_collect_energy_system,
    },
    death::{apply_living_cost_system, apply_remove_child_system, handle_cell_death_system},
    execution::execute_genome_system,
    lifecycle::{sync_cell_markers_system, update_previous_energy_system},
    transfer::{cell_pass_energy_system, cell_receive_energy_system},
};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CellSystemsSet {
    EnergyCollection,
    EnergyTransfer,
    GenomeExecution,
    CellSpawn,
    CellDeath,
}

#[derive(Debug, Clone, Copy)]
pub struct CellSystemsPlugin;

impl Plugin for CellSystemsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CellEnergyTransferMessage>()
            .add_message::<RequestDeathMessage>()
            .add_message::<RemoveChildCellMessage>()
            .add_systems(FixedFirst, sync_cell_markers_system)
            .add_systems(
                FixedUpdate,
                (
                    (
                        leaf_cell_collect_energy_system,
                        root_cell_collect_energy_system,
                        antenna_cell_collect_energy_system,
                    )
                        .in_set(CellSystemsSet::EnergyCollection),
                    (cell_pass_energy_system, cell_receive_energy_system)
                        .chain()
                        .in_set(CellSystemsSet::EnergyTransfer),
                    execute_genome_system.in_set(CellSystemsSet::GenomeExecution),
                ),
            )
            .add_systems(
                FixedPostUpdate,
                (
                    apply_living_cost_system,
                    update_previous_energy_system,
                    handle_cell_death_system.in_set(CellSystemsSet::CellDeath),
                    apply_remove_child_system,
                )
                    .chain(),
            )
            .configure_sets(
                FixedUpdate,
                (
                    CellSystemsSet::EnergyCollection.before(CellSystemsSet::EnergyTransfer),
                    CellSystemsSet::EnergyTransfer
                        .after(CellSystemsSet::EnergyCollection)
                        .before(CellSystemsSet::GenomeExecution),
                    CellSystemsSet::GenomeExecution.after(CellSystemsSet::EnergyTransfer),
                ),
            )
            .configure_sets(
                FixedPostUpdate,
                CellSystemsSet::CellSpawn.before(CellSystemsSet::CellDeath),
            );
    }
}
