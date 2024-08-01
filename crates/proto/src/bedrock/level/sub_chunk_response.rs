use util::{BinaryWrite, RVec, Serialize, Vector};

use crate::bedrock::ConnectedPacket;
use crate::types::Dimension;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum SubChunkResult {
    Undefined = 0,
    #[default]
    Success = 1,
    NotFound = 2,
    InvalidDimension = 3,
    PlayerNotFound = 4,
    OutOfBounds = 5,
    AllAir = 6,
}

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum HeightmapType {
    #[default]
    None,
    WithData,
    TooHigh,
    TooLow,
}

#[derive(Debug)]
pub struct SubChunkEntry {
    pub offset: Vector<i8, 3>,
    pub result: SubChunkResult,
    pub payload: RVec,
    pub heightmap_type: HeightmapType,
    pub heightmap: Option<Box<[i8; 256]>>,
    pub blob_hash: u64,
}

impl Default for SubChunkEntry {
    fn default() -> SubChunkEntry {
        SubChunkEntry {
            offset: [0; 3].into(),
            result: SubChunkResult::NotFound,
            payload: RVec::alloc(),
            heightmap_type: HeightmapType::None,
            heightmap: None,
            blob_hash: 0,
        }
    }
}

impl SubChunkEntry {
    #[inline]
    fn serialize_cached<W: BinaryWrite>(&self, _writer: &mut W) -> anyhow::Result<()> {
        todo!();
    }

    #[inline]
    fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_vecb(&self.offset)?;
        writer.write_u8(self.result as u8)?;
        writer.write_var_u32(self.payload.len() as u32)?;
        writer.write_all(&self.payload)?;
        writer.write_u8(self.heightmap_type as u8)?;
        if self.heightmap_type == HeightmapType::WithData {
            let slice: &[i8; 256] = self.heightmap.as_ref().unwrap();
            writer.write_all(bytemuck::cast_slice(slice))?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct SubChunkResponse {
    pub cache_enabled: bool,
    pub dimension: Dimension,
    pub position: Vector<i32, 3>,
    pub entries: Vec<SubChunkEntry>,
}

impl ConnectedPacket for SubChunkResponse {
    const ID: u32 = 0xae;
}

impl Serialize for SubChunkResponse {
    fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_bool(self.cache_enabled)?;
        writer.write_var_i32(self.dimension as i32)?;
        writer.write_veci(&self.position)?;

        writer.write_u32_le(self.entries.len() as u32)?;
        if self.cache_enabled {
            for entry in &self.entries {
                entry.serialize_cached(writer)?;
            }
        } else {
            for entry in &self.entries {
                entry.serialize_into(writer)?;
            }
        }

        Ok(())
    }
}
