use util::bytes::{BinaryWrite, MutableBuffer, size_of_varint};
use util::Result;
use util::Serialize;

use crate::network::ConnectedPacket;

/// Action to perform on the dynamic enum.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum SoftEnumAction {
    Add,
    Remove,
    Set,
}

/// Updates command autocompletion entries.
#[derive(Debug, Clone)]
pub struct UpdateDynamicEnum<'a> {
    /// ID of the enum, previously specified in [`CommandEnum::enum_id`](crate::CommandEnum::enum_id).
    pub enum_id: &'a str,
    /// List of enum options.
    pub options: &'a [String],
    /// Action to perform on the dynamic enum.
    pub action: SoftEnumAction,
}

impl ConnectedPacket for UpdateDynamicEnum<'_> {
    const ID: u32 = 0x72;

    fn serialized_size(&self) -> usize {
        size_of_varint(self.enum_id.len() as u32) + self.enum_id.len() +
            size_of_varint(self.options.len() as u32) +
            self.options.iter().fold(
                0, |acc, o| acc + size_of_varint(o.len() as u32) + o.len(),
            ) + 1
    }
}

impl Serialize for UpdateDynamicEnum<'_> {
    fn serialize<W>(&self, buffer: W) -> anyhow::Result<()> where W: BinaryWrite {
        buffer.write_str(self.enum_id)?;
        buffer.write_var_u32(self.options.len() as u32)?;
        for option in self.options {
            buffer.write_str(option)?;
        }
        buffer.write_u8(self.action as u8)
    }
}
