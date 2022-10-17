mod buffer;
mod command_buffer;
mod command_encoder;
mod double_buffered;
mod var;

use std::{
    collections::HashMap,
    sync::{Arc, Condvar, Mutex},
};

use bytemuck::{Pod, Zeroable};
use log::{debug, trace};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use command_encoder::CommandEncoderExt;
use double_buffered::DoubleBuffered;

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
struct IterationCount {
    escaped: u32,
    value: u32,
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
struct ColourRange {
    escaped: u32,
    value: f32,
}

impl Default for ColourRange {
    fn default() -> Self {
        Self {
            escaped: 0,
            value: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
struct ScreenSize {
    width: u32,
    height: u32,
}

fn create_iteration_counts_buffers(
    device: &wgpu::Device,
    size: ScreenSize,
) -> DoubleBuffered<IterationCount> {
    let initial_iteration_counts = std::iter::repeat(IterationCount {
        escaped: 0,
        value: 0,
    })
    .take((size.width * size.height) as usize)
    .collect::<Vec<_>>();

    DoubleBuffered {
        input: buffer::Builder::from_contents(&initial_iteration_counts)
            .with_label("iteration-counts-buffer-1")
            .with_usage(wgpu::BufferUsages::STORAGE)
            .with_usage(wgpu::BufferUsages::UNIFORM)
            .with_usage(wgpu::BufferUsages::COPY_SRC)
            .create(device),

        output: buffer::Builder::from_contents(&initial_iteration_counts)
            .with_label("iteration-counts-buffer-2")
            .with_usage(wgpu::BufferUsages::STORAGE)
            .with_usage(wgpu::BufferUsages::UNIFORM)
            .with_usage(wgpu::BufferUsages::COPY_SRC)
            .create(device),
    }
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy)]
struct Complex {
    real: f32,
    imaginary: f32,
}

impl Complex {
    const ZERO: Self = Complex {
        real: 0.0,
        imaginary: 0.0,
    };
}

fn create_starting_values_buffers(
    device: &wgpu::Device,
    size: ScreenSize,
) -> DoubleBuffered<Complex> {
    let initial_starting_values = std::iter::repeat(Complex::ZERO)
        .take((size.width * size.height) as usize)
        .collect::<Vec<_>>();

    DoubleBuffered {
        input: buffer::Builder::from_contents(&initial_starting_values)
            .with_label("starting-values-buffer-1")
            .with_usage(wgpu::BufferUsages::STORAGE)
            .create(device),

        output: buffer::Builder::from_contents(&initial_starting_values)
            .with_label("starting-values-buffer-2")
            .with_usage(wgpu::BufferUsages::STORAGE)
            .create(device),
    }
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
struct Vec2 {
    x: f32,
    y: f32,
}

fn compute_colour_ranges(iteration_counts: buffer::View<IterationCount>) -> Vec<ColourRange> {
    let iteration_counts = &*iteration_counts;
    let mut colour_ranges_out = Vec::with_capacity(iteration_counts.len());

    let mut samples: Vec<u32> = Vec::new();

    let mut min_iteration_count = u32::MAX;
    let mut max_iteration_count = 0;
    for iteration_count in iteration_counts {
        min_iteration_count = min_iteration_count.min(iteration_count.value);
        max_iteration_count = max_iteration_count.min(iteration_count.value);
        samples.push(iteration_count.value);
    }

    samples.sort();

    let mut histogram: HashMap<u32, f32> = HashMap::new();

    let total_samples = samples.len() as f32;
    let mut sample_count = None;
    let mut bucket_level: f32 = 0.0;
    for current_sample in samples {
        match sample_count {
            Some((previous_sample, previous_sample_count)) => {
                if current_sample == previous_sample {
                    sample_count = Some((previous_sample, previous_sample_count + 1));
                } else {
                    histogram.insert(previous_sample, bucket_level);
                    let bucket_value = previous_sample_count as f32 / total_samples;
                    bucket_level += bucket_value;

                    sample_count = Some((current_sample, 1));
                }
            }
            None => {
                sample_count = Some((current_sample, 1));
            }
        }
    }
    if let Some((previous_sample, previous_sample_count)) = sample_count {
        histogram.insert(
            previous_sample,
            previous_sample_count as f32 / total_samples,
        );
    }

    for iteration_count in iteration_counts.iter() {
        colour_ranges_out.push(ColourRange {
            escaped: iteration_count.escaped,
            value: histogram.get(&iteration_count.value).copied().unwrap(),
        });
    }

    colour_ranges_out
}

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    let instance = wgpu::Instance::new(wgpu::Backends::all());

    let size = window.inner_size();
    let surface = unsafe { instance.create_surface(&window) };

    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: Default::default(),
        force_fallback_adapter: false,
        compatible_surface: Some(&surface),
    }))
    .unwrap();

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("device"),
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::default(),
        },
        None,
    ))
    .unwrap();

    let mut surface_configuration = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface.get_supported_formats(&adapter)[0],
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
    };
    surface.configure(&device, &surface_configuration);

    let compute_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("compute-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("compute.wgsl").into()),
    });

    let compute_bind_group_layout_1 =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute-bind-group-layout-1"),
            entries: &[
                // compute.wgsl#screen_size
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // compute.wgsl#zoom
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // compute.wgsl#origin
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

    let compute_bind_group_layout_2 =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute-bind-group-layout-2"),
            entries: &[
                // compute.wgsl#starting_values_in
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // compute.wgsl#starting_values_out
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // compute.wgsl#iteration_counts_in
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // compute.wgsl#iteration_counts_out
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("compute-pipeline-layout"),
        bind_group_layouts: &[&compute_bind_group_layout_1, &compute_bind_group_layout_2],
        push_constant_ranges: &[],
    });

    let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("compute-pipeline"),
        layout: Some(&compute_pipeline_layout),
        module: &compute_shader_module,
        entry_point: "mandelbrot",
    });

    let render_shader_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("render-shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
    });

    let render_bind_group_layout_1 =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render-bind-group-layout"),
            entries: &[
                // shader.wgsl#screen_size
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

    let render_bind_group_layout_2 =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render-bind-group-layout-2"),
            entries: &[
                // shader.wgsl#colour_ranges
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("render-pipeline-layout"),
        bind_group_layouts: &[&render_bind_group_layout_1, &render_bind_group_layout_2],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("render-pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &render_shader_module,
            entry_point: "vertex_main",
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &render_shader_module,
            entry_point: "fragment_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_configuration.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview: None,
    });

    let mut screen_size = ScreenSize {
        width: size.width as u32,
        height: size.height as u32,
    };
    let screen_size_buffer = var::Builder::new(screen_size)
        .with_label("screen-size-buffer")
        .with_usage(wgpu::BufferUsages::UNIFORM)
        .create(&device);

    let mut zoom: f32 = 1.0;
    let zoom_buffer = var::Builder::new(zoom)
        .with_label("zoom-buffer")
        .with_usage(wgpu::BufferUsages::UNIFORM)
        .create(&device);

    let mut origin: Vec2 = Vec2 {
        x: -0.74529,
        y: 0.113075,
    };
    let origin_buffer = var::Builder::new(origin)
        .with_label("origin-buffer")
        .with_usage(wgpu::BufferUsages::UNIFORM)
        .create(&device);

    let mut iteration_counts_buffers = create_iteration_counts_buffers(&device, screen_size);
    let mut iteration_counts_staging_buffer =
        buffer::Builder::new((screen_size.width * screen_size.height) as u64)
            .with_label("iteration_counts_staging_buffer")
            .with_usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ)
            .create(&device);

    let mut starting_values_buffers = create_starting_values_buffers(&device, screen_size);

    let mut compute_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("compute-bind-group-1"),
        layout: &compute_bind_group_layout_1,
        entries: &[
            // compute.wgsl#screen_size
            wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_size_buffer.binding_resource(),
            },
            // compute.wgsl#zoom
            wgpu::BindGroupEntry {
                binding: 1,
                resource: zoom_buffer.binding_resource(),
            },
            // compute.wgsl#origin
            wgpu::BindGroupEntry {
                binding: 2,
                resource: origin_buffer.binding_resource(),
            },
        ],
    });

    let mut render_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("render-bind-group"),
        layout: &render_pipeline.get_bind_group_layout(0),
        entries: &[
            // shader.wgsl#screen_size
            wgpu::BindGroupEntry {
                binding: 0,
                resource: screen_size_buffer.binding_resource(),
            },
        ],
    });

    let mut cursor_position = Vec2 { x: 0.0, y: 0.0 };
    let mut zoom_changed = false;
    let mut origin_changed = false;

    let mut colour_ranges_buffer: buffer::Buffer<ColourRange> = buffer::Builder::from_contents(
        &std::iter::repeat(ColourRange::default())
            .take((screen_size.width * screen_size.height) as usize)
            .collect::<Vec<_>>(),
    )
    .with_usage(wgpu::BufferUsages::STORAGE)
    .create(&device);

    event_loop.run(move |event, _, control_flow| {
        // To present frames in realtime, *don't* set `control_flow` to `Wait`.
        // control_flow.set_wait();
        match event {
            Event::MainEventsCleared => {
                // And `request_redraw` once we've cleared all events for the frame.
                window.request_redraw();
            }
            Event::WindowEvent { window_id, event } if window_id == window.id() => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::CursorMoved { position, .. } => {
                    cursor_position.x = position.x as f32;
                    cursor_position.y = position.y as f32;
                }
                WindowEvent::MouseInput {
                    state: winit::event::ElementState::Pressed,
                    button: winit::event::MouseButton::Left,
                    ..
                } => {
                    debug!("mouse pressed at {:?}", cursor_position);

                    /*
                    when `zoom = 1.0`, we're viewing (-2, -2) to (2, 2).

                    (0, 0) corresponds to (size.width / 2, size.height / 2)

                    A click at (cursor_x, cursor_y) corresponds to (4 * cursor_x / size.width - 2, 4 * cursor_y / size.height - 2)
                     */

                    let zoom_inv = 2.0 / zoom;
                    origin = Vec2 {
                        x: origin.x
                            + (2.0 * zoom_inv * cursor_position.x / (size.width as f32) - zoom_inv),
                        y: origin.y
                            + (2.0 * zoom_inv * cursor_position.y / (size.height as f32)
                                - zoom_inv),
                    };
                    debug!("origin set to {:?}", origin);
                    origin_changed = true;
                    origin_buffer.write(&queue, origin);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    zoom += zoom
                        * 0.1
                        * match delta {
                            winit::event::MouseScrollDelta::LineDelta(_, delta) => delta,
                            winit::event::MouseScrollDelta::PixelDelta(_) => {
                                panic!("expected LineDelta, got PixelDelta")
                            }
                        };
                    zoom_changed = true;
                    zoom_buffer.write(&queue, zoom);
                }
                WindowEvent::Resized(size) => {
                    debug!("resizing to {:?}", size);

                    surface_configuration.width = size.width;
                    surface_configuration.height = size.height;

                    surface.configure(&device, &surface_configuration);

                    screen_size = ScreenSize {
                        width: size.width as u32,
                        height: size.height as u32,
                    };
                    screen_size_buffer.write(&queue, screen_size);

                    std::mem::replace(
                        &mut iteration_counts_buffers,
                        create_iteration_counts_buffers(&device, screen_size),
                    )
                    .destroy();
                    iteration_counts_staging_buffer =
                        buffer::Builder::new((screen_size.width * screen_size.height) as u64)
                            .with_label("iteration_counts_staging_buffer")
                            .with_usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ)
                            .create(&device);

                    std::mem::replace(
                        &mut starting_values_buffers,
                        create_starting_values_buffers(&device, screen_size),
                    )
                    .destroy();

                    std::mem::replace(
                        &mut colour_ranges_buffer,
                        buffer::Builder::from_contents(
                            &std::iter::repeat(ColourRange::default())
                                .take((screen_size.width * screen_size.height) as usize)
                                .collect::<Vec<_>>(),
                        )
                        .with_usage(wgpu::BufferUsages::STORAGE)
                        .create(&device),
                    )
                    .destroy();

                    compute_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("compute-bind-group"),
                        layout: &compute_bind_group_layout_1,
                        entries: &[
                            // compute.wgsl#screen_size
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: screen_size_buffer.binding_resource(),
                            },
                            // compute.wgsl#zoom
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: zoom_buffer.binding_resource(),
                            },
                            // compute.wgsl#origin
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: origin_buffer.binding_resource(),
                            },
                        ],
                    });

                    render_bind_group_1 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("render-bind-group"),
                        layout: &render_pipeline.get_bind_group_layout(0),
                        entries: &[
                            // shader.wgsl#screen_size
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: screen_size_buffer.binding_resource(),
                            },
                        ],
                    });

                    window.request_redraw();
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let reset_buffers = zoom_changed || origin_changed;
                zoom_changed = false;
                origin_changed = false;

                if reset_buffers {
                    let initial_starting_values = std::iter::repeat(Complex::ZERO)
                        .take((size.width * size.height) as usize)
                        .collect::<Vec<_>>();
                    starting_values_buffers
                        .input
                        .write(&queue, &initial_starting_values);
                    starting_values_buffers
                        .output
                        .write(&queue, &initial_starting_values);

                    let initial_iteration_counts = std::iter::repeat(IterationCount {
                        escaped: 0,
                        value: 0,
                    })
                    .take((size.width * size.height) as usize)
                    .collect::<Vec<_>>();
                    iteration_counts_buffers
                        .input
                        .write(&queue, &initial_iteration_counts);
                    iteration_counts_buffers
                        .output
                        .write(&queue, &initial_iteration_counts);
                }

                let surface_texture = surface.get_current_texture().unwrap();

                let surface_texture_view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let compute_bind_group_2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("compute-bind-group-2"),
                    layout: &compute_bind_group_layout_2,
                    entries: &[
                        // compute.wgsl#starting_values_in
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: starting_values_buffers.input.binding_resource(0, None),
                        },
                        // compute.wgsl#starting_values_out
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: starting_values_buffers.output.binding_resource(0, None),
                        },
                        // compute.wgsl#iteration_counts_in
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: iteration_counts_buffers.input.binding_resource(0, None),
                        },
                        // compute.wgsl#iteration_counts_out
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: iteration_counts_buffers.output.binding_resource(0, None),
                        },
                    ],
                });

                let render_bind_group_2 = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("render-bind-group-2"),
                    layout: &render_pipeline.get_bind_group_layout(1),
                    entries: &[
                        // shader.wgsl#colour_ranges
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: colour_ranges_buffer.binding_resource(0, None),
                        },
                    ],
                });

                let compute_command_buffer = command_buffer::create(
                    &device,
                    &wgpu::CommandEncoderDescriptor::default(),
                    |command_encoder| {
                        command_encoder.push_debug_group("compute-pass");
                        command_encoder.with_compute_pass(
                            &wgpu::ComputePassDescriptor {
                                label: Some("compute-pass"),
                            },
                            |compute_pass| {
                                compute_pass.set_pipeline(&compute_pipeline);

                                compute_pass.set_bind_group(0, &compute_bind_group_1, &[]);
                                compute_pass.set_bind_group(1, &compute_bind_group_2, &[]);

                                compute_pass.insert_debug_marker("mandelbrot");
                                compute_pass.dispatch_workgroups(size.width, size.height, 1);
                            },
                        );
                        command_encoder.pop_debug_group();

                        command_encoder.copy_buffer_to_buffer(
                            iteration_counts_buffers.output.buffer(),
                            0,
                            iteration_counts_staging_buffer.buffer(),
                            0,
                            (std::mem::size_of::<IterationCount>()
                                * screen_size.width as usize
                                * screen_size.height as usize) as u64,
                        );
                    },
                );

                queue.submit([compute_command_buffer]);

                let iteration_counts_staging_buffer_slice =
                    iteration_counts_staging_buffer.slice(..);

                {
                    trace!("waiting for staging buffer");
                    let mapped = Arc::new((Mutex::new(true), Condvar::new()));

                    iteration_counts_staging_buffer_slice.map_async(wgpu::MapMode::Read, {
                        let mapped = mapped.clone();
                        move |map_result| {
                            debug!("map_async callback called");
                            map_result.unwrap_or_else(|err| panic!("buffer async error: {}", err));
                            let mut guard = mapped.0.lock().unwrap();
                            *guard = false;
                            mapped.1.notify_all();
                        }
                    });

                    {
                        let device = Arc::from(&device);
                        std::thread::scope(|scope| {
                            scope.spawn(|| while !device.poll(wgpu::Maintain::Poll) {});
                        });
                    }

                    trace!("waiting for condition");
                    let _guard = mapped
                        .1
                        .wait_while(mapped.0.lock().unwrap(), |pending| *pending)
                        .unwrap();
                    trace!("staging buffer mapped");
                }

                let iteration_counts_staging_buffer_view =
                    iteration_counts_staging_buffer_slice.get_mapped_range();

                let colour_ranges = compute_colour_ranges(iteration_counts_staging_buffer_view);

                iteration_counts_staging_buffer.buffer().unmap();

                colour_ranges_buffer.write(&queue, &colour_ranges);

                let render_command_buffer = command_buffer::create(
                    &device,
                    &wgpu::CommandEncoderDescriptor::default(),
                    |command_encoder| {
                        command_encoder.push_debug_group("render-pass");
                        command_encoder.with_render_pass(
                            &wgpu::RenderPassDescriptor {
                                label: Some("render-pass"),
                                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                    view: &surface_texture_view,
                                    resolve_target: None,
                                    ops: wgpu::Operations {
                                        load: wgpu::LoadOp::Clear(wgpu::Color {
                                            r: 0.5,
                                            g: 0.5,
                                            b: 0.0,
                                            a: 1.0,
                                        }),
                                        store: true,
                                    },
                                })],
                                depth_stencil_attachment: None,
                            },
                            |render_pass| {
                                render_pass.set_pipeline(&render_pipeline);
                                render_pass.set_bind_group(0, &render_bind_group_1, &[]);
                                render_pass.set_bind_group(1, &render_bind_group_2, &[]);
                                render_pass.draw(0..4, 0..1);
                            },
                        );
                        command_encoder.pop_debug_group();
                    },
                );

                queue.submit([render_command_buffer]);

                surface_texture.present();

                starting_values_buffers.swap();
                iteration_counts_buffers.swap();
            }
            _ => {}
        }
    });
}
