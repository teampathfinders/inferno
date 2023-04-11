use util::{Result, Serialize, Deserialize};
use util::bytes::{BinaryWrite, MutableBuffer, SharedBuffer, BinaryRead};

use crate::network::ConnectedPacket;

#[derive(Default, Debug, Clone)]
pub struct ContainerClose {
    /// Equal to the window ID sent in the [`ContainerOpen`](crate::ContainerOpen) packet.
    pub window_id: u8,
    /// Whether the server force-closed the container.
    pub server_initiated: bool,
}

impl ConnectedPacket for ContainerClose {
    const ID: u32 = 0x2f;

    fn serialized_size(&self) -> usize {
        2
    }
}

impl<'a> Deserialize<'a> for ContainerClose {
    fn deserialize<R>(reader: R) -> anyhow::Result<Self>
    where
        R: BinaryRead<'a> + 'a
    {
        let window_id = reader.read_u8()?;
        let server_initiated = reader.read_bool()?;

        Ok(Self {
            window_id, server_initiated
        })
    }
}

impl Serialize for ContainerClose {
    fn serialize<W>(&self, buffer: W) -> anyhow::Result<()>
    where
        W: BinaryWrite
    {
        buffer.write_u8(self.window_id)?;
        buffer.write_bool(self.server_initiated)
    }
}