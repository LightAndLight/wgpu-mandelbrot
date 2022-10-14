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
@group(0) @binding(0) var results: texture_storage_2d<rgba8unorm, write>;

let ESCAPE_THRESHOLD: f32 = 2.0;
let ITERATION_LIMIT: u32 = 80u;

struct Complex{real: f32, imaginary: f32}

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

let ZERO_COMPLEX: Complex = Complex(0.0, 0.0);

fn iteration_count_color(iteration_count: u32) -> vec4<f32> {
  let iteration_limit = f32(ITERATION_LIMIT);
  let iteration_count = f32(iteration_count);
  
  return vec4<f32>(
    //    2*(x - 0.5)^2 + 0.5
    1.0 - 2.0 * pow((iteration_count / iteration_limit - 0.5), 2.0) + 0.5,
    iteration_count / iteration_limit,
    0.0, // iteration_count / iteration_limit,
    1.0
  );
}

// View the set from `(-(2 / ZOOM), -(2 / ZOOM))` to `(2 / ZOOM, 2 / ZOOM)`
let ZOOM = 1.0;

// Center the image on `ORIGIN`,
let ORIGIN = vec2<f32>(0.42, 0.22);

@group(0) @binding(1) var<uniform> screen_size : vec2<f32>;

@compute @workgroup_size(64)
fn mandelbrot(@builtin(global_invocation_id) global_invocation_id: vec3<u32>) {
  var z = ZERO_COMPLEX;
  let zoom_inv = 2.0 / ZOOM;
  var c = Complex(
    2.0 * zoom_inv * f32(global_invocation_id.x) / screen_size.x - zoom_inv + ORIGIN.x,  
    2.0 * zoom_inv * f32(global_invocation_id.y) / screen_size.y - zoom_inv + ORIGIN.y
  );

  var iteration_count: u32 = 0u;

  var hit_iteration_limit = false;
  loop {
    if length_complex(z) >= ESCAPE_THRESHOLD {
      break;
    }
    
    if iteration_count >= ITERATION_LIMIT {
      hit_iteration_limit = true;
      break;
    }

    z = add_complex(multiply_complex(z, z), c);
    iteration_count++;
  }

  // doesn't work
  // let coords: vec2<u32> = vec2<u32>(global_invocation_id.x, global_invocation_id.y);
  
  let coords: vec2<i32> = vec2<i32>(i32(global_invocation_id.x), i32(global_invocation_id.y));
  
  var value: vec4<f32>;
  if hit_iteration_limit {
    value = vec4<f32>(0.0, 0.0, 0.0, 1.0); 
  } else {
    value = iteration_count_color(iteration_count);
  };
  
  textureStore(results, coords, value)
}