use bevy::{
    ecs::{entity::Entity, query::Changed},
    prelude::{Commands, Query, With},
};

use crate::{
    cells::{
        AntennaCell, BranchCell, Cell, LeafCell, PreviousEnergy, RootCell, SeedCell, SproutCell,
    },
    energy::CellEnergy,
};

/// Keeps per-type marker components (`LeafCell`, `RootCell`, ...) in sync with
/// the `Cell` enum value. Runs on any `Cell` insertion or mutation.
pub(super) fn sync_cell_markers_system(
    mut commands: Commands,
    query: Query<(Entity, &Cell), Changed<Cell>>,
) {
    for (entity, cell) in query.iter() {
        let mut ec = commands.entity(entity);
        ec.remove::<LeafCell>()
            .remove::<AntennaCell>()
            .remove::<RootCell>()
            .remove::<SproutCell>()
            .remove::<BranchCell>()
            .remove::<SeedCell>();

        match cell {
            Cell::Leaf => {
                ec.insert(LeafCell);
            }
            Cell::Antenna => {
                ec.insert(AntennaCell);
            }
            Cell::Root => {
                ec.insert(RootCell);
            }
            Cell::Sprout => {
                ec.insert(SproutCell);
            }
            Cell::Branch => {
                ec.insert(BranchCell);
            }
            Cell::Seed => {
                ec.insert(SeedCell);
            }
        }
    }
}

pub(super) fn update_previous_energy_system(
    mut query: Query<(&mut PreviousEnergy, &CellEnergy), With<Cell>>,
) {
    for (mut previous_energy, energy) in query.iter_mut() {
        previous_energy.0 = energy.0;
    }
}
