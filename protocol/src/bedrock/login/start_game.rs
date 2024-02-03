use std::collections::HashMap;

use crate::types::Dimension;
use macros::variant_count;
use util::{Serialize, Vector};

use util::BlockPosition;
use util::{BinaryWrite, VarInt, VarString};

use crate::bedrock::{CLIENT_VERSION_STRING, ConnectedPacket, Difficulty, GameMode, GameRule};
use crate::bedrock::ExperimentData;

const MULTIPLAYER_CORRELATION_ID: &str = "5b39a9d6-f1a1-411a-b749-b30742f81771";

/// Which world generator type the server is using.
#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum WorldGenerator {
    OldLimited,
    Infinite,
    Flat,
    Nether,
    End,
}

impl WorldGenerator {
    /// Serializes the enum.
    pub fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_var_i32(*self as i32)
    }
}

/// The permission level of the client.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
#[variant_count]
pub enum PermissionLevel {
    /// A visitor is a player that has most permissions removed.
    Visitor,
    /// A member is the default permission level for players.
    Member,
    /// An operator is a player that is allowed to run commands.
    Operator,
    /// A combination of any of the above.
    Custom,
}

impl TryFrom<u8> for PermissionLevel {
    type Error = anyhow::Error;

    fn try_from(value: u8) -> anyhow::Result<PermissionLevel> {
        if value <= PermissionLevel::variant_count() as u8 {
            // SAFETY: This is safe because the discriminant is in range and
            // the representations are the same. Additionally, none of the enum members
            // have a manually assigned value (this is ensured by the `variant_count` macro).
            Ok(unsafe {
                std::mem::transmute::<u8, PermissionLevel>(value)
            })
        } else {
            anyhow::bail!("Permission level out of range <= 3, got {value}")
        }
    }
}

#[derive(Debug, Clone)]
pub struct EducationResourceURI {
    pub button_name: String,
    pub link_uri: String,
}

impl EducationResourceURI {
    /// Serializes the struct.
    pub fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_str(&self.button_name)?;
        writer.write_str(&self.link_uri)
    }
}

/// How restricted chat is.
#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum ChatRestrictionLevel {
    /// Chat has no restrictions, this is how Minecraft normally works.
    None,
    /// Same as `Disabled`.
    Dropped,
    /// Prevents the user from sending any chat messages and displays `Chat is disabled`.
    Disabled,
}

impl ChatRestrictionLevel {
    /// Serializes the enum.
    pub fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_u8(*self as u8)
    }
}

/// Determines how player movement is handled.
#[derive(Debug, Copy, Clone)]
#[repr(i32)]
pub enum PlayerMovementType {
    /// The client has full control over its movement.
    ClientAuthoritative,
    /// The server needs to authorise player movement.
    ServerAuthoritative,
    /// Same as `ServerAuthoritative` but adds the ability to rewind player movement.
    ServerAuthoritativeWithRewind,
}

/// Sets the player movement settings.
#[derive(Debug, Copy, Clone)]
pub struct PlayerMovementSettings {
    /// See [`PlayerMovementType`].
    pub movement_type: PlayerMovementType,
    /// Determines how far back a rewind can go.
    pub rewind_history_size: i32,
    /// Whether the server authorises block breaking.
    pub server_authoritative_breaking: bool,
}

impl PlayerMovementSettings {
    /// Returns the serialized size of the struct.
    pub fn serialized_size(&self) -> usize {
        (self.movement_type as i32).var_len() +
            self.rewind_history_size.var_len() +
            1
    }

    /// Serializes the struct.
    pub fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_var_i32(self.movement_type as i32)?;
        writer.write_var_i32(self.rewind_history_size)?;
        writer.write_bool(self.server_authoritative_breaking)
    }
}

#[derive(Debug, Clone)]
pub struct BlockEntry {
    /// Name of the block.
    pub name: String,
    // NBT compound containing properties.
    pub properties: HashMap<String, nbt::Value>,
}

impl BlockEntry {
    pub const fn serialized_size(&self) -> usize {
        1
        // self.name.var_len() //+ self.properties.serialized_net_size("")
    }
}

impl Serialize for BlockEntry {
    fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        // {
        //     let mut buf2 = Vec::new();
        //     buf2.write_str(&self.name)?;
        //     nbt::to_var_bytes_in(&mut buf2, &self.properties)?;

        //     let mut ss: &mut &mut [u8] = &mut buf2.as_mut();
        //     let name = ss.read_str()?;
        //     let (properties, _): (nbt::Value, usize) = nbt::from_var_bytes(*ss)?;

        //     dbg!(name, properties);
        // }

        writer.write_str(&self.name)?;
        nbt::to_var_bytes_in(writer, &self.properties)
    }
}

#[derive(Debug, Clone)]
pub struct ItemEntry {
    /// Name of the item.
    pub name: String,
    /// Runtime ID of the item.
    /// This ID is what Minecraft uses to refer to the item.
    pub runtime_id: u16,
    /// Whether this is a custom item.
    pub component_based: bool,
}

impl ItemEntry {
    pub fn serialized_size(&self) -> usize {
        self.name.var_len() + 2 + 1
    }

    pub fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_str(&self.name)?;
        writer.write_u16_le(self.runtime_id)?;
        writer.write_bool(self.component_based)
    }
}

#[derive(Debug, Copy, Clone, serde::Serialize)]
#[serde(rename = "")]
pub struct PropertyData {}

#[derive(Debug, Copy, Clone)]
pub enum SpawnBiomeType {
    Default,
    Custom,
}

#[derive(Debug, Copy, Clone)]
#[repr(u32)]
pub enum BroadcastIntent {
    NoMultiplayer,
    InviteOnly,
    FriendsOnly,
    FriendsOfFriends,
    Public,
}

impl BroadcastIntent {
    pub fn serialize_into<W: BinaryWrite>(&self, writer: &mut W) -> anyhow::Result<()> {
        writer.write_var_u32(*self as u32)
    }
}

/// The start game packet contains most of the world settings displayed in the settings menu.
#[derive(Debug, Clone)]
pub struct StartGame<'a> {
    pub entity_id: i64,
    /// Runtime ID of the client.
    pub runtime_id: u64,
    /// Current game mode of the client.
    /// This is not the same as the world game mode.
    pub game_mode: GameMode,
    /// Spawn position.
    pub position: Vector<f32, 3>,
    /// Spawn rotation.
    pub rotation: Vector<f32, 2>,
    /// World seed.
    /// This is displayed in the settings menu.
    pub world_seed: u64,
    pub spawn_biome_type: SpawnBiomeType,
    pub custom_biome_name: &'a str,
    /// Dimension the client spawns in.
    pub dimension: Dimension,
    /// Generator used to create the world.
    /// This is also displayed in the settings menu.
    pub generator: WorldGenerator,
    /// World game mode.
    /// The default game mode for new players.
    pub world_game_mode: GameMode,
    /// Difficulty of the game.
    pub difficulty: Difficulty,
    /// Default spawn position.
    pub world_spawn: BlockPosition,
    /// Whether achievements are disabled.
    /// This should generally be set to true for servers.
    ///
    /// According to wiki.vg, the client crashes if both achievements and commands are enabled.
    /// I couldn't reproduce this on Windows 10.
    pub achievements_disabled: bool,
    /// Whether the world is in editor mode.
    pub editor_world: bool,
    /// The time to which the daylight cycle is locked if the gamerule is set.
    pub day_cycle_lock_time: i32,
    /// Whether education edition features are enabled.
    pub education_features_enabled: bool,
    /// The intensity of the current rain.
    /// Set to 0 for no rain.
    pub rain_level: f32,
    /// The intensity of the current thunderstorm.
    /// Set to 0 to for no thunderstorm.
    pub lightning_level: f32,
    pub confirmed_platform_locked_content: bool,
    /// Whether to broadcast to LAN.
    pub broadcast_to_lan: bool,
    pub xbox_broadcast_intent: BroadcastIntent,
    pub platform_broadcast_intent: BroadcastIntent,
    /// Whether to enable commands.
    /// If this is disabled, the client will not allow the player to send commands under any
    /// circumstance.
    pub enable_commands: bool,
    /// Whether texture packs are required.
    /// This doesn't really seem to have a function other than displaying the force pack setting in the settings menu.
    ///
    /// Whether packs are actually required has already been specified in the resource pack raknet.
    pub texture_packs_required: bool,
    /// List of game rules.
    /// Only modified game rules have to be sent.
    /// Game rules that are not sent in the start game packet, will be set to their default values.
    pub game_rules: &'a [GameRule],
    /// Experiments used by the server.
    /// This is a visual option, since the experiments have already been specified in a resource pack packet.
    pub experiments: &'a [ExperimentData<'a>],
    /// Whether experiments have previously been enabled.
    pub experiments_previously_enabled: bool,
    /// Whether the bonus chest is enabled.
    /// This is only a visual thing shown in the settings menu.
    pub bonus_chest_enabled: bool,
    /// Whether the starter map is enabled.
    /// This generally should be left disabled as the client will otherwise force itself to have a map in its inventory.
    pub starter_map_enabled: bool,
    /// Permission level of the client: visitor, member, operator or custom.
    pub permission_level: PermissionLevel,
    /// Simulation distance of the server.
    /// This is only a visual thing shown in the settings menu.
    pub server_chunk_tick_range: i32,

    pub has_locked_behavior_pack: bool,
    pub has_locked_resource_pack: bool,
    pub is_from_locked_world_template: bool,
    pub use_msa_gamertags_only: bool,
    pub is_from_world_template: bool,
    pub is_world_template_option_locked: bool,
    pub only_spawn_v1_villagers: bool,
    pub persona_disabled: bool,
    pub custom_skins_disabled: bool,
    pub emote_chat_muted: bool,
    /// Version of the game from which vanilla features will be used.
    // pub base_game_version: String,
    pub limited_world_width: i32,
    pub limited_world_height: i32,
    // pub has_new_nether: bool,
    pub force_experimental_gameplay: bool,
    pub chat_restriction_level: ChatRestrictionLevel,
    pub disable_player_interactions: bool,
    pub level_id: &'a str,
    /// Name of the world.
    /// This is shown in the pause menu above the player list, and the settings menu.
    pub level_name: &'a str,
    pub template_content_identity: &'a str,
    pub movement_settings: PlayerMovementSettings,
    /// Current time.
    pub time: i64,
    pub enchantment_seed: i32,
    pub block_properties: &'a [BlockEntry],
    pub item_properties: &'a [ItemEntry],
    pub property_data: PropertyData,
    /// Whether inventory transactions are server authoritative.
    pub server_authoritative_inventory: bool,
    /// Version of the game that the server is running.
    pub game_version: &'a str,
    // pub property_data: nbt::Value,
    pub server_block_state_checksum: u64,
    pub world_template_id: u128,
    /// Client side generation allows the client to generate its own chunks without the server having to send them over.
    pub client_side_generation: bool,
    pub hashed_block_ids: bool,
    pub server_authoritative_sounds: bool
}

impl ConnectedPacket for StartGame<'_> {
    const ID: u32 = 0x0b;

    fn serialized_size(&self) -> usize {
        self.entity_id.var_len() +
            self.runtime_id.var_len() +
            (self.game_mode as i32).var_len() +
            3 * 4 +
            3 * 4 +
            8 +
            2 +
            self.custom_biome_name.var_len() +
            (self.dimension as u32).var_len() +
            (self.generator as i32).var_len() +
            (self.world_game_mode as i32).var_len() +
            (self.difficulty as i32).var_len() +
            self.world_spawn.serialized_size() +
            1 +
            1 +
            self.day_cycle_lock_time.var_len() +
            0.var_len() +
            1 +
            "".var_len() +
            4 +
            4 +
            1 +
            1 +
            1 +
            (self.xbox_broadcast_intent as u32).var_len() +
            (self.platform_broadcast_intent as u32).var_len() +
            1 +
            1 +
            (self.game_rules.len() as u32).var_len() +
            self.game_rules.iter().fold(0, |acc, r| acc + r.serialized_size()) +
            4 +
            self.experiments.iter().fold(0, |acc, e| acc + e.serialized_size()) +
            1 +
            1 +
            1 +
            (self.permission_level as i32).var_len() +
            4 +
            1 +
            1 +
            1 +
            1 +
            1 +
            1 +
            1 +
            1 +
            1 +
            1 +
            CLIENT_VERSION_STRING.var_len() +
            4 +
            4 +
            1 +
            "".var_len() +
            "".var_len() +
            1 +
            1 +
            1 +
            self.level_id.var_len() +
            self.level_name.var_len() +
            self.template_content_identity.var_len() +
            1 +
            self.movement_settings.serialized_size() +
            8 +
            self.enchantment_seed.var_len() +
            (self.block_properties.len() as u32).var_len() +
            self.block_properties.iter().fold(0, |acc, p| acc + p.serialized_size()) +
            (self.item_properties.len() as u32).var_len() +
            self.item_properties.iter().fold(0, |acc, p| acc + p.serialized_size()) +
            MULTIPLAYER_CORRELATION_ID.var_len() +
            1 +
            CLIENT_VERSION_STRING.var_len() +
            // self.property_data.serialized_net_size("") +
            8 +
            16 +
            1 +
            1 +
            1 +
            1 +
            1
    }
}

impl<'a> Serialize for StartGame<'a> {
    fn serialize_into<W: BinaryWrite>(&self, mut writer: &mut W) -> anyhow::Result<()> {
        writer.write_var_i64(self.entity_id)?;
        writer.write_var_u64(self.runtime_id)?;
        writer.write_var_i32(self.game_mode as i32)?;
        writer.write_vecf(&self.position)?;
        writer.write_vecf(&self.rotation)?;
        writer.write_u64_le(self.world_seed)?;
        writer.write_i16_le(self.spawn_biome_type as i16)?;
        writer.write_str(self.custom_biome_name)?;
        writer.write_var_u32(self.dimension as u32)?;
        writer.write_var_i32(self.generator as i32)?;
        writer.write_var_i32(self.world_game_mode as i32)?;
        writer.write_var_i32(self.difficulty as i32)?;
        writer.write_block_pos(&self.world_spawn)?;

        writer.write_bool(self.achievements_disabled)?;
        writer.write_bool(self.editor_world)?;
        writer.write_bool(false)?; // Not created in editor
        writer.write_bool(false)?; // Not exported from editor
        writer.write_var_i32(self.day_cycle_lock_time)?;
        writer.write_var_i32(0)?; // Education offer.
        writer.write_bool(self.education_features_enabled)?;
        writer.write_str("")?; // Education product ID.
        writer.write_f32_le(self.rain_level)?;
        writer.write_f32_le(self.lightning_level)?;
        writer.write_bool(self.confirmed_platform_locked_content)?;
        writer.write_bool(true)?; // Whether the game is multiplayer, must always be true for servers.
        writer.write_bool(self.broadcast_to_lan)?;
        self.xbox_broadcast_intent.serialize_into(writer)?;
        self.platform_broadcast_intent.serialize_into(writer)?;
        writer.write_bool(self.enable_commands)?;
        writer.write_bool(self.texture_packs_required)?;

        writer.write_var_u32(self.game_rules.len() as u32)?;
        for rule in self.game_rules {
            rule.serialize_into(writer)?;
        }

        writer.write_u32_le(self.experiments.len() as u32)?;
        for experiment in self.experiments {
            experiment.serialize_into(writer)?;
        }

        writer.write_bool(self.experiments_previously_enabled)?;
        writer.write_bool(self.bonus_chest_enabled)?;
        writer.write_bool(self.starter_map_enabled)?;
        writer.write_var_i32(self.permission_level as i32)?;
        writer.write_i32_le(self.server_chunk_tick_range)?;
        writer.write_bool(self.has_locked_behavior_pack)?;
        writer.write_bool(self.has_locked_resource_pack)?;
        writer.write_bool(self.is_from_locked_world_template)?;
        writer.write_bool(self.use_msa_gamertags_only)?;
        writer.write_bool(self.is_from_world_template)?;
        writer.write_bool(self.is_world_template_option_locked)?;
        writer.write_bool(self.only_spawn_v1_villagers)?;
        writer.write_bool(self.persona_disabled)?;
        writer.write_bool(self.custom_skins_disabled)?;
        writer.write_bool(self.emote_chat_muted)?;
        writer.write_str(CLIENT_VERSION_STRING)?; // Base game version
        writer.write_i32_le(self.limited_world_width)?;
        writer.write_i32_le(self.limited_world_height)?;
        writer.write_bool(true)?; // Use new nether
        writer.write_str("")?;
        writer.write_str("")?;
        writer.write_bool(self.force_experimental_gameplay)?;
        self.chat_restriction_level.serialize_into(writer)?;
        writer.write_bool(self.disable_player_interactions)?;
        writer.write_str(self.level_id)?;
        writer.write_str(self.level_name)?;
        writer.write_str(self.template_content_identity)?;
        writer.write_bool(false)?; // Game is not a trial.
        self.movement_settings.serialize_into(writer)?;
        writer.write_i64_le(self.time)?;
        writer.write_var_i32(self.enchantment_seed)?;

        writer.write_var_u32(self.block_properties.len() as u32)?;
        for block in self.block_properties {
            block.serialize_into(writer)?;
        }

        writer.write_var_u32(self.item_properties.len() as u32)?;
        for item in self.item_properties {
            item.serialize_into(writer)?;
        }

        // Random multiplayer correlation UUID.
        writer.write_str(MULTIPLAYER_CORRELATION_ID)?;

        writer.write_bool(self.server_authoritative_inventory)?;
        writer.write_str(CLIENT_VERSION_STRING)?; // Game version

        nbt::to_var_bytes_in(&mut writer, &self.property_data)?;

        writer.write_u64_le(self.server_block_state_checksum)?;
        writer.write_u128_le(self.world_template_id)?;
        writer.write_bool(self.client_side_generation)?;
        writer.write_bool(self.hashed_block_ids)?;
        writer.write_bool(self.server_authoritative_sounds)
    }
}
