use bevy::{
    asset::Asset,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d, Material2dPlugin},
};

use crate::{GridVisible, TILE_SIZE};

const QUAD_SIZE: f32 = 1_000_000.0;
const GRID_Z: f32 = -1.0;

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Material2dPlugin::<GridMaterial>::default())
            .add_systems(Startup, spawn_grid)
            .add_systems(Update, toggle_grid);
    }
}

#[derive(Component)]
struct GridQuad;

#[derive(ShaderType, Clone, Debug)]
struct GridParams {
    color: Vec4,
    tile_size: f32,
    major_every: f32,
    line_thickness_px: f32,
    minor_alpha: f32,
    major_alpha: f32,
    fade_min_px: f32,
    fade_max_px: f32,
}

#[derive(Asset, AsBindGroup, TypePath, Clone, Debug)]
pub struct GridMaterial {
    #[uniform(0)]
    params: GridParams,
}

impl Material2d for GridMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/grid.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

fn spawn_grid(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GridMaterial>>,
) {
    let mesh = meshes.add(Rectangle::new(QUAD_SIZE, QUAD_SIZE));
    let material = materials.add(GridMaterial {
        params: GridParams {
            color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            tile_size: TILE_SIZE,
            major_every: 10.0,
            line_thickness_px: 1.0,
            minor_alpha: 0.15,
            major_alpha: 0.25,
            fade_min_px: 2.0,
            fade_max_px: 8.0,
        },
    });
    commands.spawn((
        Mesh2d(mesh),
        MeshMaterial2d(material),
        Transform::from_translation(Vec3::new(0.0, 0.0, GRID_Z)),
        GridQuad,
    ));
}

fn toggle_grid(
    grid_visible: Res<GridVisible>,
    mut query: Query<&mut Visibility, With<GridQuad>>,
) {
    if !grid_visible.is_changed() {
        return;
    }
    for mut vis in &mut query {
        *vis = if grid_visible.0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
