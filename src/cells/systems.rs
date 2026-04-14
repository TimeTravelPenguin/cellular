use std::ops::AddAssign;

use bevy::{
    app::{App, Plugin},
    ecs::{
        entity::Entity,
        message::{MessageReader, MessageWriter},
        observer::On,
        system::{Res, Single},
    },
    platform::collections::{HashMap, HashSet},
    prelude::{
        Assets, ColorMaterial, Commands, EntityCommands, Mesh, MeshMaterial2d, Quat, Query, ResMut,
        Transform, With, Without, default, info,
    },
};
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use rand::RngExt;

use crate::{
    GridPosition, SimulationSettings,
    cells::{
        AntennaCell, Cell, CellEnergy, CellEnergyTransferMessage, CellRelation, CellRenderBundle,
        CellVisualSpec, Direction, FacingDirection, LeafCell, NewCellEvent, RootCell, SeedCell,
    },
    energy::{
        ChargeEnergyEnvironment, EnergyEnvironmentTrait, NeighbouringEnergy,
        OrganicEnergyEnvironment,
    },
    genes::{
        Genome, GenomeID, MultiCellCommand, ObstacleInfo, PreconditionParameters, SingleCellCommand,
    },
    input::{observe_cell_hover, observe_cell_out},
    utils::grid_pos_to_world_pos,
};

#[derive(Debug, Clone, Copy)]
pub struct CellPlugin;

impl Plugin for CellPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(draw_new_cells_system);
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
pub fn insert_cell_visual(
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

pub fn leaf_cell_collect_energy_system(
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

pub fn root_cell_collect_energy_system(
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

pub fn antenna_cell_collect_energy_system(
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

pub fn cell_pass_energy_system(
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

pub fn cell_receive_energy_system(
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

pub fn execute_genome_system(
    commands: Commands,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    cell_positions: Query<&GridPosition, With<Cell>>,
    mut cells: Query<(
        &GridPosition,
        &FacingDirection,
        &mut Cell,
        &Genome,
        &mut GenomeID,
    )>,
    organic_energy_env: Res<OrganicEnergyEnvironment>,
    charge_energy_env: Res<ChargeEnergyEnvironment>,
) {
    let cell_positions: HashSet<GridPosition> = cell_positions.iter().cloned().collect();
    for (grid_pos, facing_dir, mut cell, genome, mut genome_id) in cells.iter_mut() {
        let organic_energy = NeighbouringEnergy::new(grid_pos, facing_dir, &organic_energy_env);
        let charge_energy = NeighbouringEnergy::new(grid_pos, facing_dir, &charge_energy_env);

        let obstacles = ObstacleInfo {
            left: cell_positions.contains(&grid_pos.offset(facing_dir.left().delta())),
            forward: cell_positions.contains(&grid_pos.offset(facing_dir.forward().delta())),
            right: cell_positions.contains(&grid_pos.offset(facing_dir.right().delta())),
        };

        let precondition = PreconditionParameters {
            organic_energy,
            charge_energy,
            obstacles,
            cell_energy_has_increased: true, // TODO: track this properly
            rng_value: rng.random(),
        };

        let action = genome.execute(&mut genome_id, &precondition);

        match *cell {
            Cell::Sprout => match action.multi_cell_command {
                MultiCellCommand::SkipTurn => {
                    *genome_id = action.multi_cell_success_next_genome;
                }
                MultiCellCommand::BecomeASeed => {
                    *genome_id = action.multi_cell_success_next_genome;
                    *cell = Cell::Seed;
                }
                MultiCellCommand::BecomeADetachedSeed { is_stationary } => {
                    *genome_id = action.multi_cell_success_next_genome;
                    *cell = Cell::Seed;
                }
                MultiCellCommand::Die => todo!(),
                MultiCellCommand::SeparateFromOrganism => todo!(),
                MultiCellCommand::TransportSoilEnergy(relative_direction) => todo!(),
                MultiCellCommand::TransportSoilOrganicMatter(relative_direction) => {
                    todo!()
                }
                MultiCellCommand::ShootSeed { high_energy } => todo!(),
                MultiCellCommand::DistributeEnergyAsOrganicMatter => todo!(),
            },
            Cell::Seed => match action.single_cell_command {
                SingleCellCommand::MoveForward => todo!(),
                SingleCellCommand::TurnLeft => todo!(),
                SingleCellCommand::TurnRight => todo!(),
                SingleCellCommand::TurnAround => todo!(),
                SingleCellCommand::TurnLeftAndMove => todo!(),
                SingleCellCommand::TurnRightAndMove => todo!(),
                SingleCellCommand::TurnAroundAndMove => todo!(),
                SingleCellCommand::TurnRandom => todo!(),
                SingleCellCommand::MoveRandom => todo!(),
                SingleCellCommand::Parasitise => todo!(),
                SingleCellCommand::PullOrganicFromLeft => todo!(),
                SingleCellCommand::PullOrganicFromRight => todo!(),
                SingleCellCommand::PullOrganicFromForward => todo!(),
                SingleCellCommand::PullChargeFromLeft => todo!(),
                SingleCellCommand::PullChargeFromRight => todo!(),
                SingleCellCommand::PullChargeFromForward => todo!(),
                SingleCellCommand::ConsumeNeighbours => todo!(),
                SingleCellCommand::TakeEnergyFromSoil => todo!(),
            },
            _ => unreachable!("Only Sprout and Seed cells should have genomes"),
        }
    }
}

// fn execute_sprout_genome_system(
