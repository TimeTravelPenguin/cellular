use bevy::{
    ecs::{
        entity::Entity,
        message::MessageWriter,
        query::QueryData,
        system::{Res, Single},
    },
    platform::collections::HashMap,
    prelude::{Commands, Query, With, info},
};
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use nonempty::NonEmpty;
use rand::RngExt;

use crate::{
    GridPosition, SimulationSettings,
    cells::{
        Cell, CellRelation, FacingDirection, NeighbouringCells, OrganismDepth, PreviousEnergy,
        render::DrawCellEvent,
        spawn::{ChildCellBundle, SpawnChildrenCellsMessage},
    },
    energy::{
        CellEnergy, ChargeEnergyEnvironment, EnergyEnvironmentTrait, NeighbouringEnergy,
        OrganicEnergyEnvironment,
    },
    genes::{
        Genome, GenomeID, PreconditionContext, PreconditionEvaluationResult, RelativeDirection,
    },
};

#[derive(QueryData)]
#[query_data(mutable)]
pub(super) struct GenomeExecutionQuery {
    entity: Entity,
    grid_pos: &'static GridPosition,
    facing_dir: &'static mut FacingDirection,
    cell: &'static mut Cell,
    genome: &'static Genome,
    genome_id: &'static mut GenomeID,
    cell_relation: &'static mut CellRelation,
    organism_depth: &'static OrganismDepth,
    cell_energy: &'static mut CellEnergy,
    previous_energy: &'static PreviousEnergy,
}

pub(super) fn execute_genome_system(
    mut commands: Commands,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    mut cells: Query<GenomeExecutionQuery>,
    mut spawn_writer: MessageWriter<SpawnChildrenCellsMessage>,
    organic_energy_env: Res<OrganicEnergyEnvironment>,
    charge_energy_env: Res<ChargeEnergyEnvironment>,
    _simulation_settings: Res<SimulationSettings>,
) {
    let cell_positions: HashMap<GridPosition, Cell> = cells
        .iter()
        .map(|cell| (*cell.grid_pos, *cell.cell))
        .collect();

    info!("Executing genome for {} cells", cells.iter().count());

    for mut cell in cells.iter_mut() {
        let neighbouring_organic_energy =
            NeighbouringEnergy::new(cell.grid_pos, &cell.facing_dir, &organic_energy_env);

        let neighbouring_charge_energy =
            NeighbouringEnergy::new(cell.grid_pos, &cell.facing_dir, &charge_energy_env);

        let neighbouring_cells =
            NeighbouringCells::new(*cell.grid_pos, **cell.facing_dir, &cell_positions);

        let forward_3x3_positions = cell
            .grid_pos
            .position_in_relative_direction(**cell.facing_dir, RelativeDirection::Forward)
            .neighbourhood();

        let left_3x3_positions = cell
            .grid_pos
            .position_in_relative_direction(**cell.facing_dir, RelativeDirection::Left)
            .neighbourhood();

        let right_3x3_positions = cell
            .grid_pos
            .position_in_relative_direction(**cell.facing_dir, RelativeDirection::Right)
            .neighbourhood();

        let unoccupied_nontoxic_3x3_forward = forward_3x3_positions
            .iter()
            .filter(|pos| {
                !cell_positions.contains_key(*pos)
                    && organic_energy_env.peek(pos.x, pos.y).unwrap_or(0.0)
                        < organic_energy_env.toxic_threshold()
                    && charge_energy_env.peek(pos.x, pos.y).unwrap_or(0.0)
                        < charge_energy_env.toxic_threshold()
            })
            .count();

        let unoccupied_nontoxic_3x3_left = left_3x3_positions
            .iter()
            .filter(|pos| {
                !cell_positions.contains_key(*pos)
                    && organic_energy_env.peek(pos.x, pos.y).unwrap_or(0.0)
                        < organic_energy_env.toxic_threshold()
                    && charge_energy_env.peek(pos.x, pos.y).unwrap_or(0.0)
                        < charge_energy_env.toxic_threshold()
            })
            .count();

        let unoccupied_nontoxic_3x3_right = right_3x3_positions
            .iter()
            .filter(|pos| {
                !cell_positions.contains_key(*pos)
                    && organic_energy_env.peek(pos.x, pos.y).unwrap_or(0.0)
                        < organic_energy_env.toxic_threshold()
                    && charge_energy_env.peek(pos.x, pos.y).unwrap_or(0.0)
                        < charge_energy_env.toxic_threshold()
            })
            .count();

        let precondition_context = PreconditionContext {
            neighbouring_organic_energy,
            neighbouring_charge_energy,
            neighbouring_cells,
            unoccupied_nontoxic_3x3_forward,
            unoccupied_nontoxic_3x3_left,
            unoccupied_nontoxic_3x3_right,
            organism_depth: **cell.organism_depth,
            cell_energy_has_increased: **cell.cell_energy > **cell.previous_energy,
            has_parent: cell.cell_relation.parent.is_some(),
            rng_value: rng.random(),
        };

        let genome = cell.genome;
        let genome_id = cell.genome_id;

        let _precondition_result = genome.eval_preconditions(*genome_id, &precondition_context);
        let is_multicellular = cell.cell_relation.parent.is_some();

        // HACK: Temporary until fully implemented
        let precondition_result = PreconditionEvaluationResult::Unset;

        match precondition_result {
            PreconditionEvaluationResult::Unset => {
                let entry = genome.get_entry(*genome_id);

                let children: Vec<ChildCellBundle> = vec![
                    entry.spawn.forward_cell_spawn.map(|c| ChildCellBundle {
                        cell: c,
                        facing_direction: *cell.facing_dir,
                        grid_pos: cell.grid_pos.position_in_relative_direction(
                            **cell.facing_dir,
                            RelativeDirection::Forward,
                        ),
                    }),
                    entry.spawn.left_cell_spawn.map(|c| ChildCellBundle {
                        cell: c,
                        facing_direction: FacingDirection(cell.facing_dir.left()),
                        grid_pos: cell.grid_pos.position_in_relative_direction(
                            **cell.facing_dir,
                            RelativeDirection::Left,
                        ),
                    }),
                    entry.spawn.right_cell_spawn.map(|c| ChildCellBundle {
                        cell: c,
                        facing_direction: FacingDirection(cell.facing_dir.right()),
                        grid_pos: cell.grid_pos.position_in_relative_direction(
                            **cell.facing_dir,
                            RelativeDirection::Right,
                        ),
                    }),
                ]
                .into_iter()
                .flatten()
                .collect();

                let children = NonEmpty::from_vec(children);

                let children = match children {
                    Some(children) => children,
                    None => continue, // No spawn defined, so do nothing
                };

                *cell.cell = Cell::Branch;
                spawn_writer.write(SpawnChildrenCellsMessage {
                    parent: cell.entity,
                    new_cells: children,
                });

                commands.entity(cell.entity).trigger(DrawCellEvent);
            }
            PreconditionEvaluationResult::Met => {
                let entry = genome.get_entry(*genome_id);
                if is_multicellular {
                    let command = &entry.multi_cell_commands.preconditions_met_command;
                    todo!(
                        "Execute multi-cell command for met preconditions: {:?}",
                        command
                    );
                } else {
                    let command = &entry.single_cell_commands.preconditions_met_command;
                    todo!(
                        "Execute single-cell command for met preconditions: {:?}",
                        command
                    );
                }
            }
            PreconditionEvaluationResult::Unmet => {
                let entry = genome.get_entry(*genome_id);
                if is_multicellular {
                    let command = &entry.multi_cell_commands.preconditions_unmet_command;
                    todo!(
                        "Execute multi-cell command for unmet preconditions: {:?}",
                        command
                    );
                } else {
                    let command = &entry.single_cell_commands.preconditions_unmet_command;
                    todo!(
                        "Execute single-cell command for unmet preconditions: {:?}",
                        command
                    );
                }
            }
        }
    }
}
