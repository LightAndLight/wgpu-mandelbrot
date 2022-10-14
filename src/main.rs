use log::debug;
use wgpu::util::DeviceExt;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

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

    let compute_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compute-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    count: None,
                },
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
            ],
        });

    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("compute-pipeline-layout"),
        bind_group_layouts: &[&compute_bind_group_layout],
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

    let render_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("render-bind-group-layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("render-pipeline-layout"),
        bind_group_layouts: &[&render_bind_group_layout],
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

    let mut result_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("result-texture"),
        size: wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
    });

    let mut result_texture_view =
        result_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let screen_size_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("screen-size-buffer"),
        contents: bytemuck::cast_slice(&[size.width as f32, size.height as f32]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let mut compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("compute-bind-group"),
        layout: &compute_bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&result_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &screen_size_buffer,
                    offset: 0,
                    size: None,
                }),
            },
        ],
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

    let mut render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("render-bind-group"),
        layout: &render_pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&result_texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &screen_size_buffer,
                    offset: 0,
                    size: None,
                }),
            },
        ],
    });

    event_loop.run(move |event, _, control_flow| {
        let (result_texture, result_texture_view, compute_bind_group, render_bind_group) = (
            &mut result_texture,
            &mut result_texture_view,
            &mut compute_bind_group,
            &mut render_bind_group,
        );

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
                WindowEvent::Resized(size) => {
                    debug!("resizing to {:?}", size);

                    surface_configuration.width = size.width;
                    surface_configuration.height = size.height;

                    surface.configure(&device, &surface_configuration);

                    *result_texture = device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("result-texture"),
                        size: wgpu::Extent3d {
                            width: size.width,
                            height: size.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::STORAGE_BINDING
                            | wgpu::TextureUsages::TEXTURE_BINDING,
                    });

                    *result_texture_view =
                        result_texture.create_view(&wgpu::TextureViewDescriptor::default());

                    queue.write_buffer(
                        &screen_size_buffer,
                        0,
                        bytemuck::cast_slice(&[size.width as f32, size.height as f32]),
                    );

                    *compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("compute-bind-group"),
                        layout: &compute_bind_group_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(result_texture_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                    buffer: &screen_size_buffer,
                                    offset: 0,
                                    size: None,
                                }),
                            },
                        ],
                    });

                    *render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("render-bind-group"),
                        layout: &render_pipeline.get_bind_group_layout(0),
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(result_texture_view),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&sampler),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                    buffer: &screen_size_buffer,
                                    offset: 0,
                                    size: None,
                                }),
                            },
                        ],
                    });

                    window.request_redraw();
                }
                _ => {}
            },
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let surface_texture = surface.get_current_texture().unwrap();

                let surface_texture_view = surface_texture
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let command_encoder = {
                    let mut command_encoder =
                        device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

                    command_encoder.push_debug_group("compute-pass");
                    {
                        let mut compute_pass =
                            command_encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                                label: Some("compute-pass"),
                            });

                        compute_pass.set_pipeline(&compute_pipeline);
                        compute_pass.set_bind_group(0, compute_bind_group, &[]);
                        compute_pass.insert_debug_marker("mandelbrot");
                        compute_pass.dispatch_workgroups(size.width, size.height, 1);
                    }
                    command_encoder.pop_debug_group();

                    command_encoder.push_debug_group("render-pass");
                    {
                        let mut render_pass =
                            command_encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                            });

                        render_pass.set_pipeline(&render_pipeline);
                        render_pass.set_bind_group(0, render_bind_group, &[]);
                        render_pass.draw(0..4, 0..1);
                    }
                    command_encoder.pop_debug_group();

                    command_encoder
                };

                queue.submit([command_encoder.finish()]);
                surface_texture.present();
            }
            _ => {}
        }
    });
}
