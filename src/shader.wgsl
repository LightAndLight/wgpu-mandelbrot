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

struct ColourRange{escaped : u32, value : f32}

@group(0) @binding(0) var<uniform> screen_size : vec2<u32>;

@group(1) @binding(0) var<storage, read> colour_ranges : array<ColourRange>;
  
let GAMMA = vec3<f32>(2.2, 2.2, 2.2);
let EXPONENT = vec3<f32>(2.0, 2.0, 2.0);

fn compute_colour(colour_range : ColourRange) -> vec4<f32> {
  let unescaped = pow(vec3<f32>(0.0, 0.0, 0.0), GAMMA);
  let initial_colour = pow(vec3<f32>(15.0 / 255.0, 66.0 / 255.0, 7.0 / 255.0), GAMMA);
  let final_colour = pow(vec3<f32>(1.0, 1.0, 1.0), GAMMA);
  
  let scale = vec3<f32>(colour_range.value, colour_range.value, colour_range.value);
  
  if colour_range.escaped == 1u {
    return vec4<f32>(
      initial_colour + (final_colour - initial_colour) * pow(scale, EXPONENT),
      1.0
    );
  } else {
    return vec4<f32>(unescaped, 1.0);
  }
}

// builtins are documented here: https://www.w3.org/TR/WGSL/#builtin-values
@fragment
fn fragment_main(@builtin(position) position : vec4<f32>) -> @location(0) vec4<f32> {
  // TODO: why is position not coming throught as NDC?
  let x = u32(position.x);
  let y = u32(position.y);

  return compute_colour(colour_ranges[y * screen_size.x + x]);
}