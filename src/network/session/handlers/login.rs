use std::collections::HashMap;
use std::num::NonZeroU64;
use std::sync::atomic::Ordering;

use bytes::{BufMut, BytesMut};
use jsonwebtoken::jwk::KeyOperations::Encrypt;

use crate::bail;
use crate::config::SERVER_CONFIG;
use crate::crypto::Encryptor;
use crate::error::{VErrorKind, VResult};
use crate::network::packets::{ChatRestrictionLevel, ClientCacheStatus, ClientToServerHandshake, Difficulty, Dimension, Disconnect, DISCONNECTED_LOGIN_FAILED, DISCONNECTED_NOT_AUTHENTICATED, GameMode, ItemEntry, Login, NETWORK_VERSION, NetworkSettings, PermissionLevel, PlayerMovementSettings, PlayerMovementType, PlayStatus, RequestNetworkSettings, ResourcePackClientResponse, ResourcePacksInfo, ResourcePackStack, ServerToClientHandshake, StartGame, Status, WorldGenerator};
use crate::network::raknet::{Frame, FrameBatch};
use crate::network::raknet::Reliability;
use crate::network::session::send_queue::SendPriority;
use crate::network::session::session::Session;
use crate::network::traits::{Decodable, Encodable};
use crate::util::{BlockPosition, Vector2f, Vector3f};

impl Session {
    /// Handles a [`ClientCacheStatus`] packet.
    pub fn handle_client_cache_status(&self, mut packet: BytesMut) -> VResult<()> {
        let request = ClientCacheStatus::decode(packet)?;
        // Unused

        Ok(())
    }

    pub fn handle_resource_pack_client_response(&self, mut packet: BytesMut) -> VResult<()> {
        let request = ResourcePackClientResponse::decode(packet)?;
        tracing::info!("{request:?}");

        self.kick("Hello, World!")?;
        return Ok(());

        // TODO: Implement resource packs.

        let start_game = StartGame {
            entity_id: 0,
            runtime_id: 0,
            gamemode: GameMode::Creative,
            position: Vector3f::from([0.0, 0.0, 0.0]),
            rotation: Vector2f::from([0.0, 0.0]),
            world_seed: 0,
            spawn_biome_type: 0,
            custom_biome_name: "plains".to_string(),
            dimension: Dimension::Overworld,
            generator: WorldGenerator::Flat,
            world_gamemode: GameMode::Creative,
            difficulty: Difficulty::Normal,
            world_spawn: BlockPosition::new(0, 0, 0),
            achievements_disabled: true,
            editor_world: false,
            day_cycle_lock_time: 0,
            education_offer: 0,
            education_features_enabled: false,
            education_production_id: "".to_string(),
            rain_level: 0.0,
            lightning_level: 0.0,
            confirmed_platform_locked_content: false,
            broadcast_to_lan: true,
            xbox_live_broadcast_mode: 0,
            platform_broadcast_mode: 0,
            enable_commands: true,
            texture_packs_required: false,
            gamerules: vec![],
            experiments: vec![],
            experiments_previously_enabled: false,
            bonus_chest_enabled: false,
            starter_map_enabled: false,
            permission_level: PermissionLevel::Visitor,
            server_chunk_tick_range: 0,
            has_locked_behavior_pack: false,
            has_locked_resource_pack: false,
            is_from_locked_world_template: false,
            use_msa_gamertags_only: false,
            is_from_world_template: false,
            is_world_template_option_locked: false,
            only_spawn_v1_villagers: false,
            persona_disabled: false,
            custom_skins_disabled: false,
            base_game_version: "1.19".to_string(),
            limited_world_width: 16,
            limited_world_height: 16,
            has_new_nether: false,
            force_experimental_gameplay: false,
            chat_restriction_level: ChatRestrictionLevel::None,
            disable_player_interactions: false,
            level_id: "".to_string(),
            level_name: "Vex Server".to_string(),
            template_content_identity: "".to_string(),
            is_trial: false,
            movement_settings: PlayerMovementSettings {
                movement_type: PlayerMovementType::ClientAuthoritative,
                rewind_history_size: 0,
                server_authoritative_breaking: false,
            },
            time: 0,
            enchantment_seed: 0,
            block_properties: vec![],
            item_properties: vec![
                ItemEntry {
                    name: "minecraft:stick".to_owned(),
                    runtime_id: 0,
                    component_based: false,
                }
            ],
            multiplayer_correlation_id: "".to_string(),
            server_authoritative_inventory: false,
            game_version: "1.19.60".to_string(),
            property_data: nbt::Value::Compound(HashMap::new()),
            server_block_state_checksum: 0,
            world_template_id: 0,
            client_side_generation: false,
        };
        self.send_packet(start_game)?;

        Ok(())
    }

    pub fn handle_client_to_server_handshake(&self, mut packet: BytesMut) -> VResult<()> {
        ClientToServerHandshake::decode(packet)?;

        let response = PlayStatus {
            status: Status::LoginSuccess,
        };
        self.send_packet(response)?;

        // TODO: Implement resource packs
        let pack_info = ResourcePacksInfo {
            required: false,
            scripting_enabled: false,
            forcing_server_packs: false,
            behavior_info: vec![],
            resource_info: vec![],
        };
        self.send_packet(pack_info)?;

        let pack_stack = ResourcePackStack {
            forced_to_accept: false,
            resource_packs: vec![],
            behavior_packs: vec![],
            game_version: "1.19".to_string(),
            experiments: vec![],
            experiments_previously_toggled: false,
        };
        self.send_packet(pack_stack)?;

        Ok(())
    }

    /// Handles a [`Login`] packet.
    pub async fn handle_login(&self, mut packet: BytesMut) -> VResult<()> {
        let request = Login::decode(packet);
        let request = match request {
            Ok(r) => r,
            Err(e) => {
                self.kick(DISCONNECTED_LOGIN_FAILED)?;
                return Err(e);
            }
        };
        tracing::info!("{} has joined the server", request.identity.display_name);

        let (encryptor, jwt) = Encryptor::new(&request.identity.public_key)?;

        self.identity.set(request.identity)?;
        self.user_data.set(request.user_data)?;

        // Flush packets before enabling encryption
        self.flush().await?;

        self.send_packet(ServerToClientHandshake { jwt: jwt.as_str() })?;
        self.encryptor.set(encryptor)?;

        Ok(())
    }

    /// Handles a [`RequestNetworkSettings`] packet.
    pub fn handle_request_network_settings(&self, mut packet: BytesMut) -> VResult<()> {
        let request = RequestNetworkSettings::decode(packet)?;
        if request.protocol_version != NETWORK_VERSION {
            if request.protocol_version > NETWORK_VERSION {
                let response = PlayStatus {
                    status: Status::FailedServer,
                };
                self.send_packet(response)?;

                bail!(
                    VersionMismatch,
                    "Client is using a newer protocol ({} vs. {})",
                    request.protocol_version,
                    NETWORK_VERSION
                );
            } else {
                let response = PlayStatus {
                    status: Status::FailedClient,
                };
                self.send_packet(response)?;

                bail!(
                    VersionMismatch,
                    "Client is using an older protocol ({} vs. {})",
                    request.protocol_version,
                    NETWORK_VERSION
                );
            }
        }

        let response = {
            let lock = SERVER_CONFIG.read();
            NetworkSettings {
                compression_algorithm: lock.compression_algorithm,
                compression_threshold: lock.compression_threshold,
                client_throttle: lock.client_throttle,
            }
        };

        self.send_packet(response)?;
        self.compression_enabled.store(true, Ordering::SeqCst);

        Ok(())
    }
}
