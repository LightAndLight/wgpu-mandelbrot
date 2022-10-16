use std::marker::PhantomData;

use wgpu::util::DeviceExt;

pub struct Var<A> {
    buffer: wgpu::Buffer,
    phantom_data: PhantomData<A>,
}

impl<A: bytemuck::Pod + bytemuck::Zeroable> Var<A> {
    pub fn write(&self, queue: &wgpu::Queue, contents: A) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[contents]));
    }

    pub fn binding_resource(&self) -> wgpu::BindingResource {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.buffer,
            offset: 0,
            size: None,
        })
    }

    pub fn destroy(self) {
        self.buffer.destroy()
    }
}

pub struct Builder<'a, A> {
    label: Option<&'a str>,
    contents: A,
    usage: wgpu::BufferUsages,
}

impl<'a, A: bytemuck::Pod + bytemuck::Zeroable> Builder<'a, A> {
    pub fn new(contents: A) -> Self {
        Self {
            label: None,
            contents,
            usage: wgpu::BufferUsages::empty(),
        }
    }

    pub fn with_label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }

    pub fn with_usage(mut self, usage: wgpu::BufferUsages) -> Self {
        self.usage |= usage;
        self
    }

    pub fn create(self, device: &wgpu::Device) -> Var<A> {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: self.label,
            contents: bytemuck::cast_slice(&[self.contents]),
            usage: self.usage,
        });

        Var {
            buffer,
            phantom_data: PhantomData,
        }
    }
}
