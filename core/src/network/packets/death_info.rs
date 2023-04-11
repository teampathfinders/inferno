use util::bytes::{BinaryWrite, MutableBuffer, size_of_varint};
use util::Result;
use util::Serialize;

use crate::network::ConnectedPacket;

/// Information about a player's death.
#[derive(Debug, Clone)]
pub struct DeathInfo<'a> {
    /// Cause of death.
    pub cause: &'a str,
    /// Additional info display in the death screen.
    pub messages: &'a [&'a str],
}

impl<'a> ConnectedPacket for DeathInfo<'a> {
    const ID: u32 = 0xbd;

    fn serialized_size(&self) -> usize {
        size_of_varint(self.cause.len() as u32) + self.cause.len() +
            size_of_varint(self.messages.len() as u32) +
            self.messages.iter().fold(
                0, |acc, m| acc + size_of_varint(m.len() as u32) + m.len(),
            )
    }
}

impl<'a> Serialize for DeathInfo<'a> {
    fn serialize<W>(&self, writer: W) -> anyhow::Result<()>
    where
        W: BinaryWrite
    {
        writer.write_str(self.cause)?;

        writer.write_var_u32(self.messages.len() as u32)?;
        for message in self.messages {
            writer.write_str(message)?;
        }

        Ok(())
    }
}
