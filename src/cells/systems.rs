use bevy::prelude::{
    Assets, ColorMaterial, Commands, Entity, EntityCommands, Mesh, MeshMaterial2d, Quat, Query,
    ResMut, Single, Transform, Vec3, With, Without, default, info,
};
use bevy_rand::{global::GlobalRng, prelude::WyRand};

use crate::{
    GridPosition, TILE_SIZE,
    cells::{
        Cell, CellEnergy, CellRenderBundle, CellVisualSpec, Direction, FacingDirection, Mesh2d,
    },
    genes::{Genome, GenomeID},
    input::{observe_cell_hover, observe_cell_out},
};

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

    commands.spawn((grid_pos, facing, cell, energy, genome, genome_id));
}

fn facing_rotation(direction: Direction) -> Quat {
    match direction {
        Direction::East => Quat::IDENTITY,
        Direction::South => Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2),
        Direction::West => Quat::from_rotation_z(std::f32::consts::PI),
        Direction::North => Quat::from_rotation_z(std::f32::consts::FRAC_PI_2),
    }
}

#[inline]
const fn grid_pos_to_world_pos(grid_pos: &GridPosition) -> Vec3 {
    let world_x = grid_pos.x as f32 * TILE_SIZE;
    let world_y = grid_pos.y as f32 * TILE_SIZE;

    Vec3::new(world_x, world_y, 1.0)
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

pub fn invoke_cell_genome_actions_system(
    _commands: Commands,
    _rng: Single<&mut WyRand, With<GlobalRng>>,
    mut cells: Query<(&GridPosition, &Cell, &Genome, &mut GenomeID)>,
) {
    for (_grid_pos, _cell, _genome, _genome_id) in cells.iter_mut() {
        debug_assert!(
            matches!(_cell, Cell::Sprout | Cell::Seed(_)),
            "Only Sprout and Seed cells should have genomes"
        );
        // TODO: Implement
    }
}
