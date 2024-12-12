use std::cmp::Ordering;

use crate::{file::NxFile, NxError, NxTryGet};

const NX_NODE_OFFSET: u64 = 20;

#[derive(Debug, Copy, Clone)]
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
    pub fn get(&self, name: &str) -> Option<NxNode> {
        // TODO: this might need to be self.data.index + ...
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
                    index += middle * (NX_NODE_OFFSET * 2);
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

    pub fn name(&self) -> Result<&str, NxError> {
        self.file.get_str(self.data.name)
    }

    /// Gets the data type of the node.
    pub fn data_type(&self) -> NxNodeType {
        self.data.data_type
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
