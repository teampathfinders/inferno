use util::{BinaryWrite};

use util::Serialize;

use crate::bedrock::ConnectedPacket;

/// Supported compression algorithms.
///
/// Snappy is fast, but has produces lower compression ratios.
/// Flate is slow, but produces high compression ratios.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum CompressionAlgorithm {
    /// The Deflate/Zlib compression algorithm.
    Flate,
    /// The Snappy compression algorithm.
    /// Available since Minecraft 1.19.30.
    /// 
    /// WARNING: This option is currently not support by the server.
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
    /// Any raknet below this threshold will not be compressed.
    /// Settings this to 0 disables compression.
    pub compression_threshold: u16,
    /// Algorithm used to compress raknet.
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
    fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_u16_be(self.compression_threshold)?;
        writer.write_u16_be(self.compression_algorithm as u16)?;
        writer.write_bool(self.client_throttle.enabled)?;
        writer.write_u8(self.client_throttle.threshold)?;
        writer.write_f32_be(self.client_throttle.scalar)
    }
}
