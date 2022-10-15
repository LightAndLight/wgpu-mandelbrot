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

struct IterationCount{escaped : u32, value : u32}

@group(0) @binding(0) var<uniform> screen_size : vec2<u32>;
@group(0) @binding(1) var<storage, read> iteration_counts : array<IterationCount>;
@group(0) @binding(2) var<uniform> iteration_limit : u32;

fn iteration_count_color(iteration_count : IterationCount) -> vec4<f32> {
  if iteration_count.escaped == 1u {
    let iteration_limit = f32(iteration_limit);
    let iteration_count = f32(iteration_count.value);

    return vec4<f32>(
      //    2*(x - 0.5)^2 + 0.5
      1.0 - 2.0 * pow((iteration_count / iteration_limit - 0.5), 2.0) + 0.5,
      iteration_count / iteration_limit,
      0.0, // iteration_count / iteration_limit,
      1.0
    );
  } else {
    return vec4<f32>(0.0, 0.0, 0.0, 0.0);
  }
}

// builtins are documented here: https://www.w3.org/TR/WGSL/#builtin-values
@fragment
fn fragment_main(@builtin(position) position : vec4<f32>) -> @location(0) vec4<f32> {
  // TODO: why is position not coming throught as NDC?
  let x = u32(position.x);
  let y = u32(position.y);

  return iteration_count_color(iteration_counts[y * screen_size.x + x]);
}