use bevy::math::Vec3;

use crate::{GridPosition, TILE_SIZE};

/// Converts a grid position to a world position for rendering.
#[inline]
pub const fn grid_pos_to_world_pos(grid_pos: &GridPosition) -> Vec3 {
    let world_x = grid_pos.x as f32 * TILE_SIZE;
    let world_y = grid_pos.y as f32 * TILE_SIZE;

    Vec3::new(world_x, world_y, 1.0)
}
