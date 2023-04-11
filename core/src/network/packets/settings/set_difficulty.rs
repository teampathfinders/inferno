use util::{bail, Error, Result};
use util::{Deserialize, Serialize};
use util::bytes::{BinaryRead, BinaryWrite, MutableBuffer, SharedBuffer, size_of_varint};

use crate::network::ConnectedPacket;

/// The Minecraft difficulties.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Difficulty {
    Peaceful,
    Easy,
    Normal,
    Hard,
}

impl TryFrom<i32> for Difficulty {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> anyhow::Result<Self> {
        Ok(match value {
            0 => Self::Peaceful,
            1 => Self::Easy,
            2 => Self::Normal,
            3 => Self::Hard,
            _ => bail!(Malformed, "Invalid difficulty type {value}"),
        })
    }
}

/// Sets the difficulty of the level.
///
/// This does not do a lot client-side, it is mainly used to sync the difficulty setting in the client's world settings.
#[derive(Debug, Clone)]
pub struct SetDifficulty {
    /// Difficulty to apply.
    pub difficulty: Difficulty,
}

impl ConnectedPacket for SetDifficulty {
    const ID: u32 = 0x3c;

    fn serialized_size(&self) -> usize {
        size_of_varint(self.difficulty as i32)
    }
}

impl Serialize for SetDifficulty {
    fn serialize<W>(&self, buffer: W) -> anyhow::Result<()> where W: BinaryWrite {
        buffer.write_var_i32(self.difficulty as i32)
    }
}

impl Deserialize<'_> for SetDifficulty {
    fn deserialize(mut buffer: SharedBuffer) -> anyhow::Result<Self> {
        let difficulty = Difficulty::try_from(buffer.read_var_i32()?)?;

        Ok(Self { difficulty })
    }
}
