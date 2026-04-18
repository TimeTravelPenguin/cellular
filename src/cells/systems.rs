use std::ops::AddAssign;

use bevy::{
    app::{App, FixedPostUpdate, FixedUpdate, Plugin},
    ecs::{
        entity::Entity,
        message::{MessageReader, MessageWriter},
        observer::On,
        query::QueryData,
        schedule::{IntoScheduleConfigs, SystemSet},
        system::{Res, Single},
    },
    platform::collections::{HashMap, HashSet},
    prelude::{Commands, Query, ResMut, With, Without, info},
};
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use rand::RngExt;

use crate::{
    GridPosition, SimulationSettings,
    cells::{
        render::{DrawCellEvent, draw_new_cells_system},
        spawn::spawn_cell,
        *,
    },
    energy::{
        ChargeEnergyEnvironment, EnergyEnvironmentTrait, NeighbouringEnergy,
        OrganicEnergyEnvironment,
    },
    genes::{
        Genome, GenomeID, PreconditionContext, PreconditionEvaluationResult, RelativeDirection,
    },
};

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum CellSystemsSet {
    EnergyCollection,
    EnergyTransfer,
    GenomeExecution,
}

#[derive(Debug, Clone, Copy)]
pub struct CellPlugin;

impl Plugin for CellPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<CellEnergyTransferMessage>()
            .add_message::<RequestDeathMessage>()
            .add_message::<RemoveChildCellMessage>()
            .add_observer(|event: On<NewCellEvent>, mut commands: Commands| {
                commands.entity(event.entity).trigger(DrawCellEvent);
            })
            .add_observer(draw_new_cells_system)
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
                    handle_cell_death_system,
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
            );
    }
}

fn leaf_cell_collect_energy_system(
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

    for (grid_pos, mut energy) in query.iter_mut() {
        let neighbors = [
            (grid_pos.x - 1, grid_pos.y - 1),
            (grid_pos.x - 1, grid_pos.y + 1),
            (grid_pos.x - 1, grid_pos.y),
            (grid_pos.x + 1, grid_pos.y - 1),
            (grid_pos.x + 1, grid_pos.y + 1),
            (grid_pos.x + 1, grid_pos.y),
            (grid_pos.x, grid_pos.y - 1),
            (grid_pos.x, grid_pos.y + 1),
        ];

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

fn root_cell_collect_energy_system(
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

fn antenna_cell_collect_energy_system(
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

fn cell_pass_energy_system(
    mut query: Query<(Entity, &mut CellEnergy, &CellRelation), With<CellEnergy>>,
    mut transfer_writer: MessageWriter<CellEnergyTransferMessage>,
) {
    for (entity, energy, relation) in query.iter_mut() {
        if energy.0 <= 0.0 {
            continue; // No energy to transfer
        }

        let transfer_amount = energy.0 / relation.children.len() as f32;
        for &child in &relation.children {
            transfer_writer.write(CellEnergyTransferMessage {
                from: entity,
                to: child,
                amount: transfer_amount,
            });
        }
    }
}

fn cell_receive_energy_system(
    mut query: Query<(Entity, &mut CellEnergy)>,
    mut transfer_reader: MessageReader<CellEnergyTransferMessage>,
) {
    let mut energy_map = HashMap::new();

    for transfer in transfer_reader.read() {
        energy_map
            .entry(transfer.to)
            .or_insert(0.0)
            .add_assign(transfer.amount);

        // Remove energy from sender
        let (_, mut sender_energy) = query.get_mut(transfer.from).unwrap();
        sender_energy.0 = (sender_energy.0 - transfer.amount).max(0.0);
    }

    // Apply received energy to recipients
    for (entity, mut energy) in query.iter_mut() {
        if let Some(amount) = energy_map.get(&entity) {
            energy.0 += *amount;
        }
    }
}

fn update_previous_energy_system(mut query: Query<(&mut PreviousEnergy, &CellEnergy), With<Cell>>) {
    for (mut previous_energy, energy) in query.iter_mut() {
        previous_energy.0 = energy.0;
    }
}

fn has_enough_energy_for_spawn(
    cell_energy: CellEnergy,
    spawn: &[Cell],
    settings: &SimulationSettings,
) -> bool {
    let total_cost = settings.config.cell_action_costs.reproduce_cost * spawn.len() as f32;

    cell_energy.0 >= total_cost
}

#[derive(QueryData)]
#[query_data(mutable)]
struct GenomeExecutionQuery {
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

fn execute_genome_system(
    mut commands: Commands,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    mut cells: Query<GenomeExecutionQuery>,
    organic_energy_env: Res<OrganicEnergyEnvironment>,
    charge_energy_env: Res<ChargeEnergyEnvironment>,
    simulation_settings: Res<SimulationSettings>,
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

        let precondition_result = PreconditionEvaluationResult::Unset; // TEMP

        match precondition_result {
            PreconditionEvaluationResult::Unset => {
                let entry = genome.get_entry(*genome_id);
                let spawned = entry.spawn.into_iter().collect::<Vec<_>>();

                if spawned.is_empty()
                    || !has_enough_energy_for_spawn(
                        *cell.cell_energy,
                        &spawned,
                        &simulation_settings,
                    )
                {
                    return; // Not enough energy to spawn or no spawn defined
                }

                let energies = **cell.cell_energy / spawned.len() as f32;

                for spawned_cell in &spawned {
                    let entity = spawn_cell(
                        &mut commands,
                        *cell.grid_pos,
                        *cell.facing_dir,
                        *spawned_cell,
                        CellEnergy(energies),
                        genome.clone(),
                        *genome_id,
                        CellRelation {
                            parent: Some(cell.entity),
                            children: HashSet::new(),
                        },
                        OrganismDepth(**cell.organism_depth + 1),
                        RemainingTicksWithoutEnergy(
                            simulation_settings
                                .config
                                .cell_defaults
                                .max_ticks_without_energy,
                        ),
                    );

                    cell.cell_relation.children.insert(entity);
                }

                **cell.cell_energy = 0.0;
                *cell.cell = Cell::Branch;

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

fn apply_living_cost_system(
    mut query: Query<(
        Entity,
        &mut CellEnergy,
        &Cell,
        &mut RemainingTicksWithoutEnergy,
    )>,
    settings: Res<SimulationSettings>,
    mut writer: MessageWriter<RequestDeathMessage>,
) {
    info!("Applying living costs for {} cells", query.iter().count());

    for (entity, mut energy, cell, mut remaining) in query.iter_mut() {
        let cost = match *cell {
            Cell::Leaf => settings.config.cell_living_costs.leaf_living_cost,
            Cell::Root => settings.config.cell_living_costs.root_living_cost,
            Cell::Antenna => settings.config.cell_living_costs.antenna_living_cost,
            Cell::Branch => settings.config.cell_living_costs.branch_living_cost,
            Cell::Sprout => settings.config.cell_living_costs.sprout_living_cost,
            Cell::Seed => settings.config.cell_living_costs.seed_living_cost,
        };

        **energy = (**energy - cost).max(0.0);

        if **energy <= 0.0 {
            if **remaining == 0 {
                writer.write(RequestDeathMessage { entity });
            } else {
                **remaining -= 1;
            }
        }
    }
}

fn handle_cell_death_system(
    mut commands: Commands,
    query: Query<(&Cell, &GridPosition, &CellRelation, &CellEnergy)>,
    mut organic_env: ResMut<OrganicEnergyEnvironment>,
    mut charge_env: ResMut<ChargeEnergyEnvironment>,
    settings: Res<SimulationSettings>,
    mut reader: MessageReader<RequestDeathMessage>,
    mut writer: MessageWriter<RemoveChildCellMessage>,
) {
    for msg in reader.read() {
        let Ok((cell, pos, relation, energy)) = query.get(msg.entity) else {
            continue;
        };

        relation.children.iter().for_each(|&child| {
            writer.write(RemoveChildCellMessage {
                parent: msg.entity,
                child,
            });
        });

        if let Some(parent) = relation.parent {
            writer.write(RemoveChildCellMessage {
                parent,
                child: msg.entity,
            });
        }

        let death_energy_config = &settings.config.cell_death_energy_redistribution;
        let organic_released = match cell {
            Cell::Leaf => death_energy_config.leaf_organic_death_energy,
            Cell::Root => death_energy_config.root_organic_death_energy,
            Cell::Antenna => death_energy_config.antenna_organic_death_energy,
            Cell::Branch => death_energy_config.branch_organic_death_energy,
            Cell::Sprout => death_energy_config.sprout_organic_death_energy,
            Cell::Seed => death_energy_config.seed_organic_death_energy,
        };

        organic_env.add(pos.x, pos.y, organic_released);

        let remaining_energy = **energy - organic_released;
        if remaining_energy > 0.0 {
            charge_env.add(pos.x, pos.y, remaining_energy);
        }

        commands.entity(msg.entity).despawn();
    }
}
