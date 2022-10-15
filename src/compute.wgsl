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

@group(0) @binding(0) var<storage, read_write> iteration_counts: array<IterationCount>;
@group(0) @binding(1) var<uniform> screen_size : vec2<u32>;

// View the set from `(-(2 / zoom), -(2 / zoom))` to `(2 / zoom, 2 / zoom)`
@group(0) @binding(2) var<uniform> zoom : f32;

// Center the image on `origin`,
@group(0) @binding(3) var<uniform> origin : vec2<f32>;

@group(0) @binding(4) var<uniform> iteration_limit : u32;
@group(0) @binding(5) var<storage, read_write> starting_values : array<Complex>;

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
  var iteration_result : Complex = starting_values[index];
  var iteration_count: u32 = 0u;

  var escaped = false;
  loop {
    if length_complex(iteration_result) >= ESCAPE_THRESHOLD {
      escaped = true;
      break;
    }

    if iteration_count >= iteration_limit {
      break;
    }

    iteration_result = add_complex(multiply_complex(iteration_result, iteration_result), c);
    iteration_count++;
  }

  starting_values[index] = iteration_result;
  if escaped {
    iteration_counts[index] = IterationCount(1u, iteration_counts[index].value + iteration_count);
  } else {
    iteration_counts[index] = IterationCount(0u, iteration_counts[index].value + iteration_count);
  }
}