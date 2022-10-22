/*!
Typed `wgpu` buffers.

[`bytemuck`](https://docs.rs/bytemuck/latest/bytemuck/) seems to be the
recommended way to cast Rust datatypes to bytes that can be sent to the GPU
([example](https://github.com/gfx-rs/wgpu/blob/d3ab5a197ed61b80d264bde150f76538d0c129a6/wgpu/examples/cube/main.rs)).

Casting in applications is error prone; you might create a buffer that's "supposed to"
contain `A`s, but nothing will stop you from writing a bunch of `B`s to it.

This module provides a type safe buffer API.
*/

use std::{
    marker::PhantomData,
    mem::size_of,
    num::NonZeroU64,
    ops::{Deref, DerefMut, RangeBounds},
};

use wgpu::util::DeviceExt;

pub struct Buffer<A> {
    buffer: wgpu::Buffer,
    phantom_data: PhantomData<A>,
}

impl<A: bytemuck::Pod + bytemuck::Zeroable> Buffer<A> {
    pub fn write(&self, queue: &wgpu::Queue, contents: &[A]) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(contents));
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn slice<S: RangeBounds<wgpu::BufferAddress>>(&self, bounds: S) -> Slice<A> {
        Slice {
            slice: self.buffer.slice(bounds),
            phantom_data: PhantomData,
        }
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

pub struct Slice<'a, A> {
    slice: wgpu::BufferSlice<'a>,
    phantom_data: PhantomData<A>,
}

impl<'a, A> Slice<'a, A> {
    pub fn map_async(
        &self,
        mode: wgpu::MapMode,
        callback: impl FnOnce(Result<(), wgpu::BufferAsyncError>) + Send + 'static,
    ) {
        self.slice.map_async(mode, callback)
    }

    pub fn get_mapped_range(&self) -> View<'a, A> {
        View {
            view: self.slice.get_mapped_range(),
            phantom_data: PhantomData,
        }
    }

    pub fn get_mapped_range_mut(&mut self) -> ViewMut<'a, A> {
        ViewMut {
            view_mut: self.slice.get_mapped_range_mut(),
            phantom_data: PhantomData,
        }
    }
}

pub struct View<'a, A> {
    view: wgpu::BufferView<'a>,
    phantom_data: PhantomData<A>,
}

impl<'a, A: bytemuck::Pod + bytemuck::Zeroable> Deref for View<'a, A> {
    type Target = [A];

    fn deref(&self) -> &Self::Target {
        bytemuck::cast_slice(&*self.view)
    }
}

pub struct ViewMut<'a, A> {
    view_mut: wgpu::BufferViewMut<'a>,
    phantom_data: PhantomData<A>,
}

impl<'a, A: bytemuck::Pod + bytemuck::Zeroable> Deref for ViewMut<'a, A> {
    type Target = [A];

    fn deref(&self) -> &Self::Target {
        bytemuck::cast_slice(&*self.view_mut)
    }
}

impl<'a, A: bytemuck::Pod + bytemuck::Zeroable> DerefMut for ViewMut<'a, A> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        bytemuck::cast_slice_mut(&mut *self.view_mut)
    }
}

enum Contents<'a> {
    Contents(&'a [u8]),
    Size(u64),
}

pub struct Builder<'a, A> {
    label: Option<&'a str>,
    contents: Contents<'a>,
    usage: wgpu::BufferUsages,
    phantom_data: PhantomData<A>,
}

impl<'a, A: bytemuck::Pod + bytemuck::Zeroable> From<&'a [A]> for Builder<'a, A> {
    fn from(value: &'a [A]) -> Self {
        Self {
            label: None,
            contents: Contents::Contents(bytemuck::cast_slice(value)),
            usage: wgpu::BufferUsages::COPY_DST,
            phantom_data: PhantomData,
        }
    }
}

impl<'a, A: bytemuck::Pod + bytemuck::Zeroable> Builder<'a, A> {
    pub fn new(size: u64) -> Self {
        Self {
            label: None,
            contents: Contents::Size(size),
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
        let buffer = match self.contents {
            Contents::Contents(contents) => {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label,
                    contents,
                    usage: self.usage,
                })
            }
            Contents::Size(size) => device.create_buffer(&wgpu::BufferDescriptor {
                label: self.label,
                size: size * size_of::<A>() as u64,
                usage: self.usage,
                mapped_at_creation: false,
            }),
        };

        Buffer {
            buffer,
            phantom_data: PhantomData,
        }
    }
}

pub struct DoubleBuffer<A> {
    pub input: Buffer<A>,
    pub output: Buffer<A>,
}

impl<A: bytemuck::Pod + bytemuck::Zeroable> DoubleBuffer<A> {
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.input, &mut self.output)
    }

    pub fn destroy(self) {
        self.input.destroy();
        self.output.destroy();
    }
}

pub fn copy_buffer_to_buffer<A: bytemuck::Pod + bytemuck::Zeroable>(
    command_encoder: &mut wgpu::CommandEncoder,
    source: &Buffer<A>,
    source_index: u64,
    destination: &Buffer<A>,
    destination_index: u64,
    copy_size: u64,
) {
    command_encoder.copy_buffer_to_buffer(
        source.buffer(),
        source_index * size_of::<A>() as u64,
        destination.buffer(),
        destination_index * size_of::<A>() as u64,
        copy_size * size_of::<A>() as u64,
    )
}
