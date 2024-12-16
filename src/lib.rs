use core::str;

use node::NxNodeData;
use thiserror::Error;

pub mod file;
pub mod node;

/// An error that occured when reading an NX file.
#[derive(Error, Debug)]
pub enum NxError {
    #[error("failed to load nx file")]
    Io(#[from] std::io::Error),

    #[error("the file's header is invalid")]
    InvalidHeader,

    #[error("{0} is out of bounds")]
    OutOfBoundsIndex(usize),

    #[error("{0}..{1} is out of bounds")]
    OutOfBoundsRange(usize, usize),

    #[error("invalid cast")]
    InvalidCast(#[from] core::array::TryFromSliceError),

    #[error("invalid string")]
    InvalidString(#[from] core::str::Utf8Error),
}

#[derive(Debug)]
pub struct NxBitmap {
    pub width: u16,
    pub height: u16,
    pub data: Vec<u8>,
}

pub(crate) trait NxTryGet {
    fn try_get_bytes(&self, index: u64, len: usize) -> Result<&[u8], NxError>;

    fn try_get_u16(&self, index: u64) -> Result<u16, NxError>;

    fn try_get_u32(&self, index: u64) -> Result<u32, NxError>;

    fn try_get_u64(&self, index: u64) -> Result<u64, NxError>;

    fn try_get_str(&self, index: u64, len: u16) -> Result<&str, NxError>;

    fn try_get_node_data(&self, index: u64) -> Result<NxNodeData, NxError>;
}

impl NxTryGet for [u8] {
    fn try_get_bytes(&self, index: u64, len: usize) -> Result<&[u8], NxError> {
        let usize_index = index as usize;
        let usize_len = len as usize;

        Ok(self
            .get(usize_index..usize_index + usize_len)
            .ok_or(NxError::OutOfBoundsRange(
                usize_index,
                usize_index + usize_len,
            ))?)
    }

    fn try_get_u16(&self, index: u64) -> Result<u16, NxError> {
        let bytes = self.try_get_bytes(index, size_of::<u16>())?;
        Ok(u16::from_le_bytes(bytes.try_into()?))
    }

    fn try_get_u32(&self, index: u64) -> Result<u32, NxError> {
        let bytes = self.try_get_bytes(index, size_of::<u32>())?;
        Ok(u32::from_le_bytes(bytes.try_into()?))
    }

    fn try_get_u64(&self, index: u64) -> Result<u64, NxError> {
        let bytes = self.try_get_bytes(index, size_of::<u64>())?;
        Ok(u64::from_le_bytes(bytes.try_into()?))
    }

    fn try_get_str(&self, index: u64, len: u16) -> Result<&str, NxError> {
        let bytes = self.try_get_bytes(index, len as usize)?;
        Ok(str::from_utf8(bytes)?)
    }

    fn try_get_node_data(&self, index: u64) -> Result<NxNodeData, NxError> {
        let usize_index = index as usize;
        let node_table = self
            .get(usize_index..)
            .ok_or(NxError::OutOfBoundsIndex(usize_index))?;

        let name = node_table.try_get_u32(0)?;
        let children = node_table.try_get_u32(4)?;
        let count = node_table.try_get_u16(8)?;
        let data_type = node_table.try_get_u16(10)?.into();
        let data = node_table.try_get_u64(12)?;

        Ok(NxNodeData {
            index,
            name,
            children,
            count,
            data_type,
            data,
        })
    }
}
