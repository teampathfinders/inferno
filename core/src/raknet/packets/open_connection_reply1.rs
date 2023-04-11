use std::io::Write;

use util::bytes::{BinaryWrite, MutableBuffer};
use util::Result;
use util::Serialize;

use crate::raknet::OFFLINE_MESSAGE_DATA;

/// Sent in response to [`OpenConnectionRequest1`](crate::open_connection_request1::OpenConnectionRequest1).
#[derive(Debug)]
pub struct OpenConnectionReply1 {
    /// GUID of the server.
    /// Corresponds to [`ServerInstance::guid`](crate::ServerInstance::guid).
    pub server_guid: u64,
    /// MTU of the connection.
    /// This should be given the same value as [`OpenConnectionRequest1::mtu`](crate::open_connection_request1::OpenConnectionRequest1::mtu).
    pub mtu: u16,
}

impl OpenConnectionReply1 {
    /// Unique identifier of this packet.
    pub const ID: u8 = 0x06;

    pub fn serialized_size(&self) -> usize {
        1 + 16 + 8 + 1 + 2
    }
}

impl Serialize for OpenConnectionReply1 {
    fn serialize<W>(&self, writer: W) -> anyhow::Result<()>
    where
        W: BinaryWrite,
    {
        writer.write_u8(Self::ID)?;
        writer.write_all(OFFLINE_MESSAGE_DATA)?;
        writer.write_u64_be(self.server_guid)?;
        // Disable security, required for login sequence.
        // Encryption will be enabled later on.
        writer.write_u8(0)?;
        writer.write_u16_be(self.mtu)
    }
}
