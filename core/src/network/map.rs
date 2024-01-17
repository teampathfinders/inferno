use std::net::SocketAddr;

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use anyhow::Context;
use dashmap::DashMap;

use raknet::{RaknetUser, BroadcastPacket, UserCreateInfo};

use tokio::sync::{broadcast, mpsc};
use proto::bedrock::{ConnectedPacket, Disconnect};
use replicator::Replicator;


use tokio::task::JoinSet;
use util::Serialize;
use util::MutableBuffer;

use crate::config::SERVER_CONFIG;
use crate::level::Level;

use super::{ForwardablePacket, BedrockUser};

const BROADCAST_CHANNEL_CAPACITY: usize = 5;
const FORWARD_TIMEOUT: Duration = Duration::from_millis(10);

pub struct ChannelUser<T> {
    channel: mpsc::Sender<MutableBuffer>,
    state: Arc<T>
}

impl<T> ChannelUser<T> {
    #[inline]
    pub async fn forward(&self, packet: MutableBuffer) -> anyhow::Result<()> {
        self.channel.send_timeout(packet, FORWARD_TIMEOUT).await.context("Server-side client timed out")?;
        Ok(())
    }
}

pub struct UserMap {
    replicator: Arc<Replicator>,
    level: OnceLock<Arc<Level>>,

    connecting_map: Arc<DashMap<SocketAddr, ChannelUser<RaknetUser>>>,
    connected_map: Arc<DashMap<SocketAddr, ChannelUser<BedrockUser>>>,
    /// Channel that sends a packet to all connected sessions.
    broadcast: broadcast::Sender<BroadcastPacket>
}

impl UserMap {
    pub fn new(replicator: Arc<Replicator>) -> Self {
        let connecting_map = Arc::new(DashMap::new());
        let connected_map = Arc::new(DashMap::new());

        let (broadcast, _) = broadcast::channel(BROADCAST_CHANNEL_CAPACITY);

        Self {
            replicator, connecting_map, connected_map, broadcast, level: OnceLock::new()
        }
    }

    pub fn set_level(&self, level: Arc<Level>) {
        if self.level.set(level).is_err() {
            tracing::error!("Level reference was already set");
        }
    }

    pub fn insert(&self, info: UserCreateInfo) {
        let (tx, rx) = mpsc::channel(BROADCAST_CHANNEL_CAPACITY);

        let address = info.address;
        let (state, state_rx) = 
            RaknetUser::new(info, self.broadcast.clone(), rx);
        
        let connecting_map = self.connecting_map.clone();
        let connected_map = self.connected_map.clone();
        let level = self.level.get().unwrap().clone();
        let replicator = self.replicator.clone();
        let broadcast = self.broadcast.clone();

        // Callback to move the client from the connecting map to the connected map.
        // This is done when the Raknet layer attempts to send a message to the Bedrock layer
        // signalling that the Raknet connection is fully set up.
        tokio::spawn(async move {
            if let Some((_, raknet_user)) = connecting_map.remove(&address) {
                let bedrock_user = ChannelUser {
                    channel: raknet_user.channel, state: BedrockUser::new(
                        raknet_user.state, level, replicator, state_rx, broadcast
                    )
                };

                connected_map.insert(address, bedrock_user);
            } else {
                tracing::error!("Raknet client exists but is not tracked by user map");
            }
        });

        let connecting_map = self.connecting_map.clone();
        let connected_map = self.connected_map.clone();
        let state_clone = state.clone();
        tokio::spawn(async move {
            state_clone.active.cancelled().await;
            connected_map.remove(&state_clone.address);
            connecting_map.remove(&state_clone.address);
        });

        self.connecting_map.insert(address, ChannelUser {
            channel: tx, state
        });
    }

    pub async fn forward(&self, packet: ForwardablePacket) -> anyhow::Result<()> {
        if let Some(user) = self.connected_map.get(&packet.addr) {
            return user.channel.send_timeout(packet.buf, FORWARD_TIMEOUT)
                .await
                .context("Forwarding packet to user timed out")
        }

        if let Some(user) = self.connecting_map.get(&packet.addr) {
            return user.channel.send_timeout(packet.buf, FORWARD_TIMEOUT)
                .await
                .context("Forwarding packet to connecting user timed out")
        }

        Ok(())
    }

    pub fn broadcast<T: ConnectedPacket + Serialize>(&self, _packet: T) -> anyhow::Result<()> {
        todo!()
    }

    /// Sends a [`Disconnect`] packet to every connected user.
    /// 
    /// This does not wait for the users to actually be disconnected.
    ///
    /// # Errors
    /// 
    /// This function returns an error when the [`Disconnect`] packet fails to serialize.
    pub fn kick_all(&self, message: &str) -> anyhow::Result<()> {
        // Ignore result because it can only fail if there are no receivers remaining.
        // In that case this shouldn't do anything anyways.
        self.broadcast.send(BroadcastPacket::new(
            Disconnect {
                hide_message: false,
                message,
            },
            None,
        )?)?;

        // Cancel all tokens.
        self.connected_map
            .iter()
            .for_each(|user| user.value().state.raknet.active.cancel());

        Ok(())
    }

    pub async fn shutdown(&self) -> anyhow::Result<()> {
        self.kick_all("disconnect.kicked")?;

        let mut join_set = JoinSet::new();
        self.connected_map.retain(|_k, v| {
            let handle = v.state.handle.write().take().unwrap();
            join_set.spawn(handle);

            false
        });

        while join_set.join_next().await.is_some() {}
        
        tracing::info!("All clients disconnected");
        Ok(())
    }

    pub fn count(&self) -> usize {
        self.connected_map.len()
    }

    pub fn max_count(&self) -> usize {
        SERVER_CONFIG.read().max_players
    }
}

//     /// Creates a new session and adds it to the tracker.
//     pub fn add_session(
//         self: &Arc<Self>,
//         ipv4_socket: Arc<UdpSocket>,
//         address: SocketAddr,
//         mtu: u16,
//         client_guid: u64,
//     ) {
//         let (sender, receiver) = mpsc::channel(BROADCAST_CHANNEL_CAPACITY);

//         let level_manager =
//             self.level_manager.get().unwrap().upgrade().unwrap();

//         let replicator = self.replicator.clone();
//         let session = Session::new(
//             self.broadcast.clone(),
//             receiver,
//             level_manager,
//             replicator,
//             ipv4_socket,
//             address,
//             mtu,
//             client_guid,
//         );

//         // Lightweight task that removes the session from the list when it is no longer active.
//         // This prevents cyclic references.
//         {
//             let list = self.list.clone();
//             let session = session.clone();

//             tokio::spawn(async move {
//                 session.cancelled().await;
//                 list.remove(&session.raknet.address);
//             });
//         }

//         self.list.insert(address, (sender, session));
//     }

//     #[inline]
//     pub fn set_level_manager(
//         &self,
//         level_manager: Weak<LevelManager>,
//     ) -> anyhow::Result<()> {
//         self.level_manager.set(level_manager)?;
//         Ok(())
//     }

//     /// Forwards a packet from the network service to the correct session.
//     pub fn forward_packet(&self, packet: RawPacket) {
//         // Spawn a new task so that the UDP receiver isn't interrupted
//         // if forwarding takes a long amount time.

//         let list = self.list.clone();
//         tokio::spawn(async move {
//             if let Some(session) = list.get(&packet.addr) {
//                 match session.0.send_timeout(packet.buf, FORWARD_TIMEOUT).await {
//                     Ok(_) => (),
//                     Err(_) => {
//                         // Session incoming queue is full.
//                         // If after a 20 ms timeout it is still full, destroy the session,
//                         // it probably froze.
//                         tracing::error!(
//                             "Closing hanging session"
//                         );

//                         // Attempt to send a disconnect packet.
//                         let _ = session.1.kick(DISCONNECTED_TIMEOUT);
//                         // Then close the session.
//                         session.1.on_disconnect();
//                     }
//                 }
//             }
//         });
//     }

//     /// Sends the given packet to every session.
//     pub fn broadcast<P: ConnectedPacket + Serialize + Clone>(
//         &self,
//         packet: P,
//     ) -> anyhow::Result<()> {
//         self.broadcast.send(BroadcastPacket::new(packet, None)?)?;
//         Ok(())
//     }

//     /// Kicks all sessions from the server, displaying the given message.
//     /// This function also waits for all sessions to be destroyed.
//     pub async fn kick_all<S: AsRef<str>>(&self, message: S) -> anyhow::Result<()> {
//         // Ignore result because it can only fail if there are no receivers remaining.
//         // In that case this shouldn't do anything anyways.
//         let _ = self.broadcast.send(BroadcastPacket::new(
//             Disconnect {
//                 hide_message: false,
//                 message: message.as_ref(),
//             },
//             None,
//         )?);

//         for session in self.list.iter() {
//             session.value().1.cancelled().await;
//         }

//         // Clear to get rid of references, so that the sessions are dropped once they're ready.
//         self.list.clear();

//         Ok(())
//     }

//     /// Returns how many clients are currently connected this tracker.
//     #[inline]
//     pub fn player_count(&self) -> usize {
//         self.list.len()
//     }

//     /// Returns the maximum amount of sessions this tracker will allow.
//     #[inline]
//     pub fn max_player_count(&self) -> usize {
//         SERVER_CONFIG.read().max_players
//     }

//     #[inline]
//     async fn garbage_collector(
//         list: Arc<DashMap<SocketAddr, (mpsc::Sender<MutableBuffer>, Arc<Session>)>>,
//     ) -> ! {
//         let mut interval = tokio::time::interval(GARBAGE_COLLECT_INTERVAL);
//         loop {
//             list.retain(|_, session| -> bool { session.1.is_active() });

//             interval.tick().await;
//         }
//     }
// }