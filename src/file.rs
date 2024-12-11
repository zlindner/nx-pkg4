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
    /// Opens and memory maps an NX file, then validates its header.
    pub fn open(path: &Path) -> Result<Self, NxError> {
        let file = File::open(path)?;

        // Safety: a memory mapped file is unsafe as undefined behaviour can occur if the file is
        // ever modified by another process while in use. This crate aims to provide a fully safe
        // api so this shouldn't be a concern, however modifying the file will result in either
        // NxErrors or values that don't make sense.
        let data = unsafe { Mmap::map(&file)? };
        let header = NxHeader::new(&data)?;
        let root = data.try_get_node_data(header.node_offset)?;

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
            return Err(NxError::InvalidHeader);
        }

        // TODO any errors here should be mapped to InvalidHeader.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_file_does_not_exist() {
        let result = unsafe { NxFile::open(Path::new("data/file_that_does_not_exist.nx")) };
        assert!(result.is_err());
    }

    #[test]
    fn open_file_with_invalid_header() {
        let result = unsafe { NxFile::open(Path::new("data/invalid_header.nx")) };
        assert!(result.is_err());
        assert!(matches!(result.err().unwrap(), NxError::InvalidHeader));
    }

    #[test]
    fn open_valid_file() {
        let result = unsafe { NxFile::open(Path::new("data/valid.nx")) };
        assert!(result.is_ok());

        let file = result.unwrap();
        assert_eq!(file.node_count(), 432);
        assert_eq!(file.string_count(), 227);
        assert_eq!(file.bitmap_count(), 0);
        assert_eq!(file.audio_count(), 0);
    }
}
