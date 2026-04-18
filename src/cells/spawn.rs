use bevy::{platform::collections::HashSet, prelude::*};
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use itertools::Itertools;
use nonempty::NonEmpty;
use rand::RngExt;
use thiserror::Error;

use crate::{
    Cell, CellEnergy, CellPositions, Entity, FacingDirection, Genome, GenomeID, GridPosition,
    OrganismDepth, RemainingTicksWithoutEnergy, SimulationSettings,
    cells::{CellRelation, CellSystemsSet, NewCellEvent, PreviousEnergy, render::DrawCellEvent},
};

#[derive(Debug, Clone, Copy)]
pub struct CellSpawnPlugin;

impl Plugin for CellSpawnPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<SpawnCellMessage>()
            .add_message::<SpawnChildrenCellsMessage>()
            .add_message::<SpawnChildCellsResultMessage>()
            .add_systems(
                FixedPostUpdate,
                (spawn_children_cells_system, spawn_cell)
                    .chain()
                    .in_set(CellSystemsSet::CellSpawn),
            );
    }
}

#[derive(Error, Debug)]
pub enum SpawnCellError {
    #[error("The grid position ({}, {}) is already occupied by another cell.", .0.x, .0.y)]
    PositionOccupied(GridPosition),
    #[error("The cell type {0:?} cannot be spawned at the given position.")]
    NotEnoughEnergy(CellEnergy),
}

#[derive(Bundle, Debug, Clone)]
pub struct ChildCellBundle {
    pub grid_pos: GridPosition,
    pub cell: Cell,
    pub facing_direction: FacingDirection,
}

#[derive(Bundle, Debug, Clone)]
pub struct NewCellBundle {
    pub grid_pos: GridPosition,
    pub facing_direction: FacingDirection,
    pub cell: Cell,
    pub cell_energy: CellEnergy,
    pub genome: Genome,
    pub genome_id: GenomeID,
    pub organism_depth: OrganismDepth,
    pub remaining_ticks_without_energy: RemainingTicksWithoutEnergy,
}

#[derive(Message)]
pub struct SpawnCellMessage {
    pub parent: Option<Entity>,
    pub new_cell: NewCellBundle,
}

#[derive(Message)]
pub struct SpawnChildrenCellsMessage {
    pub parent: Entity,
    pub new_cells: NonEmpty<ChildCellBundle>,
}

#[derive(Message)]
pub struct SpawnChildCellsResultMessage {
    pub parent: Entity,
    pub child: Result<Entity, SpawnCellError>,
}

fn spawn_children_cells_system(
    mut parent_cells: Query<(&Genome, &mut CellEnergy, &OrganismDepth), With<Cell>>,
    mut reader: MessageReader<SpawnChildrenCellsMessage>,
    mut error_writer: MessageWriter<SpawnChildCellsResultMessage>,
    mut spawn_writer: MessageWriter<SpawnCellMessage>,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    grid_positions: Res<CellPositions>,
    settings: Res<SimulationSettings>,
) {
    let reproduction_cost = settings.config.cell_action_costs.reproduce_cost;
    let ticks_without_energy = settings.config.cell_defaults.max_ticks_without_energy;

    for msg in reader.read() {
        let SpawnChildrenCellsMessage { parent, new_cells } = msg;

        let Ok((genome, mut parent_energy, depth)) = parent_cells.get_mut(*parent) else {
            panic!(
                "Parent entity {:?} does not have CellEnergy component.",
                parent
            );
        };

        if **parent_energy < reproduction_cost {
            error_writer.write(SpawnChildCellsResultMessage {
                parent: *parent,
                child: Err(SpawnCellError::NotEnoughEnergy(*parent_energy)),
            });

            continue;
        }

        **parent_energy -= reproduction_cost;

        let spawnable = new_cells
            .iter()
            .filter(|cell| !grid_positions.contains(&cell.grid_pos))
            .collect_vec();

        let num_spawnable = spawnable.len() as f32;
        let split_energy = **parent_energy / num_spawnable;
        **parent_energy = 0.0;

        for cell in spawnable {
            let new_cell = NewCellBundle {
                grid_pos: cell.grid_pos,
                facing_direction: cell.facing_direction,
                cell: cell.cell,
                cell_energy: CellEnergy(split_energy),
                genome: genome.clone(),
                genome_id: rng.random(), // TODO: Is this right?
                organism_depth: OrganismDepth(**depth + 1),
                remaining_ticks_without_energy: RemainingTicksWithoutEnergy(ticks_without_energy),
            };

            spawn_writer.write(SpawnCellMessage {
                parent: Some(*parent),
                new_cell,
            });
        }
    }
}

pub fn spawn_cell(
    mut commands: Commands,
    mut grid_positions: ResMut<CellPositions>,
    mut reader: MessageReader<SpawnCellMessage>,
) {
    for msg in reader.read() {
        let SpawnCellMessage { parent, new_cell } = msg;

        if !grid_positions.insert(new_cell.grid_pos) {
            warn!(
                "Skipping cell spawn at {:?} — tile already occupied.",
                new_cell.grid_pos
            );
            continue;
        }

        commands
            .spawn((
                new_cell.clone(),
                PreviousEnergy(*new_cell.cell_energy),
                CellRelation {
                    parent: *parent,
                    children: HashSet::new(),
                },
            ))
            .trigger(NewCellEvent)
            .trigger(DrawCellEvent);
    }
}
