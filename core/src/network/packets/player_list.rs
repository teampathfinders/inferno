use uuid::Uuid;

use util::{Result, Serialize};
use util::bytes::{BinaryWrite, MutableBuffer, size_of_varint};

use crate::network::ConnectedPacket;
use crate::network::DeviceOS;
use crate::network::Skin;

#[derive(Debug, Clone)]
pub struct PlayerListAddEntry<'a> {
    /// UUID.
    pub uuid: Uuid,
    /// Unique entity ID.
    pub entity_id: i64,
    /// Username of the client.
    pub username: &'a str,
    /// XUID of the client.
    pub xuid: u64,
    /// Operating system of the client.
    pub device_os: DeviceOS,
    /// The client's skin.
    pub skin: &'a Skin,
    /// Whether the client is the host of the game.
    pub host: bool,
}

/// Adds player(s) to the client's player list.
///
/// This and [`PlayerListRemove`] are the same packet, but are separated here for optimisation reasons.
/// This separation allows the server to remove players from the player list without having to copy over all the player data
/// contained in [`PlayerListAddEntry`].
#[derive(Debug, Clone)]
pub struct PlayerListAdd<'a> {
    pub entries: &'a [PlayerListAddEntry<'a>],
}

impl<'a> ConnectedPacket for PlayerListAdd<'a> {
    const ID: u32 = 0x3f;
}

impl<'a> Serialize for PlayerListAdd<'a> {
    fn serialize<W>(&self, writer: W) -> anyhow::Result<()>
    where
        W: BinaryWrite
    {
        writer.write_u8(0)?; // Add player.
        writer.write_var_u32(self.entries.len() as u32)?;
        for entry in self.entries {
            writer.write_uuid_le(&entry.uuid)?;

            writer.write_var_i64(entry.entity_id)?;
            writer.write_str(entry.username)?;
            writer.write_str(&entry.xuid.to_string())?;
            writer.write_str("")?; // Platform chat ID.
            writer.write_i32_le(entry.device_os as i32)?;
            entry.skin.serialize(writer)?;
            writer.write_bool(false)?; // Player is not a teacher.
            writer.write_bool(entry.host)?;
        }

        for entry in self.entries {
            writer.write_bool(entry.skin.is_trusted)?;
        }

        Ok(())
    }
}

/// Removes player(s) from the client's player list.
#[derive(Debug, Clone)]
pub struct PlayerListRemove<'a> {
    pub entries: &'a [Uuid],
}

impl<'a> ConnectedPacket for PlayerListRemove<'a> {
    const ID: u32 = 0x3f;

    fn serialized_size(&self) -> usize {
        1 + size_of_varint(self.entries.len() as u32) + 16 * self.entries.len()
    }
}

impl<'a> Serialize for PlayerListRemove<'a> {
    fn serialize<W>(&self, writer: W) -> anyhow::Result<()>
    where
        W: BinaryWrite
    {
        writer.write_u8(1)?; // Remove player.
        writer.write_var_u32(self.entries.len() as u32)?;
        for entry in self.entries {
            writer.write_uuid_le(entry)?;
        }

        Ok(())
    }
}
