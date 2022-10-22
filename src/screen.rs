use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}
