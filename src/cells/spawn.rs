use bevy::{ecs::system::Commands, log::info};

use crate::{
    Cell, CellEnergy, Entity, FacingDirection, Genome, GenomeID, GridPosition, OrganismDepth,
    RemainingTicksWithoutEnergy,
    cells::{CellRelation, NewCellEvent, PreviousEnergy},
};

pub fn spawn_cell(
    commands: &mut Commands,
    grid_pos: GridPosition,
    facing_direction: FacingDirection,
    cell: Cell,
    cell_energy: CellEnergy,
    genome: Genome,
    genome_id: GenomeID,
    cell_relation: CellRelation,
    organism_depth: OrganismDepth,
    remaining_ticks_without_energy: RemainingTicksWithoutEnergy,
) -> Entity {
    info!(
        "Spawning cell at ({}, {}) of type {:?}",
        grid_pos.x, grid_pos.y, cell,
    );

    commands
        .spawn((
            grid_pos,
            facing_direction,
            cell,
            genome,
            genome_id,
            cell_relation,
            organism_depth,
            cell_energy,
            PreviousEnergy(*cell_energy),
            remaining_ticks_without_energy,
        ))
        .trigger(|entity| NewCellEvent {
            entity,
            grid_pos,
            cell,
            facing_direction,
        })
        .id()
}
