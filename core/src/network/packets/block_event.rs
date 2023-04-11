use util::{bail, BlockPosition, Deserialize, Error, Result, Serialize};
use util::bytes::{BinaryRead, BinaryWrite, MutableBuffer, SharedBuffer, size_of_varint};

use crate::network::ConnectedPacket;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum BlockEventType {
    ChangeChestState
}

impl TryFrom<i32> for BlockEventType {
    type Error = anyhow::Error;

    fn try_from(value: i32) -> anyhow::Result<Self> {
        Ok(match value {
            0 => Self::ChangeChestState,
            _ => bail!(Malformed, "Invalid block event type {value}")
        })
    }
}

#[derive(Debug, Clone)]
pub struct BlockEvent {
    pub position: BlockPosition,
    pub event_type: BlockEventType,
    pub event_data: i32,
}

impl ConnectedPacket for BlockEvent {
    const ID: u32 = 0x1a;

    fn serialized_size(&self) -> usize {
        size_of_varint(self.position.x) +
            size_of_varint(self.position.y) +
            size_of_varint(self.position.z) +
            size_of_varint(self.event_type as i32) +
            size_of_varint(self.event_data)
    }
}

impl Serialize for BlockEvent {
    fn serialize<W>(&self, buffer: W) -> anyhow::Result<()> where W: BinaryWrite {
        buffer.write_block_pos(&self.position)?;
        buffer.write_var_i32(self.event_type as i32)?;
        buffer.write_var_i32(self.event_data)
    }
}

impl Deserialize<'_> for BlockEvent {
    fn deserialize(mut buffer: SharedBuffer) -> anyhow::Result<Self> {
        let position = buffer.read_block_pos()?;
        let event_type = BlockEventType::try_from(buffer.read_var_i32()?)?;
        let event_data = buffer.read_var_i32()?;

        Ok(Self {
            position,
            event_type,
            event_data,
        })
    }
}