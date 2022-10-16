use std::{marker::PhantomData, num::NonZeroU64};

use wgpu::util::DeviceExt;

pub struct Buffer<A> {
    buffer: wgpu::Buffer,
    phantom_data: PhantomData<A>,
}

impl<A: bytemuck::Pod + bytemuck::Zeroable> Buffer<A> {
    pub fn write(&self, queue: &wgpu::Queue, contents: &[A]) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(contents));
    }

    pub fn binding_resource(&self, offset: u64, size: Option<NonZeroU64>) -> wgpu::BindingResource {
        wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer: &self.buffer,
            offset,
            size,
        })
    }

    pub fn destroy(self) {
        self.buffer.destroy()
    }
}

pub struct Builder<'a, A> {
    label: Option<&'a str>,
    contents: &'a [u8],
    usage: wgpu::BufferUsages,
    phantom_data: PhantomData<A>,
}

impl<'a, A: bytemuck::Pod + bytemuck::Zeroable> Builder<'a, A> {
    pub fn new(contents: &'a [A]) -> Self {
        Self {
            label: None,
            contents: bytemuck::cast_slice(contents),
            usage: wgpu::BufferUsages::COPY_DST,
            phantom_data: PhantomData,
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

    pub fn create(self, device: &wgpu::Device) -> Buffer<A> {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: self.label,
            contents: self.contents,
            usage: self.usage,
        });

        Buffer {
            buffer,
            phantom_data: PhantomData,
        }
    }
}
