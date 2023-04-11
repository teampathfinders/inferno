use util::bytes::{BinaryWrite, MutableBuffer, VarString};
use util::Result;
use util::Serialize;

use crate::network::ConnectedPacket;

/// Sent by the server to initiate encryption.
/// The client responds with a [`ClientToServerHandshake`](crate::ClientToServerHandshake) to
/// indicate encryption has successfully been initiated.
#[derive(Debug, Clone)]
pub struct ServerToClientHandshake<'a> {
    /// Token containing the salt and public key.
    pub jwt: &'a str,
}

impl<'a> ConnectedPacket for ServerToClientHandshake<'a> {
    const ID: u32 = 0x03;

    fn serialized_size(&self) -> usize {
        self.jwt.var_len()
    }
}

impl<'a> Serialize for ServerToClientHandshake<'a> {
    fn serialize<W>(&self, buffer: W) -> anyhow::Result<()> where W: BinaryWrite {
        buffer.write_str(self.jwt)
    }
}
