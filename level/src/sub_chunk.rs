use std::collections::HashMap;
use std::ops::{Index, IndexMut};

use serde::{Deserialize, Serialize};

use util::{bail, Error, Result, Vector};
use util::bytes::{BinaryRead, BinaryWrite, MutableBuffer};

const CHUNK_SIZE: usize = 4096;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SubChunkVersion {
    Legacy = 1,
    Limited = 8,
    Limitless = 9,
}

impl TryFrom<u8> for SubChunkVersion {
    type Error = Error;

    fn try_from(v: u8) -> Result<Self> {
        Ok(match v {
            1 => Self::Legacy,
            8 => Self::Limited,
            9 => Self::Limitless,
            _ => bail!(Malformed, "Invalid chunk version: {v}"),
        })
    }
}

mod block_version {
    use serde::{Deserialize, Deserializer, Serializer};

    #[inline]
    pub fn deserialize<'de, D>(de: D) -> Result<Option<[u8; 4]>, D::Error>
        where
            D: Deserializer<'de>
    {
        let word = Option::<i32>::deserialize(de)?;
        Ok(word.map(|w| w.to_be_bytes()))
    }

    #[inline]
    pub fn serialize<S>(v: &Option<[u8; 4]>, ser: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer
    {
        if let Some(b) = v {
            ser.serialize_i32(i32::from_be_bytes(*b))
        } else {
            ser.serialize_none()
        }
    }
}

/// Definition of block in the sub chunk block palette.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename = "")]
pub struct PaletteEntry {
    /// Name of the block.
    pub name: String,
    /// Version of the block.
    #[serde(with = "block_version")]
    pub version: Option<[u8; 4]>,
    /// Block-specific properties.
    pub states: HashMap<String, nbt::Value>,
}

/// A layer in a sub chunk.
///
/// Sub chunks can have multiple layers.
/// The first layer contains plain old block data,
/// while the second layer (if it exists) generally contains water logging data.
///
/// The layer is prefixed with a byte indicating the size in bits of the block indices.
/// This is followed by `4096 / (32 / bits)` 32-bit integers containing the actual indices.
/// In case the size is 3, 5 or 6, there is one more integer appended to the end to fit all data.
///
/// Immediately following the indices, the palette starts.
/// This is prefixed with a 32-bit little endian integer specifying the size of the palette.
/// The rest of the palette then consists of `n` concatenated NBT compounds.
#[doc(alias = "storage record")]
#[derive(Debug)]
pub struct SubLayer {
    /// List of indices into the palette.
    ///
    /// Coordinates can be converted to an offset into the array using [`to_offset`].
    pub(crate) indices: [u16; 4096],
    /// List of all different block types in this sub chunk layer.
    pub(crate) palette: Vec<PaletteEntry>,
}

impl SubLayer {
    /// Creates an iterator over the blocks in this layer.
    ///
    /// This iterates over every indices
    #[inline]
    pub fn iter(&self) -> LayerIter {
        LayerIter::from(self)
    }

    pub fn get(&self, pos: Vector<u8, 3>) -> Option<&PaletteEntry> {
        if pos.x > 16 || pos.y > 16 || pos.z > 16 {
            return None
        }

        let offset = to_offset(pos);
        debug_assert!(offset < 4096);

        let index = self.indices[offset] as usize;
        Some(&self.palette[index])
    }

    pub fn get_mut(&mut self, pos: Vector<u8, 3>) -> Option<&mut PaletteEntry> {
        if pos.x > 16 || pos.y > 16 || pos.z > 16 {
            return None
        }

        let offset = to_offset(pos);
        debug_assert!(offset < 4096);

        let index = self.indices[offset] as usize;
        Some(&mut self.palette[index])
    }

    #[inline]
    pub fn palette(&self) -> &[PaletteEntry] {
        &self.palette
    }

    #[inline]
    pub fn palette_mut(&mut self) -> &mut [PaletteEntry] {
        &mut self.palette
    }

    #[inline]
    pub fn indices(&self) -> &[u16; 4096] {
        &self.indices
    }

    #[inline]
    pub fn indices_mut(&mut self) -> &mut [u16; 4096] {
        &mut self.indices
    }


}

impl<I> Index<I> for SubLayer
where
    I: Into<Vector<u8, 3>>
{
    type Output = PaletteEntry;

    fn index(&self, position: I) -> &Self::Output {
        let position = position.into();
        assert!(position.x <= 16 && position.y <= 16 && position.z <= 16, "Block position out of sub chunk bounds");

        let offset = to_offset(position);
        let index = self.indices[offset] as usize;
        &self.palette[index]
    }
}

impl<I> IndexMut<I> for SubLayer
where
    I: Into<Vector<u8, 3>>
{
    fn index_mut(&mut self, position: I) -> &mut Self::Output {
        let position = position.into();
        assert!(position.x <= 16 && position.y <= 16 && position.z <= 16, "Block position out of sub chunk bounds");

        let offset = to_offset(position);
        let index = self.indices[offset] as usize;
        &mut self.palette[index]
    }
}

impl Default for SubLayer {
    // Std using const generics for arrays would be really nice...
    fn default() -> Self {
        Self { indices: [0; 4096], palette: Vec::new() }
    }
}

/// Converts coordinates to offsets into the block palette indices.
///
/// These coordinates should be in the range [0, 16) for each component.
#[inline]
pub fn to_offset(position: Vector<u8, 3>) -> usize {
    16 * 16 * position.x as usize
        + 16 * position.z as usize
        + position.y as usize
}

/// Converts an offset back to coordinates.
///
/// This offset should be in the range [0, 4096).
#[inline]
pub fn from_offset(offset: usize) -> Vector<u8, 3> {
    Vector::from([
        (offset >> 8) as u8 & 0xf,
        offset as u8 & 0xf,
        (offset >> 4) as u8 & 0xf,
    ])
}

/// A Minecraft sub chunk.
///
/// Every world contains
#[derive(Debug)]
pub struct SubChunk {
    /// Version of the sub chunk.
    ///
    /// See [`SubChunkVersion`] for more info.
    pub(crate) version: SubChunkVersion,
    /// Index of the sub chunk.
    ///
    /// This specifies the vertical position of the sub chunk.
    /// It is only used if `version` is set to [`Limitless`](SubChunkVersion::Limitless)
    /// and set to 0 otherwise.
    pub(crate) index: i8,
    /// Layers the sub chunk consists of.
    ///
    /// See [`SubLayer`] for more info.
    pub(crate) layers: Vec<SubLayer>,
}

impl SubChunk {
    /// Returns the specified layer from the sub chunk.
    #[inline]
    pub fn layer(&self, index: usize) -> Option<&SubLayer> {
        self.layers.get(index)
    }

    #[inline]
    pub fn layer_mut(&mut self, index: usize) -> Option<&mut SubLayer> {
        self.layers.get_mut(index)
    }
}

impl SubChunk {

}

impl Index<usize> for SubChunk {
    type Output = SubLayer;

    #[inline]
    fn index(&self, index: usize) -> &Self::Output {
        &self.layers[index]
    }
}

impl IndexMut<usize> for SubChunk {
    #[inline]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.layers[index]
    }
}

/// Iterator over blocks in a layer.
pub struct LayerIter<'a> {
    /// Indices in the sub chunk.
    /// While iterating, this is slowly consumed by `std::slice::split_at`.
    indices: &'a [u16],
    /// All possible block states in the current chunk.
    palette: &'a [PaletteEntry],
}

impl<'a> From<&'a SubLayer> for LayerIter<'a> {
    #[inline]
    fn from(value: &'a SubLayer) -> Self {
        Self {
            indices: &value.indices,
            palette: value.palette.as_ref(),
        }
    }
}

impl<'a> Iterator for LayerIter<'a> {
    type Item = &'a PaletteEntry;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // ExactSizeIterator::is_empty is unstable
        if self.len() == 0 {
            return None;
        }

        let (a, b) = self.indices.split_at(1);
        self.indices = b;
        self.palette.get(a[0] as usize)
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.len()))
    }
}

impl<'a> ExactSizeIterator for LayerIter<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.indices.len()
    }
}