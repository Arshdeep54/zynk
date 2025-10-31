pub mod block;
pub mod builder;
pub mod index;
pub mod iter;
pub mod reader;

#[derive(Copy, Clone)]
pub struct BlockHandle {
    pub offset: u64,
    pub length: u32,
}

pub type TableId = u64;

pub const SSTABLE_VERSION: u32 = 1;
pub const SSTABLE_MAGIC: u64 = 0xF3515A5453544142;
pub const FOOTER_SIZE: usize = 8 + 4 + 4 + 8;
