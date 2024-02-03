use proto::types::Dimension;
use util::{BinaryRead, BinaryWrite, Vector};

/// The `AutonomousEntities` database key.
pub const AUTONOMOUS_ENTITIES: &[u8] = b"AutonomousEntities";
/// The `BiomeData` database key.
pub const BIOME_DATA: &[u8] = b"BiomeData";
/// The `LevelChunkMetaDataDictionary` database key.
pub const CHUNK_METADATA: &[u8] = b"LevelChunkMetaDataDictionary";
/// The `Overworld` database key.
pub const OVERWORLD: &[u8] = b"Overworld";
/// The `mobevents` database key.
pub const MOB_EVENTS: &[u8] = b"mobevents";
/// The `scoreboard` database key.
pub const SCOREBOARD: &[u8] = b"scoreboard";
/// The `schedulerWT` database key.
pub const SCHEDULER: &[u8] = b"schedulerWT";
/// The `~local_player` database key.
pub const LOCAL_PLAYER: &[u8] = b"~local_player";

/// Database key prefixes.
///
/// Data from [`Minecraft fandom`](https://minecraft.fandom.com/wiki/Bedrock_Edition_level_format#Chunk_key_format).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyType {
    /// 3D biome map.
    Biome3d = 0x2b,
    /// Version of the specified chunk.
    ChunkVersion = 0x2c,
    /// Heightmap containing the highest blocks in the given subchunk.
    HeightMap = 0x2d,
    /// Sub chunk data.
    SubChunk {
        /// Vertical position of the subchunk.
        /// This index can also be negative, indicating subchunks that are below 0.
        index: i8,
    } = 0x2f,
    /// The old terrain format.
    LegacyTerrain = 0x30,
    /// A block entity.
    BlockEntity = 0x31,
    /// An entity.
    Entity = 0x32,
    /// Pending tick data.
    PendingTicks = 0x33,
    /// Biome state.
    BiomeState = 0x35,
    /// Finalized state.
    FinalizedState = 0x36,
    /// Education Edition border blocks.
    BorderBlocks = 0x38,
    /// Bounding boxes for structure spawns stored in binary format.
    HardCodedSpawnAreas = 0x39,
    /// Random tick data.
    RandomTicks = 0x3a,
}

impl KeyType {
    /// Returns the discriminant of `self`.
    pub fn discriminant(&self) -> u8 {
        // SAFETY: KeyData is marked as `repr(u8)` and therefore its layout is a
        // `repr(C)` union of `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field. Hence, we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }
}

/// A key that can be loaded from the database.
#[derive(Debug, Clone)]
pub struct DataKey {
    /// X and Z coordinates of the requested chunk.
    pub coordinates: Vector<i32, 2>,
    /// Dimension of the chunk.
    pub dimension: Dimension,
    /// The tag of the data to load.
    pub data: KeyType,
}

impl DataKey {
    /// What is the size of this key after it has been serialised?
    pub(crate) fn serialized_size(&self) -> usize {
        4 + 4 + if self.dimension != Dimension::Overworld { 4 } else { 0 } + 1 + if let KeyType::SubChunk { .. } = self.data { 1 } else { 0 }
    }

    /// Serialises the key into the given writer.
    pub(crate) fn serialize<W>(&self, mut writer: W) -> anyhow::Result<()>
    where
        W: BinaryWrite,
    {
        writer.write_i32_le(self.coordinates.x)?;
        writer.write_i32_le(self.coordinates.y)?;

        if self.dimension != Dimension::Overworld {
            writer.write_i32_le(self.dimension as i32)?;
        }

        writer.write_u8(self.data.discriminant())?;
        if let KeyType::SubChunk { index } = self.data {
            writer.write_i8(index)?;
        }

        Ok(())
    }

    /// Deserialises a key from the given reader.
    pub(crate) fn deserialize<'a, R>(mut reader: R) -> anyhow::Result<Self>
    where
        R: BinaryRead<'a> + 'a,
    {
        let x = reader.read_i32_le()?;
        let z = reader.read_i32_le()?;

        let dimension = if reader.remaining() > 6 {
            Dimension::try_from(reader.read_u32_le()?)?
        } else {
            Dimension::Overworld
        };

        let key_ty = reader.read_u8()?;
        let data = match key_ty {
            0x2f => KeyType::SubChunk { index: reader.read_i8()? },
            0x2b => KeyType::Biome3d,
            0x2c => KeyType::ChunkVersion,
            0x2d => KeyType::HeightMap,
            0x30 => KeyType::LegacyTerrain,
            0x31 => KeyType::BlockEntity,
            0x32 => KeyType::Entity,
            0x33 => KeyType::PendingTicks,
            0x35 => KeyType::BiomeState,
            0x36 => KeyType::FinalizedState,
            0x38 => KeyType::BorderBlocks,
            0x39 => KeyType::HardCodedSpawnAreas,
            0x3a => KeyType::RandomTicks,
            _ => anyhow::bail!(format!("Invalid key type: {key_ty:x?}")),
        };

        Ok(Self {
            coordinates: Vector::from([x, z]),
            dimension,
            data,
        })
    }
}
