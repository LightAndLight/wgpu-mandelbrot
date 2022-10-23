//! Screen data.

use bytemuck::{Pod, Zeroable};

/// [`bytemuck`]-compatible screen size.
#[repr(C)]
#[derive(Pod, Zeroable, Clone, Copy, Debug)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}
