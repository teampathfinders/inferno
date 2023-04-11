use util::bytes::{BinaryRead, SharedBuffer};
use util::pyassert;
use util::Deserialize;
use util::Result;

/// Sent by the client, in response to [`OpenConnectionReply2`](crate::open_connection_reply2::OpenConnectionReply2).
#[derive(Debug)]
pub struct OpenConnectionRequest2 {
    /// MTU of the connection.
    pub mtu: u16,
    /// GUID of the client.
    pub client_guid: u64,
}

impl OpenConnectionRequest2 {
    /// Unique identifier of the packet.
    pub const ID: u8 = 0x07;
}

impl<'a> Deserialize<'a> for OpenConnectionRequest2 {
    fn deserialize<R>(mut reader: R) -> anyhow::Result<Self>
    where
        R: BinaryRead<'a> + 'a,
    {
        pyassert!(reader.read_u8()? == Self::ID);

        reader.advance(16); // Skip magic
        reader.read_addr()?; // Skip server address
        let mtu = reader.read_u16_be()?;
        let client_guid = reader.read_u64_be()?;

        Ok(Self { mtu, client_guid })
    }
}
