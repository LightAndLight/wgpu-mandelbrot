struct Complex{real: f32, imaginary: f32}

let ZERO_COMPLEX: Complex = Complex(0.0, 0.0);

fn multiply_complex(first: Complex, second: Complex) -> Complex {
  let a = first.real * second.real;
  let b = first.real * second.imaginary + first.imaginary * second.real;
  let c = first.imaginary * second.imaginary;

  return Complex(a - c, b);
}

fn add_complex(first: Complex, second: Complex) -> Complex {
  return Complex(first.real + second.real, first.imaginary + second.imaginary);
}

fn length_complex(value: Complex) -> f32 {
  return sqrt(pow(value.real, 2.0) + pow(value.imaginary, 2.0));
}

let ESCAPE_THRESHOLD: f32 = 2.0;

struct IterationCount{escaped : u32, value : u32}

/*
From [Wikipedia](https://en.wikipedia.org/wiki/Mandelbrot_set)

> The Mandelbrot set is the set of complex numbers `c` for which the function `f_c( z ) = z^2 + c`
> does not diverge to infinity when iterated from `z = 0`, i.e., for which the sequence `f_c(0)`,
> `f_c(f_c(0))`, etc., remains bounded in absolute value.

We treat each complex number `c = a + bi` as a pixel with coordinates `(a, b)`.

Each pixel's membership in the Mandelbrot set depends only on that pixel. This is a perfect
function to parallelise on the GPU.

According to the Wikipedia article, renderings of the Mandelbrot set colour each pixel
according to how quickly the pixel crosses a chosen threshold (the threshold must be >2).

We can run the iteration logic on a compute shader and store the resulting colours in a
2D texture the size of the screen. We then make the fragment shader trigger for each pixel
on the screen, and sample the results texture for its color.
*/

@group(0) @binding(0) var<uniform> screen_size : vec2<u32>;

// View the set from `(-(2 / zoom), -(2 / zoom))` to `(2 / zoom, 2 / zoom)`
@group(0) @binding(1) var<uniform> zoom : f32;

// Center the image on `origin`,
@group(0) @binding(2) var<uniform> origin : vec2<f32>;

@group(1) @binding(0) var<storage, read> starting_values_in : array<Complex>;
@group(1) @binding(1) var<storage, read_write> starting_values_out : array<Complex>;
@group(1) @binding(2) var<storage, read> iteration_counts_in : array<IterationCount>;
@group(1) @binding(3) var<storage, read_write> iteration_counts_out : array<IterationCount>;

@compute @workgroup_size(64)
fn mandelbrot(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
  let x = global_invocation_id.x;
  let y = global_invocation_id.y;

  let zoom_inv = 2.0 / zoom;

  var c = Complex(
    2.0 * zoom_inv * f32(x) / f32(screen_size.x) - zoom_inv + origin.x,
    2.0 * zoom_inv * f32(y) / f32(screen_size.y) - zoom_inv + origin.y
  );

  let index = y * screen_size.x + x;
  var starting_value : Complex = starting_values_in[index];

  /*
  conditions:

  length_complex(starting_value) > ESCAPE_THRESHOLD implies xx < 0
  length_complex(starting_value) == ESCAPE_THRESHOLD implies xx == 0
  length_complex(starting_value) < ESCAPE_THRESHOLD implies xx > 0
  
  length_complex(starting_value) >= ESCAPE_THRESHOLD implies xx <= 0
  */
  let escape_threshold_minus_length = ESCAPE_THRESHOLD - length_complex(starting_value);

  // length_complex(starting_value) >= ESCAPE_THRESHOLD implies max(xx, 0.0) == 0
  //
  // If `escape_threshold_minus_length` is negative, then `max` outputs `0.0`.
  let escape_threshold_minus_length_max_0 = max(escape_threshold_minus_length, 0.0);
    
  // escape_threshold_minus_length_max_0 == 0.0 should imply iteration_counts_out[index].escaped == 1u;
  // escape_threshold_minus_length_max_0 > 0.0 should imply iteration_counts_out[index].escaped == 0u;
  //
  // escape_threshold_minus_length_max_0 == 0.0 implies sign(escape_threshold_minus_length_max_0) == 0.0
  // escape_threshold_minus_length_max_0 > 0.0 implies sign(escape_threshold_minus_length_max_0) == 1.0
  //
  // escaped == u32(1.0 - sign(escape_threshold_minus_length_max_0))
  //
  // escape_threshold_minus_length_max_0 == 0.0 implies
  //   escaped == u32(1.0 - 0.0)
  //   escaped == u32(1.0)
  //   escaped == 1u
  //
  // escape_threshold_minus_length_max_0 > 0.0 implies
  //   escaped == u32(1.0 - 1.0)
  //   escaped == u32(0.0)
  //   escaped == 0u
  iteration_counts_out[index].escaped = u32(1.0 - sign(escape_threshold_minus_length_max_0));
  
  if escape_threshold_minus_length_max_0 == 0.0 {
    starting_values_out[index] = starting_value;
    iteration_counts_out[index].value = iteration_counts_in[index].value;
  } else {
    starting_values_out[index] = add_complex(multiply_complex(starting_value, starting_value), c);
    iteration_counts_out[index].value = iteration_counts_in[index].value + 1u;
  }
}