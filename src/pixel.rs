use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
pub struct Complex {
    pub real: f32,
    pub imaginary: f32,
}

impl Complex {
    pub const ZERO: Self = Complex {
        real: 0.0,
        imaginary: 0.0,
    };
}

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
pub struct Pixel {
    pub x: u32,
    pub y: u32,
    pub escaped: u32,
    pub current_value: Complex,
    pub iteration_count: u32,
}
