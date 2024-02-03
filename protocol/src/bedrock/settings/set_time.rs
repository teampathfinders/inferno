use util::{BinaryWrite, size_of_varint};

use util::Serialize;

use crate::bedrock::ConnectedPacket;

/// Sets the current time for the client.
#[derive(Debug, Clone)]
pub struct SetTime {
    /// Current time (in ticks)
    pub time: i32,
}

impl ConnectedPacket for SetTime {
    const ID: u32 = 0x0a;

    fn serialized_size(&self) -> usize {
        size_of_varint(self.time)
    }
}

impl Serialize for SetTime {
    fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_var_i32(self.time)
    }
}
