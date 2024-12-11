use core::str;
use std::{fs::File, path::Path};

use memmap2::Mmap;

use crate::{
    node::{NxNode, NxNodeData},
    NxError, NxTryGet,
};

/// A memory mapped NX file.
pub struct NxFile {
    pub(crate) data: Mmap,
    pub(crate) header: NxHeader,
    pub(crate) root: NxNodeData,
}

impl NxFile {
    /// Opens and memory maps an NX file.
    ///
    /// This function is marked unsafe since memory mapping a file is inherently unsafe. See the
    /// memmap2 docs for more info.
    pub unsafe fn open(path: &Path) -> Result<Self, NxError> {
        let file = File::open(path)?;
        let data = Mmap::map(&file)?;

        let header = NxHeader::new(&data)?;
        println!("{:?}", header);

        let root = data.try_get_node_data(header.node_offset)?;
        println!("{:?}", root);

        Ok(Self { data, header, root })
    }

    /// Gets the total number of nodes in the file.
    pub fn node_count(&self) -> u32 {
        self.header.node_count
    }

    /// Gets the total number of strings in the file.
    pub fn string_count(&self) -> u32 {
        self.header.string_count
    }

    /// Gets the total number of bitmaps in the file.
    pub fn bitmap_count(&self) -> u32 {
        self.header.bitmap_count
    }

    /// Gets the total number of audio tracks in the file.
    pub fn audio_count(&self) -> u32 {
        self.header.audio_count
    }

    /// Gets the root node.
    pub fn root(&self) -> NxNode {
        NxNode {
            data: self.root,
            file: self,
        }
    }

    pub(crate) fn get_str(&self, index: u32) -> Result<&str, NxError> {
        let offset = self
            .data
            .try_get_u64(self.header.string_offset + (index as u64 * size_of::<u64>() as u64))?;

        let len = self.data.try_get_u16(offset)?;
        Ok(self.data.try_get_str(offset + 2, len)?)
    }
}

#[derive(Debug)]
pub(crate) struct NxHeader {
    node_count: u32,
    pub(crate) node_offset: u64,
    string_count: u32,
    pub(crate) string_offset: u64,
    bitmap_count: u32,
    pub(crate) bitmap_offset: u64,
    audio_count: u32,
    pub(crate) audio_offset: u64,
}

impl NxHeader {
    pub fn new(data: &Mmap) -> Result<Self, NxError> {
        // Validate that the first 4 bytes equals "PKG4".
        if data.try_get_u32(0)? != 0x34474B50 {
            return Err(NxError::InvalidMagicBytes);
        }

        Ok(Self {
            node_count: data.try_get_u32(4)?,
            node_offset: data.try_get_u64(8)?,
            string_count: data.try_get_u32(16)?,
            string_offset: data.try_get_u64(20)?,
            bitmap_count: data.try_get_u32(28)?,
            bitmap_offset: data.try_get_u64(32)?,
            audio_count: data.try_get_u32(40)?,
            audio_offset: data.try_get_u64(44)?,
        })
    }
}
