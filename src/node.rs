use std::cmp::Ordering;

use lz4_flex::decompress;

use crate::{file::NxFile, NxBitmap, NxError, NxTryGet};

const NX_NODE_OFFSET: u64 = 20;

#[derive(Copy, Clone)]
pub(crate) struct NxNodeData {
    pub(crate) index: u64,
    pub(crate) name: u32,
    pub(crate) children: u32,
    pub(crate) count: u16,
    pub(crate) data_type: NxNodeType,
    pub(crate) data: u64,
}

/// A node in an NX file.
pub struct NxNode<'a> {
    pub(crate) data: NxNodeData,
    pub(crate) file: &'a NxFile,
}

impl<'a> NxNode<'a> {
    /// Gets a node with the given name starting from the current node.
    fn get(&self, name: &str) -> Option<NxNode> {
        let mut index = self.file.header.node_offset + self.data.children as u64 * NX_NODE_OFFSET;
        let mut count = self.data.count as u64;

        while count > 0 {
            let middle = count / 2;

            let current = match self
                .file
                .data
                .try_get_node_data(index + (middle * NX_NODE_OFFSET))
            {
                Ok(node) => node,
                Err(_) => return None,
            };

            let current_name = match self.file.get_str(current.name) {
                Ok(name) => name,
                Err(_) => return None,
            };

            match current_name.cmp(name) {
                Ordering::Less => {
                    index = current.index + NX_NODE_OFFSET;
                    count -= middle + 1;
                }
                Ordering::Equal => {
                    return Some(NxNode {
                        data: current,
                        file: self.file,
                    });
                }
                Ordering::Greater => count = middle,
            }
        }

        None
    }

    /// Gets the name of the node.
    pub fn name(&self) -> Result<&str, NxError> {
        self.file.get_str(self.data.name)
    }

    /// Gets the data type of the node.
    pub fn data_type(&self) -> NxNodeType {
        self.data.data_type
    }

    /// Gets a bitmap from a node.
    pub fn bitmap(&self) -> Result<Option<NxBitmap>, NxError> {
        match self.data.data_type {
            NxNodeType::Bitmap => {
                // Data is a u64 that we need to reinterpret as a u32 (index), and two u16's
                // (width and height).
                let bytes = self.data.data.to_le_bytes();

                let index = u32::from_le_bytes(bytes[0..4].try_into()?);
                let width = u16::from_le_bytes(bytes[4..6].try_into()?);
                let height = u16::from_le_bytes(bytes[6..8].try_into()?);

                let data = decompress(
                    self.file.get_bitmap(index)?,
                    width as usize * height as usize * size_of::<u32>(),
                )
                .unwrap();

                let bitmap = NxBitmap {
                    width,
                    height,
                    data,
                };

                Ok(Some(bitmap))
            }
            _ => Ok(None),
        }
    }

    /// Gets an iterator over the node's children.
    pub fn iter(&self) -> Result<NxNodeIterator, NxError> {
        let data = self.file.data.try_get_node_data(
            self.file.header.node_offset + self.data.children as u64 * NX_NODE_OFFSET,
        )?;

        Ok(NxNodeIterator {
            data,
            file: self.file,
            count: self.data.count as usize,
        })
    }
}

/// The type of a node.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum NxNodeType {
    Empty,
    Integer,
    Float,
    String,
    Vector,
    Bitmap,
    Audio,
    Invalid(u16),
}

impl From<u16> for NxNodeType {
    fn from(value: u16) -> Self {
        match value {
            0 => Self::Empty,
            1 => Self::Integer,
            2 => Self::Float,
            3 => Self::String,
            4 => Self::Vector,
            5 => Self::Bitmap,
            6 => Self::Audio,
            _ => Self::Invalid(value),
        }
    }
}

pub trait Node {
    fn get(&self, name: &str) -> Option<NxNode>;

    fn bitmap(&self) -> Result<Option<NxBitmap>, NxError>;
}

impl<'a> Node for NxNode<'a> {
    fn get(&self, name: &str) -> Option<NxNode> {
        self.get(name)
    }

    fn bitmap(&self) -> Result<Option<NxBitmap>, NxError> {
        self.bitmap()
    }
}

impl<'a> Node for Option<NxNode<'a>> {
    fn get(&self, name: &str) -> Option<NxNode> {
        match self {
            Some(node) => node.get(name),
            None => None,
        }
    }

    fn bitmap(&self) -> Result<Option<NxBitmap>, NxError> {
        match self {
            Some(node) => node.bitmap(),
            None => Ok(None),
        }
    }
}

/// A node iterator.
pub struct NxNodeIterator<'a> {
    data: NxNodeData,
    file: &'a NxFile,
    count: usize,
}

impl<'a> Iterator for NxNodeIterator<'a> {
    type Item = NxNode<'a>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }

    fn next(&mut self) -> Option<NxNode<'a>> {
        match self.count {
            0 => None,
            _ => {
                self.count -= 1;

                let node = NxNode {
                    data: self.data,
                    file: self.file,
                };

                // Get the next child node.
                // It's position will be the current node's position + the size of a node.
                let next = match self
                    .file
                    .data
                    .try_get_node_data(self.data.index + NX_NODE_OFFSET)
                {
                    Ok(node) => node,
                    Err(_) => return None,
                };

                self.data = next;
                Some(node)
            }
        }
    }
}
