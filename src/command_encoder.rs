pub trait CommandEncoderExt {
    fn with_compute_pass<'pass, A>(
        &'pass mut self,
        descriptor: &wgpu::ComputePassDescriptor,
        function: impl FnOnce(&mut wgpu::ComputePass<'pass>) -> A,
    ) -> A;

    fn with_render_pass<'pass, A>(
        &'pass mut self,
        descriptor: &wgpu::RenderPassDescriptor<'pass, '_>,
        function: impl FnOnce(&mut wgpu::RenderPass<'pass>) -> A,
    ) -> A;
}

impl CommandEncoderExt for wgpu::CommandEncoder {
    fn with_compute_pass<'pass, A>(
        &'pass mut self,
        descriptor: &wgpu::ComputePassDescriptor,
        function: impl FnOnce(&mut wgpu::ComputePass<'pass>) -> A,
    ) -> A {
        let mut compute_pass = self.begin_compute_pass(descriptor);
        function(&mut compute_pass)
    }

    fn with_render_pass<'pass, A>(
        &'pass mut self,
        descriptor: &wgpu::RenderPassDescriptor<'pass, '_>,
        function: impl FnOnce(&mut wgpu::RenderPass<'pass>) -> A,
    ) -> A {
        let mut render_pass = self.begin_render_pass(descriptor);
        function(&mut render_pass)
    }
}
