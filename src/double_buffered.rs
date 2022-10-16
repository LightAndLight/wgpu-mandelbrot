use bytemuck::{Pod, Zeroable};

use crate::buffer;

pub struct DoubleBuffered<A> {
    pub input: buffer::Buffer<A>,
    pub output: buffer::Buffer<A>,
}

impl<A: Pod + Zeroable> DoubleBuffered<A> {
    pub fn swap(&mut self) {
        std::mem::swap(&mut self.input, &mut self.output)
    }

    pub fn destroy(self) {
        self.input.destroy();
        self.output.destroy();
    }
}
