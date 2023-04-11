use util::bytes::{BinaryWrite, MutableBuffer};
use util::Result;
use util::Serialize;

use crate::network::ConnectedPacket;

/// Supported compression algorithms.
///
/// Snappy is fast, but has produces lower compression ratios.
/// Flate is slow, but produces high compression ratios.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum CompressionAlgorithm {
    /// The Deflate/Zlib compression algorithm.
    Deflate,
    /// The Snappy compression algorithm.
    /// Available since Minecraft 1.19.30.
    Snappy,
}

/// Settings for client throttling.
///
/// If client throttling is enabled, the client will tick fewer players,
/// improving performance on low-end devices.
#[derive(Debug, Copy, Clone)]
pub struct ClientThrottleSettings {
    /// Regulates whether the client should throttle players.
    pub enabled: bool,
    /// Threshold for client throttling.
    /// If the number of players in the game exceeds this value, players will be throttled.
    pub threshold: u8,
    /// Amount of players that are ticked when throttling is enabled.
    pub scalar: f32,
}

/// Sent by the server to modify network related settings.
#[derive(Debug)]
pub struct NetworkSettings {
    /// Minimum size of a packet that is compressed.
    /// Any packets below this threshold will not be compressed.
    /// Settings this to 0 disables compression.
    pub compression_threshold: u16,
    /// Algorithm used to compress packets.
    pub compression_algorithm: CompressionAlgorithm,
    /// Client throttling settings.
    pub client_throttle: ClientThrottleSettings,
}

impl ConnectedPacket for NetworkSettings {
    /// Unique ID of this packet.
    const ID: u32 = 0x8f;

    fn serialized_size(&self) -> usize {
        2 + 2 + 1 + 1 + 4
    }
}

impl Serialize for NetworkSettings {
    fn serialize<W>(&self, buffer: W) -> anyhow::Result<()> where W: BinaryWrite {
        buffer.write_u16_be(self.compression_threshold)?;
        buffer.write_u16_be(self.compression_algorithm as u16)?;
        buffer.write_bool(self.client_throttle.enabled)?;
        buffer.write_u8(self.client_throttle.threshold)?;
        buffer.write_f32_be(self.client_throttle.scalar)
    }
}
