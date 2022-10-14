@vertex
fn vertex_main(@builtin(vertex_index) index : u32) -> @builtin(position) vec4<f32> {
  var vertices = array<vec2<f32>, 4>(
    vec2<f32>(-1.0, -1.0),
    vec2<f32>(1.0, -1.0),
    vec2<f32>(-1.0, 1.0),
    vec2<f32>(1.0, 1.0)
  );
  
  return vec4<f32>(vertices[index], 0.0, 1.0);
}

@group(0) @binding(0) var colors : texture_2d<f32>;
@group(0) @binding(1) var colors_sampler : sampler;
@group(0) @binding(2) var<uniform> screen_size : vec2<f32>;

// builtins are documented here: https://www.w3.org/TR/WGSL/#builtin-values
@fragment
fn fragment_main(@builtin(position) position : vec4<f32>) -> @location(0) vec4<f32> {
  return textureSample(colors, colors_sampler, vec2<f32>(position.x / screen_size.x, position.y / screen_size.y));
}