use std::collections::HashSet;

use bevy::{
    app::{App, Plugin, Update},
    ecs::{message::MessageReader, observer::On, system::Res},
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
        Cell, CellEnergy, CellRenderBundle, CellVisualSpec, Direction, FacingDirection, Mesh2d,
        NewCellEvent,
    },
    energy::{ChargeEnergyEnvironment, NeighbouringEnergy, OrganicEnergyEnvironment},
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
