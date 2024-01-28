use std::net::SocketAddr;

use raknet::CONNECTED_PEER_BIT_FLAG;

/// An unprocessed packet.
pub struct ForwardablePacket {
    pub buf: Vec<u8>,
    pub addr: SocketAddr,
}

impl ForwardablePacket {
    /// Checks whether this frame is encapsulated in a [`Frame`](crate::raknet::Frame).
    #[inline]
    pub fn is_unconnected(&self) -> bool {
        self.buf
            .first()
            .map_or(false, |f| f & CONNECTED_PEER_BIT_FLAG == 0)
    }

    /// Returns the ID of this packet.
    ///
    /// If the packet is encapsulated, this will always return a frame ID in the range 0x80 to 0x8d.
    /// When the packet is not encapsulated, the actual packet ID will be used.
    ///
    /// So this should generally only be used for raknet that are not encapsulated.
    #[inline]
    pub fn packet_id(&self) -> Option<u8> {
        self.buf.first().copied()
    }
}
