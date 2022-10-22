/// Workgroup size for `compute.wsgl#mandelbrot`.
pub const MANDELBROT_WORKGROUP_SIZE_Y: u32 = 64;

/// Corresponds to `compute.wsgl#MANDELBROT_DISPATCH_SIZE_Y`.
pub const MANDELBROT_DISPATCH_SIZE_Y: u32 = 1024;

/**
Dispatch size for `compute.wgsl#mandelbrot`

[WGSL compute shader workgroups reference](https://www.w3.org/TR/WGSL/#compute-shader-workgroups)

For a single workgroup, a compute entrypoint with `@workgroup_size(w_x, w_y, w_z)`,
is run `w_x * w_y * w_z` times. A call to `dispatch_workgroups(x, y, z)` runs `x * y * z`
workgroups, which would run the compute entrypoint `x * y * z * w_x * w_y * w_z` times.

I want to call `compute.wgsl#mandelbrot` a specific number of times; once for
each pixel that hasn't yet escaped. Call this number `total_work`. I can then
use the `global_invocation_id` to index into the pixel data buffer to process
each pixel, potentially in parallel.

To simply things I started with `@workgroup_size(1, 1, 1)`, which seems to roughly
correspond to 1 invocation per streaming multiprocessor[^stackoverflow-workgroups].
Then used a dispatch size of `(total_work, 1, 1)`. The pixel data index would
be `global_invocation_id.x`.

When I did this, I exceeded the
[maxComputeWorkgroupsPerDimension](https://www.w3.org/TR/webgpu/#dom-supported-limits-maxcomputeworkgroupsperdimension)
limit of 65535 (2^16 - 1). When `total_work` exeeds this limit, I have
use more than one dimension to dispatch the correct number of workgroups.

Using two dimensions allows me to dispatch up to `65535^2` (approx. 2^32)
workgroups. With one workgroup per pixel on my 4k screen, I'd need
`3840 * 2160 = 8294400` workgroups, which is 0.1% of `65535^2`.

I can divide `total_work` into chunks of 65535. When `total_work <= 65535`,
the dispatch size should be `(1, 65535, 1)`. When `65535 < total_work <= 2 * 65535`,
the dispatch size should be `(2, 65535, 1)`. For `2 * 65535 < total_work <= 3 * 65535`
it should be `(3, 65535, 1)`. And so on.

A formula for this is `(total_work / 65535 + 1, 65535, 1)`. It dispatches
up to 65535 redundant workgroups - at its maximum when 65535 evenly divides
`total_work`. That's okay for now.

The pixel data index's formula now looks like this:
`global_invocation_id.x * 65535 + global_invocation_id.y`.

With `@workgroup_size(1, 1, 1)`, the graphics card might only run a single invocation
per streaming multiprocessor (SM). There's more opportunity for parallelism. My
Nvidia RTX 2070 apparently has 2304 threads across 36 SMs, which works out to
64 threads per SM[^geforce-20].
I want to aim for 64 invocations per workgroup to increase my
chances of parallelism.

I get 64 invocations per workgroup by setting `@workgroup_size(1, 64, 1)`. This
increases the number of invocations by a factor of 64. A dispatch size `(x, y, z)`
with workgroup size `(1, 64, 1)` is generates the same [compute shader grid](https://www.w3.org/TR/WGSL/#compute-shader-grid)
as a dispatch size of `(x, 64 * y, z)` with a workgroup size of `(1, 1, 1)`.

With a workgroup size of `(1, 64, 1)`, I need to scale down the `y` dimension of the
dispatch size to keep the same compute shader grid: `(total_work / 65535 + 1, 65535 / 64, 1)`.

There's a problem: 64 doesn't evenly divide 65535. But we have `65536 / 64 = 1024`. Let's
use 1024 for the dispatch `y` size. Which means we need to divide `total_work` into chunks
of `1024 (dispatch y size) * 64 (workgroup y size) = 65536`. This gives `(total_work / (1024 * 64) + 1, 1024, 1)`.
This means the correct pixel index formula is `global_invocation_id.x * (1024 * 64) + global_invocation_id.y`.

[^stackoverflow-workgroups]: <https://stackoverflow.com/questions/34638336/calculating-the-right-number-of-workgroups-and-their-size-opencl>

[^geforce-20]: <https://en.wikipedia.org/wiki/GeForce_20_series#GeForce_20_(20xx)_series_for_desktops>
*/
pub fn mandelbrot_dispatch_size(total_work: usize) -> (u32, u32, u32) {
    let x = (total_work / (MANDELBROT_DISPATCH_SIZE_Y * MANDELBROT_WORKGROUP_SIZE_Y) as usize + 1)
        .try_into()
        .unwrap();
    (x, MANDELBROT_DISPATCH_SIZE_Y, 1)
}
