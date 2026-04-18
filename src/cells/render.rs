use bevy::{
    app::Plugin,
    log::info,
    prelude::{
        Assets, Bundle, Children, Circle, Color, ColorMaterial, Commands, ContainsEntity, Deref,
        Ellipse, Entity, EntityCommands, EntityEvent, Mesh, Mesh2d, MeshMaterial2d, On, Quat,
        Query, Rectangle, ResMut, Transform, Vec3, default,
    },
};

use crate::{
    GridPosition, TILE_SIZE,
    cells::{Cell, Direction, FacingDirection},
    utils::grid_pos_to_world_pos,
};

const CELL_GREEN: Color = Color::linear_rgb(23.0 / 255.0, 185.0 / 255.0, 0.0 / 255.0);
const CELL_ORANGE: Color = Color::linear_rgb(235.0 / 255.0, 138.0 / 255.0, 64.0 / 255.0);
const CELL_BLUE: Color = Color::linear_rgb(82.0 / 255.0, 107.0 / 255.0, 1.0);
const CELL_BROWN: Color = Color::linear_rgb(30.0 / 255.0, 20.0 / 255.0, 10.0 / 255.0);

const LEAF_VISUAL_SPEC: CellVisualSpec = CellVisualSpec {
    shape: ShapeSpec::Ellipse {
        half_width: TILE_SIZE / 1.75,
        half_height: TILE_SIZE / 3.0,
    },
    color: CELL_GREEN,
    children: &[],
};

const ANTENNA_VISUAL_SPEC: CellVisualSpec = CellVisualSpec {
    shape: ShapeSpec::Circle(TILE_SIZE / 3.0),
    color: CELL_BLUE,
    children: &[],
};

const ROOT_VISUAL_SPEC: CellVisualSpec = CellVisualSpec {
    shape: ShapeSpec::Rect {
        width: TILE_SIZE / 1.5,
        height: TILE_SIZE / 1.5,
    },
    color: CELL_ORANGE,
    children: &[],
};

const SPROUT_VISUAL_SPEC: CellVisualSpec = CellVisualSpec {
    shape: ShapeSpec::Circle(TILE_SIZE / 3.0),
    color: Color::WHITE,
    children: &[
        &ChildVisualSpec {
            shape: ShapeSpec::Circle(TILE_SIZE / 15.0),
            color: Color::BLACK,
            transform: Transform::from_translation(Vec3::new(
                TILE_SIZE / 6.0,
                TILE_SIZE / 6.0,
                2.0,
            )),
        },
        &ChildVisualSpec {
            shape: ShapeSpec::Circle(TILE_SIZE / 15.0),
            color: Color::BLACK,
            transform: Transform::from_translation(Vec3::new(
                TILE_SIZE / 6.0,
                -TILE_SIZE / 6.0,
                2.0,
            )),
        },
    ],
};

const BRANCH_VISUAL_SPEC: CellVisualSpec = CellVisualSpec {
    shape: ShapeSpec::Rect {
        width: TILE_SIZE * 1.5,
        height: TILE_SIZE / 6.0,
    },
    color: CELL_BROWN,
    children: &[],
};

const SEED_VISUAL_SPEC: CellVisualSpec = CellVisualSpec {
    shape: ShapeSpec::Circle(TILE_SIZE / 6.0),
    color: Color::WHITE,
    children: &[],
};

#[derive(Clone, Copy, Debug)]
pub struct CellRenderPlugin;

impl Plugin for CellRenderPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_observer(draw_new_cells_system);
    }
}

/// Event to trigger drawing a cell. Contains the entity of the cell to be drawn.
#[derive(EntityEvent, Debug, Clone, Deref)]
pub struct DrawCellEvent(pub Entity);

/// Bundle for rendering a cell.
#[derive(Bundle)]
pub struct CellRenderBundle {
    mesh: Mesh2d,
    material: MeshMaterial2d<ColorMaterial>,
    transform: Transform,
}

#[derive(Clone, Copy, Debug)]
enum ShapeSpec {
    Circle(f32),
    Ellipse { half_width: f32, half_height: f32 },
    Rect { width: f32, height: f32 },
}

impl ShapeSpec {
    pub fn into_mesh(self, meshes: &mut Assets<Mesh>) -> Mesh2d {
        let handle = match self {
            ShapeSpec::Circle(r) => meshes.add(Circle::new(r)),
            ShapeSpec::Ellipse {
                half_width,
                half_height,
            } => meshes.add(Ellipse::new(half_width, half_height)),
            ShapeSpec::Rect { width, height } => meshes.add(Rectangle::new(width, height)),
        };

        Mesh2d(handle)
    }
}

#[derive(Clone, Copy, Debug)]
struct ChildVisualSpec {
    shape: ShapeSpec,
    color: Color,
    transform: Transform,
}

/// Visual specification for a cell, including its shape, color, and any child
/// visuals (e.g., for details like eyes).
#[derive(Clone, Debug)]
struct CellVisualSpec {
    shape: ShapeSpec,
    color: Color,
    children: &'static [&'static ChildVisualSpec],
}

impl Cell {
    fn visual_spec(&self) -> CellVisualSpec {
        match self {
            Cell::Leaf => LEAF_VISUAL_SPEC,
            Cell::Antenna => ANTENNA_VISUAL_SPEC,
            Cell::Root => ROOT_VISUAL_SPEC,
            Cell::Sprout => SPROUT_VISUAL_SPEC,
            Cell::Branch => BRANCH_VISUAL_SPEC,
            Cell::Seed => SEED_VISUAL_SPEC,
        }
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
fn get_transform_with_rotation(grid_pos: &GridPosition, facing: Direction) -> Transform {
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

/// System to (re)create visual entities for cells. Despawns any existing child
/// visuals first so stale sprites don't linger when the cell's type changes.
pub fn draw_new_cells_system(
    event: On<DrawCellEvent>,
    mut commands: Commands,
    cells: Query<(Entity, &Cell, &GridPosition, &FacingDirection)>,
    existing_children: Query<&Children>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let Ok((entity, cell, grid_pos, facing_direction)) = cells.get(event.entity()) else {
        return;
    };

    info!(
        "Drawing cell at ({}, {}) of type {:?}",
        grid_pos.x, grid_pos.y, cell,
    );

    if let Ok(children) = existing_children.get(entity) {
        for &child in children.iter() {
            commands.entity(child).despawn();
        }
    }

    let transform = get_transform_with_rotation(grid_pos, **facing_direction);
    let spec = cell.visual_spec();

    let mut entity_commands = commands.entity(entity);
    insert_cell_visual(
        &mut entity_commands,
        spec,
        transform,
        *grid_pos,
        &mut meshes,
        &mut materials,
    );
}
