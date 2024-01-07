use util::{BinaryRead, SharedBuffer};
use util::Deserialize;
use util::Result;

use crate::bedrock::ConnectedPacket;

/// Sent during login to let the server know whether the client supports caching.
#[derive(Debug, Clone)]
pub struct CacheStatus {
    /// Whether the client supports the client-side blob cache.
    pub supports_cache: bool,
}

impl ConnectedPacket for CacheStatus {
    const ID: u32 = 0x81;
}

impl Deserialize<'_> for CacheStatus {
    fn deserialize(mut buffer: SharedBuffer) -> anyhow::Result<Self> {
        let support = buffer.read_bool()?;

        Ok(Self { supports_cache: support })
    }
}