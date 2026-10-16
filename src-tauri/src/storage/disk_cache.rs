use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::storage::frame_buffer::RawFrame;

const FRAME_MAGIC: &[u8; 4] = b"GFRM";
const HEADER_SIZE: usize = 28;

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("LZ4 compression failed: {0}")]
    Compression(String),

    #[error("LZ4 decompression failed: {0}")]
    Decompression(String),

    #[error("Frame index {index} out of bounds (total frames: {total})")]
    IndexOutOfBounds { index: usize, total: usize },

    #[error("Corrupt frame header at index {index}: {reason}")]
    CorruptFrameHeader { index: usize, reason: String },

    #[error("Storage file lock poisoned")]
    LockPoisoned,

    #[error("Cache file is closed")]
    CacheClosed,
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// In-memory index entry tracking byte offsets and frame metadata in `frames.bin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameIndexEntry {
    pub offset: u64,
    pub timestamp_ms: u64,
    pub width: u32,
    pub height: u32,
    pub uncompressed_len: u32,
    pub compressed_len: u32,
}

/// DiskFrameCache manages LZ4-compressed sequential frame buffers in `frames.bin`.
/// It provides indexed random access to frames and automatic RAII cleanup on drop.
pub struct DiskFrameCache {
    file_path: PathBuf,
    file: Option<Mutex<File>>,
    index: Vec<FrameIndexEntry>,
    delete_on_drop: bool,
}

impl std::fmt::Debug for DiskFrameCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiskFrameCache")
            .field("file_path", &self.file_path)
            .field("total_frames", &self.index.len())
            .field("delete_on_drop", &self.delete_on_drop)
            .finish()
    }
}

impl DiskFrameCache {
    /// Initialize a new frame cache within the specified directory.
    /// Creates or truncates `frames.bin`.
    pub fn new(temp_dir: impl AsRef<Path>) -> Result<Self> {
        let dir = temp_dir.as_ref();
        fs::create_dir_all(dir)?;
        let file_path = dir.join("frames.bin");

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&file_path)?;

        Ok(Self {
            file_path,
            file: Some(Mutex::new(file)),
            index: Vec::new(),
            delete_on_drop: true,
        })
    }

    /// Open an existing `frames.bin` file and rebuild the in-memory index table.
    pub fn open_existing(file_path: impl AsRef<Path>) -> Result<Self> {
        let file_path = file_path.as_ref().to_path_buf();
        let mut file = OpenOptions::new().read(true).write(true).open(&file_path)?;

        let index = Self::scan_index_from_file(&mut file)?;

        Ok(Self {
            file_path,
            file: Some(Mutex::new(file)),
            index,
            delete_on_drop: false,
        })
    }

    /// Appends a new uncompressed frame to disk using LZ4 block compression.
    /// Frame metadata (magic, timestamp, dimensions, payload sizes) is written
    /// in the 28-byte chunk header preceding the compressed payload.
    pub fn append_frame(&mut self, frame: &RawFrame) -> Result<()> {
        let compressed = lz4_flex::compress(&frame.bgra_data);
        let compressed_len = u32::try_from(compressed.len())
            .map_err(|e| StorageError::Compression(format!("compressed frame too large: {e}")))?;
        let uncompressed_len = u32::try_from(frame.bgra_data.len())
            .map_err(|e| StorageError::Compression(format!("frame data too large: {e}")))?;

        let file_ref = self.file.as_ref().ok_or(StorageError::CacheClosed)?;
        let mut file_guard = file_ref.lock().map_err(|_| StorageError::LockPoisoned)?;

        let offset = file_guard.seek(SeekFrom::End(0))?;

        let mut header = [0u8; HEADER_SIZE];
        header[0..4].copy_from_slice(FRAME_MAGIC);
        header[4..12].copy_from_slice(&frame.timestamp_ms.to_le_bytes());
        header[12..16].copy_from_slice(&frame.width.to_le_bytes());
        header[16..20].copy_from_slice(&frame.height.to_le_bytes());
        header[20..24].copy_from_slice(&uncompressed_len.to_le_bytes());
        header[24..28].copy_from_slice(&compressed_len.to_le_bytes());

        file_guard.write_all(&header)?;
        file_guard.write_all(&compressed)?;
        file_guard.flush()?;

        self.index.push(FrameIndexEntry {
            offset,
            timestamp_ms: frame.timestamp_ms,
            width: frame.width,
            height: frame.height,
            uncompressed_len,
            compressed_len,
        });

        Ok(())
    }

    /// Read and decompress a frame at the specified index.
    pub fn read_frame(&self, index: usize) -> Result<RawFrame> {
        let entry = self
            .index
            .get(index)
            .ok_or(StorageError::IndexOutOfBounds {
                index,
                total: self.index.len(),
            })?;

        let file_ref = self.file.as_ref().ok_or(StorageError::CacheClosed)?;
        let mut file_guard = file_ref.lock().map_err(|_| StorageError::LockPoisoned)?;

        file_guard.seek(SeekFrom::Start(entry.offset))?;

        let mut header = [0u8; HEADER_SIZE];
        file_guard.read_exact(&mut header)?;

        if &header[0..4] != FRAME_MAGIC {
            return Err(StorageError::CorruptFrameHeader {
                index,
                reason: format!("invalid frame magic bytes: {:?}", &header[0..4]),
            });
        }

        let timestamp_ms = u64::from_le_bytes(Self::parse_header_le::<8>(
            &header,
            4,
            index,
            "timestamp_ms",
        )?);
        let width = u32::from_le_bytes(Self::parse_header_le::<4>(&header, 12, index, "width")?);
        let height = u32::from_le_bytes(Self::parse_header_le::<4>(&header, 16, index, "height")?);
        let uncompressed_len = u32::from_le_bytes(Self::parse_header_le::<4>(
            &header,
            20,
            index,
            "uncompressed_len",
        )?);
        let compressed_len = u32::from_le_bytes(Self::parse_header_le::<4>(
            &header,
            24,
            index,
            "compressed_len",
        )?);

        if compressed_len != entry.compressed_len || uncompressed_len != entry.uncompressed_len {
            return Err(StorageError::CorruptFrameHeader {
                index,
                reason: format!(
                    "header length mismatch (got compressed={compressed_len}, uncompressed={uncompressed_len}; expected compressed={}, uncompressed={})",
                    entry.compressed_len, entry.uncompressed_len
                ),
            });
        }

        let mut compressed_buf = vec![0u8; compressed_len as usize];
        file_guard.read_exact(&mut compressed_buf)?;

        let bgra_data = lz4_flex::decompress(&compressed_buf, uncompressed_len as usize)
            .map_err(|e| StorageError::Decompression(e.to_string()))?;

        Ok(RawFrame {
            timestamp_ms,
            width,
            height,
            bgra_data,
        })
    }

    /// Total number of frames currently buffered in cache.
    pub fn total_frames(&self) -> usize {
        self.index.len()
    }

    /// Whether the frame cache contains no frames.
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Path to the underlying `frames.bin` file.
    pub fn file_path(&self) -> &Path {
        &self.file_path
    }

    /// In-memory slice of frame index entries.
    pub fn index_entries(&self) -> &[FrameIndexEntry] {
        &self.index
    }

    /// Enable or disable automatic cleanup of `frames.bin` when dropped.
    pub fn set_delete_on_drop(&mut self, delete: bool) {
        self.delete_on_drop = delete;
    }

    /// Parses a little-endian field from the fixed-size frame header.
    ///
    /// The header is a compile-time-known `[u8; HEADER_SIZE]`, so a short slice
    /// can only mean a broken header layout — surfaced as `CorruptFrameHeader`
    /// rather than an unchecked unwrap.
    fn parse_header_le<const N: usize>(
        header: &[u8],
        offset: usize,
        index: usize,
        field: &'static str,
    ) -> Result<[u8; N]> {
        header
            .get(offset..offset + N)
            .and_then(|bytes| bytes.try_into().ok())
            .ok_or(StorageError::CorruptFrameHeader {
                index,
                reason: format!("failed to parse {field}: need {N} bytes at offset {offset}"),
            })
    }

    /// Scan a `frames.bin` file and rebuild index entries.
    fn scan_index_from_file(file: &mut File) -> Result<Vec<FrameIndexEntry>> {
        let mut index = Vec::new();
        file.seek(SeekFrom::Start(0))?;
        let total_size = file.metadata()?.len();
        let mut current_offset = 0u64;

        while current_offset < total_size {
            if current_offset + (HEADER_SIZE as u64) > total_size {
                return Err(StorageError::CorruptFrameHeader {
                    index: index.len(),
                    reason: "truncated header at end of file".into(),
                });
            }

            let mut header = [0u8; HEADER_SIZE];
            file.read_exact(&mut header)?;

            if &header[0..4] != FRAME_MAGIC {
                return Err(StorageError::CorruptFrameHeader {
                    index: index.len(),
                    reason: format!("invalid frame magic bytes: {:?}", &header[0..4]),
                });
            }

            let timestamp_ms = u64::from_le_bytes(Self::parse_header_le::<8>(
                &header,
                4,
                index.len(),
                "timestamp_ms",
            )?);
            let width = u32::from_le_bytes(Self::parse_header_le::<4>(
                &header,
                12,
                index.len(),
                "width",
            )?);
            let height = u32::from_le_bytes(Self::parse_header_le::<4>(
                &header,
                16,
                index.len(),
                "height",
            )?);
            let uncompressed_len = u32::from_le_bytes(Self::parse_header_le::<4>(
                &header,
                20,
                index.len(),
                "uncompressed_len",
            )?);
            let compressed_len = u32::from_le_bytes(Self::parse_header_le::<4>(
                &header,
                24,
                index.len(),
                "compressed_len",
            )?);

            index.push(FrameIndexEntry {
                offset: current_offset,
                timestamp_ms,
                width,
                height,
                uncompressed_len,
                compressed_len,
            });

            current_offset += (HEADER_SIZE as u64) + (compressed_len as u64);
            file.seek(SeekFrom::Start(current_offset))?;
        }

        Ok(index)
    }
}

impl Drop for DiskFrameCache {
    fn drop(&mut self) {
        // Drop the file handle explicitly before attempting removal (critical on Windows).
        drop(self.file.take());

        if self.delete_on_drop && self.file_path.exists() {
            let _ = fs::remove_file(&self.file_path);
        }
    }
}
