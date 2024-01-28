//! Server configuration

use std::time::Duration;

use parking_lot::RwLock;
use proto::bedrock::{ClientThrottleSettings, CompressionAlgorithm};

/// Global service that contains all configuration settings
pub struct ServerConfig {
    /// Port to bind the IPv4 socket to.
    pub ipv4_port: u16,
    /// Port to bind the IPv6 socket to.
    pub ipv6_port: u16,
    /// Max player count.
    pub max_players: usize,
    /// Compression algorithm to use (either Snappy or Deflate).
    pub compression_algorithm: CompressionAlgorithm,
    /// When a packet's size surpasses this threshold, it will be compressed.
    /// Set the threshold to 0 to disable compression.
    pub compression_threshold: u16,
    /// Client throttling settings.
    pub client_throttle: ClientThrottleSettings,
    /// Name of the server.
    /// This is only visible in LAN games.
    pub server_name: &'static str,
    /// Maximum render distance that the server will accept.
    /// Clients requesting a higher value will be told to use this.
    pub allowed_render_distance: i32,
    /// Interval between world autosaves.
    /// Set to 0 to disable autosaves.
    pub autosave_interval: Duration,
    /// Path to the world to host.
    pub level_path: &'static str,
}

pub static SERVER_CONFIG: RwLock<ServerConfig> = RwLock::new(ServerConfig {
    ipv4_port: 19132,
    ipv6_port: 19133,
    max_players: 1000,
    compression_algorithm: CompressionAlgorithm::Flate,
    compression_threshold: 1, // Compress all raknet
    client_throttle: ClientThrottleSettings {
        // Disable client throttling
        enabled: false,
        threshold: 0,
        scalar: 0.0,
    },
    server_name: "Pathfinders",
    allowed_render_distance: 16,
    autosave_interval: Duration::from_secs(60),
    level_path: "test-level",
});
