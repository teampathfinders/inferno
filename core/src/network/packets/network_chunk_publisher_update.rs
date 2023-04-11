use util::{BlockPosition, Result, Vector};
use util::bytes::{BinaryWrite, MutableBuffer, size_of_varint};
use util::Serialize;

use crate::network::ConnectedPacket;

#[derive(Debug, Clone)]
pub struct NetworkChunkPublisherUpdate {
    pub position: Vector<i32, 3>,
    pub radius: u32,
}

impl ConnectedPacket for NetworkChunkPublisherUpdate {
    const ID: u32 = 0x79;

    fn serialized_size(&self) -> usize {
        size_of_varint(self.position.x) +
            size_of_varint(self.position.y) +
            size_of_varint(self.position.z) +
            size_of_varint(self.radius) + 4
    }
}

impl Serialize for NetworkChunkPublisherUpdate {
    fn serialize<W>(&self, writer: W) -> anyhow::Result<()>
    where
        W: BinaryWrite
    {
        writer.write_veci(&self.position)?;
        writer.write_var_u32(self.radius)?;

        // No saved chunks.
        writer.write_u32_be(0)
    }
}
