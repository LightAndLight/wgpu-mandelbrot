//! Functions for working with [`wgpu::CommandBuffer`]s.

/// A scoped alternative to [`wgpu::Device::create_command_encoder`] and [`wgpu::CommandEncoder::finish`].
pub fn create(
    device: &wgpu::Device,
    descriptor: &wgpu::CommandEncoderDescriptor,
    function: impl FnOnce(&mut wgpu::CommandEncoder),
) -> wgpu::CommandBuffer {
    let mut command_encoder = device.create_command_encoder(descriptor);
    function(&mut command_encoder);
    command_encoder.finish()
}
