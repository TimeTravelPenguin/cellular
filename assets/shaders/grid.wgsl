#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct GridParams {
    color: vec4<f32>,
    tile_size: f32,
    major_every: f32,
    line_thickness_px: f32,
    minor_alpha: f32,
    major_alpha: f32,
    fade_min_px: f32,
    fade_max_px: f32,
};

@group(2) @binding(0) var<uniform> params: GridParams;

// Returns an alpha value for a line centered at multiples of `period` 
// with the given thickness in pixels.
fn line_alpha(coord: f32, period: f32, thickness_px: f32) -> f32 {
    let pixel_size = fwidth(coord);
    let dist = abs(fract(coord / period - 0.5) - 0.5) * period;
    let half_thickness = thickness_px * 0.5 * pixel_size;
    return 1.0 - smoothstep(half_thickness, half_thickness + pixel_size, dist);
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let world = in.world_position.xy + vec2<f32>(params.tile_size * 0.5);
    let pixel_size = max(fwidth(world.x), fwidth(world.y));
    let tile_px = params.tile_size / pixel_size;

    let minor_x = line_alpha(world.x, params.tile_size, params.line_thickness_px);
    let minor_y = line_alpha(world.y, params.tile_size, params.line_thickness_px);
    let minor_mask = max(minor_x, minor_y);

    let major_period = params.tile_size * params.major_every;
    let major_x = line_alpha(world.x, major_period, params.line_thickness_px);
    let major_y = line_alpha(world.y, major_period, params.line_thickness_px);
    let major_mask = max(major_x, major_y);

    let fade = smoothstep(params.fade_min_px, params.fade_max_px, tile_px);
    let minor = minor_mask * params.minor_alpha * fade;
    let major = major_mask * params.major_alpha;
    let alpha = max(minor, major);

    return vec4<f32>(params.color.rgb, alpha * params.color.a);
}
