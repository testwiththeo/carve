pub mod disk_cache;
pub mod frame_buffer;

pub use disk_cache::{DiskFrameCache, FrameIndexEntry, Result, StorageError};
pub use frame_buffer::RawFrame;
