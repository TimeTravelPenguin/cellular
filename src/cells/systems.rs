use std::ops::AddAssign;

use bevy::{
    app::{App, FixedUpdate, Plugin},
    ecs::{
        entity::Entity,
        message::{MessageReader, MessageWriter},
        observer::On,
        query::QueryData,
        schedule::{IntoScheduleConfigs, SystemSet},
        system::{Res, Single},
    },
    platform::collections::{HashMap, HashSet},
    prelude::{
        Assets, ColorMaterial, Commands, EntityCommands, Mesh, MeshMaterial2d, Quat, Query, ResMut,
        Transform, With, Without, default, info,
    },
};
use bevy_rand::{global::GlobalRng, prelude::WyRand, traits::ForkableInnerSeed};
use rand::{RngExt, SeedableRng};

use crate::{
    GridPosition, SimulationSettings,
    cells::{
        AntennaCell, Cell, CellEnergy, CellEnergyTransferMessage, CellRelation, CellRenderBundle,
        CellVisualSpec, Direction, FacingDirection, LeafCell, NeighbouringCells, NewCellEvent,
        OrganismDepth, PreviousEnergy, RootCell,
    },
    energy::{
        ChargeEnergyEnvironment, EnergyEnvironmentTrait, NeighbouringEnergy,
        OrganicEnergyEnvironment,
    },
    genes::{Genome, GenomeID, PreconditionContext, RelativeDirection},
    input::{observe_cell_hover, observe_cell_out},
    utils::grid_pos_to_world_pos,
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
        app.add_observer(draw_new_cells_system)
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

/// Computes the rotation needed to orient a cell in the specified facing direction.
fn facing_rotation(direction: Direction) -> Quat {
    match direction {
        Direction::East => Quat::IDENTITY,
        Direction::South => Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
        Direction::West => Quat::from_rotation_z(std::f32::consts::PI),
        Direction::North => Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
    }
}

/// Computes the world transform for a cell based on its grid position and facing direction.
pub fn cell_transform(grid_pos: &GridPosition, facing: Direction) -> Transform {
    let translation = grid_pos_to_world_pos(grid_pos);

    Transform {
        translation,
        rotation: facing_rotation(facing),
        ..default()
    }
}

/// Inserts the necessary components to render a cell based on its visual specification.
fn insert_cell_visual(
    entity_commands: &mut EntityCommands,
    spec: CellVisualSpec,
    transform: Transform,
    grid_pos: GridPosition,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ColorMaterial>,
) {
    let mesh = spec.shape.into_mesh(meshes);
    let material = MeshMaterial2d(materials.add(ColorMaterial::from_color(spec.color)));

    entity_commands.insert((
        CellRenderBundle {
            mesh,
            material,
            transform,
        },
        grid_pos,
    ));

    entity_commands.with_children(|parent| {
        for child in spec.children {
            parent.spawn(CellRenderBundle {
                mesh: child.shape.into_mesh(meshes),
                material: MeshMaterial2d(materials.add(ColorMaterial::from_color(child.color))),
                transform: child.transform,
            });
        }
    });
}

/// System to create visual entities for cells that don't already have them.
fn draw_new_cells_system(
    event: On<NewCellEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let transform = cell_transform(&event.grid_pos, *event.facing_direction);
    let spec = event.cell.visual_spec();

    info!(
        "Spawning cell at ({}, {}) of type {:?}",
        event.grid_pos.x, event.grid_pos.y, event.cell,
    );

    let mut entity_commands = commands.entity(event.entity);
    insert_cell_visual(
        &mut entity_commands,
        spec,
        transform,
        event.grid_pos,
        &mut meshes,
        &mut materials,
    );

    entity_commands
        .observe(observe_cell_hover)
        .observe(observe_cell_out);
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

#[derive(QueryData)]
#[query_data(mutable)]
struct GenomeExecutionQuery {
    grid_pos: &'static GridPosition,
    facing_dir: &'static FacingDirection,
    cell: &'static mut Cell,
    genome: &'static Genome,
    genome_id: &'static mut GenomeID,
    cell_relation: &'static CellRelation,
    organism_depth: &'static OrganismDepth,
    cell_energy: &'static CellEnergy,
    previous_energy: &'static PreviousEnergy,
}

fn execute_genome_system(
    _commands: Commands,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    mut cells: Query<GenomeExecutionQuery>,
    organic_energy_env: Res<OrganicEnergyEnvironment>,
    charge_energy_env: Res<ChargeEnergyEnvironment>,
) {
    let cell_positions: HashMap<GridPosition, Cell> = cells
        .iter()
        .map(|cell| (*cell.grid_pos, *cell.cell))
        .collect();

    for cell in cells.iter_mut() {
        let neighbouring_organic_energy =
            NeighbouringEnergy::new(cell.grid_pos, cell.facing_dir, &organic_energy_env);

        let neighbouring_charge_energy =
            NeighbouringEnergy::new(cell.grid_pos, cell.facing_dir, &charge_energy_env);

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

        continue;
        let _precondition_context = PreconditionContext {
            neighbouring_organic_energy,
            neighbouring_charge_energy,
            neighbouring_cells,
            unoccupied_nontoxic_3x3_forward,
            unoccupied_nontoxic_3x3_left,
            unoccupied_nontoxic_3x3_right,
            organism_depth: todo!(),
            cell_energy_has_increased: todo!(),
            has_parent: cell.cell_relation.parent.is_some(),
            rng_value: rng.random(),
        };

        todo!();
    }
}
