use std::collections::HashSet;

use bevy::{
    ecs::system::Res,
    prelude::{
        Assets, ColorMaterial, Commands, Entity, EntityCommands, Mesh, MeshMaterial2d, Quat, Query,
        ResMut, Single, Transform, Vec3, With, Without, default, info,
    },
};
use bevy_rand::{global::GlobalRng, prelude::WyRand};
use rand::RngExt;

use crate::{
    GridPosition, TILE_SIZE,
    cells::{
        Cell, CellEnergy, CellRenderBundle, CellVisualSpec, Direction, FacingDirection,
        GenomeActionable, Mesh2d, SeedCell,
    },
    energy::{ChargeEnergyEnvironment, NeighbouringEnergy, OrganicEnergyEnvironment},
    genes::{
        Genome, GenomeID, MultiCellCommand, ObstacleInfo, PreconditionParameters, SingleCellCommand,
    },
    input::{observe_cell_hover, observe_cell_out},
    utils::grid_pos_to_world_pos,
};

/// Spawns a new cell entity with the specified components.
pub fn spawn_cell(
    commands: &mut Commands,
    cell: Cell,
    grid_pos: GridPosition,
    facing: FacingDirection,
    energy: CellEnergy,
    genome: Genome,
    genome_id: GenomeID,
) {
    info!(
        "Spawning cell at ({}, {}) of type {:?}",
        grid_pos.x, grid_pos.y, cell,
    );

    let mut entity_commands = commands.spawn((grid_pos, facing, cell, energy, genome, genome_id));

    if matches!(cell, Cell::Sprout | Cell::Seed(_)) {
        entity_commands.insert(GenomeActionable);
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
pub fn draw_cells_system(
    mut commands: Commands,
    cells: Query<(Entity, &GridPosition, &FacingDirection, &Cell), Without<Mesh2d>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for (entity, grid_pos, facing_direction, cell) in &cells {
        let transform = cell_transform(grid_pos, facing_direction.0);
        let spec = cell.visual_spec();

        info!(
            "Spawning cell at ({}, {}) of type {:?}",
            grid_pos.x, grid_pos.y, cell,
        );

        let mut entity_commands = commands.entity(entity);
        insert_cell_visual(
            &mut entity_commands,
            spec,
            transform,
            *grid_pos,
            &mut meshes,
            &mut materials,
        );

        entity_commands
            .observe(observe_cell_hover)
            .observe(observe_cell_out);
    }
}

/// System to invoke genome actions for cells that have genomes (Sprout and Seed cells).
pub fn invoke_cell_genome_actions_system(
    commands: Commands,
    mut rng: Single<&mut WyRand, With<GlobalRng>>,
    cell_positions: Query<&GridPosition, With<Cell>>,
    mut cells: Query<
        (
            &GridPosition,
            &FacingDirection,
            &mut Cell,
            &Genome,
            &mut GenomeID,
        ),
        With<GenomeActionable>,
    >,
    organic_energy_env: Res<OrganicEnergyEnvironment>,
    charge_energy_env: Res<ChargeEnergyEnvironment>,
) {
    let cell_positions: HashSet<GridPosition> = cell_positions.iter().cloned().collect();
    for (grid_pos, facing_dir, mut cell, genome, mut genome_id) in cells.iter_mut() {
        let organic_energy = NeighbouringEnergy::new(grid_pos, facing_dir, &*organic_energy_env);
        let charge_energy = NeighbouringEnergy::new(grid_pos, facing_dir, &*charge_energy_env);

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
                    *cell = Cell::Seed(SeedCell::DormantSeed);
                }
                MultiCellCommand::BecomeADetachedSeed { is_stationary } => {
                    *genome_id = action.multi_cell_success_next_genome;
                    *cell = Cell::Seed(SeedCell::DetachedSeed { is_stationary });
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
            Cell::Seed(seed_type) => match action.single_cell_command {
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
