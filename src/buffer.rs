use std::{
    marker::PhantomData,
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
