use std::io::Read;
use std::net::SocketAddr;
use std::sync::{Arc, Weak};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use bytes::{Buf, BytesMut};
use dashmap::DashMap;
use flate2::read::{DeflateDecoder, DeflateEncoder, GzDecoder, ZlibDecoder};
use parking_lot::RwLock;
use tokio::net::UdpSocket;
use tokio_util::sync::CancellationToken;

use crate::{vex_assert, vex_error};
use crate::error::VexResult;
use crate::packets::{GAME_PACKET_ID, RequestNetworkSettings};
use crate::raknet::{
    CompoundCollector, Frame, FrameBatch, Header, OrderChannel, RecoveryQueue, Reliability,
    SendPriority, SendQueue,
};
use crate::raknet::packet::RawPacket;
use crate::raknet::packets::{Ack, AckRecord, Decodable, Encodable, Nack};
use crate::raknet::packets::{
    ConnectionRequest, ConnectionRequestAccepted, DisconnectNotification, NewIncomingConnection,
};
use crate::raknet::packets::{OnlinePing, OnlinePong};
use crate::util::{AsyncDeque, ReadExtensions};

/// Tick interval of the internal session ticker.
const INTERNAL_TICK_INTERVAL: Duration = Duration::from_millis(1000 / 20);
/// Tick interval for session packet processing.
const TICK_INTERVAL: Duration = Duration::from_millis(1000 / 20);
/// Inactivity timeout.
///
/// Any sessions that do not respond within this specified timeout will be disconnect from the server.
/// Timeouts can happen if a client's game crashed for example.
/// They will stop responding to the server, but will not explicitly send a disconnect request.
/// Hence, they have to be disconnected manually after the timeout passes.
const SESSION_TIMEOUT: Duration = Duration::from_secs(5);

const ORDER_CHANNEL_COUNT: usize = 5;
const GARBAGE_COLLECT_INTERVAL: Duration = Duration::from_secs(10);

/// Sessions directly correspond to clients connected to the server.
///
/// Anything that has to do with specific clients must be communicated with their associated sessions.
/// The server does not interact with clients directly, everything is done through these sessions.
///
#[derive(Debug)]
pub struct Session {
    current_tick: AtomicU64,
    ipv4_socket: Arc<UdpSocket>,
    /// IP address of this session.
    address: SocketAddr,
    /// Maximum packet size
    mtu: u16,
    /// Client-provided GUID.
    /// These IDs are randomly generated by Minecraft for each connection and are unreliable.
    /// They should not be used as unique identifiers, use the XUID instead.
    guid: i64,
    /// Timestamp of when the last packet was received from this client.
    last_update: RwLock<Instant>,
    /// Indicates whether this session is active.
    active: CancellationToken,
    last_assigned_batch_number: AtomicU32,
    last_assigned_sequence_index: AtomicU32,
    acknowledgment_index: AtomicU32,
    /// Latest sequence index that was received.
    /// Sequenced packets with sequence numbers less than this one will be discarded.
    last_client_batch_number: AtomicU32,
    /// Collects fragmented packets.
    compound_collector: CompoundCollector,
    /// Channels used to order packets.
    order_channels: [OrderChannel; ORDER_CHANNEL_COUNT],
    /// Keeps track of all packets that are waiting to be sent.
    send_queue: SendQueue,
    /// Keeps track of all unprocessed received packets.
    receive_queue: AsyncDeque<BytesMut>,
    recovery_queue: RecoveryQueue,
}

impl Session {
    /// Creates a new session.
    pub fn new(
        ipv4_socket: Arc<UdpSocket>,
        address: SocketAddr,
        mtu: u16,
        client_guid: i64,
    ) -> Arc<Self> {
        let session = Arc::new(Self {
            current_tick: AtomicU64::new(0),
            ipv4_socket,
            address,
            mtu,
            guid: client_guid,
            last_update: RwLock::new(Instant::now()),
            active: CancellationToken::new(),
            last_assigned_batch_number: AtomicU32::new(0),
            last_assigned_sequence_index: AtomicU32::new(0),
            last_client_batch_number: AtomicU32::new(0),
            acknowledgment_index: AtomicU32::new(0),
            compound_collector: CompoundCollector::new(),
            order_channels: Default::default(),
            send_queue: SendQueue::new(),
            receive_queue: AsyncDeque::new(5),
            recovery_queue: RecoveryQueue::new(),
        });

        // Session ticker
        {
            let session = session.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(INTERNAL_TICK_INTERVAL);
                while !session.active.is_cancelled() {
                    match session.tick().await {
                        Ok(_) => (),
                        Err(e) => tracing::error!("{e}"),
                    }
                    interval.tick().await;
                }

                tracing::info!("Session {:X} closed", session.guid);
            });
        }

        // Packet processor
        {
            let session = session.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(TICK_INTERVAL);
                while !session.active.is_cancelled() {
                    match session.process_raw_packet().await {
                        Ok(_) => (),
                        Err(e) => tracing::error!("{e}"),
                    }
                    interval.tick().await;
                }
            });
        }

        tracing::info!("Session {client_guid:X} created");
        session
    }

    /// Processes the raw packet coming directly from the network.
    ///
    /// If a packet is an ACK or NACK type, it will be responded to accordingly (using [`Session::process_ack`] and [`Session::process_nack`]).
    /// Frame batches are processed by [`Session::process_frame_batch`].
    async fn process_raw_packet(&self) -> VexResult<()> {
        let task = tokio::select! {
            _ = self.active.cancelled() => {
                return Ok(())
            },
            task = self.receive_queue.pop() => task
        };
        *self.last_update.write() = Instant::now();

        match *task.first().unwrap() {
            Ack::ID => self.process_ack(task).await,
            Nack::ID => self.process_nack(task).await,
            _ => self.process_frame_batch(task).await,
        }
    }

    /// Processes a batch of frames.
    ///
    /// This performs the actions required by the Raknet reliability layer, such as
    /// * Inserting packets into the order channels
    /// * Inserting packets into the compound collector
    /// * Discarding old sequenced frames
    /// * Acknowledging reliable packets
    async fn process_frame_batch(&self, task: BytesMut) -> VexResult<()> {
        let batch = FrameBatch::decode(task)?;
        self.last_client_batch_number
            .fetch_max(batch.batch_number, Ordering::SeqCst);

        for frame in batch.frames {
            if frame.reliability.is_sequenced()
                && frame.sequence_index < self.last_client_batch_number.load(Ordering::SeqCst)
            {
                // Discard packet
                continue;
            }

            if frame.reliability.is_reliable() {
                // Send ACK
                let encoded = Ack {
                    records: vec![AckRecord::Single(frame.reliable_index)],
                }
                    .encode();

                let acknowledgement = match encoded {
                    Ok(a) => a,
                    Err(e) => {
                        tracing::error!("{e}");
                        continue;
                    }
                };

                self.ipv4_socket
                    .send_to(acknowledgement.as_ref(), self.address)
                    .await?;
            }

            // TODO: Handle errors in processing properly

            // Sequenced implies ordered
            if frame.reliability.is_ordered() || frame.reliability.is_sequenced() {
                assert!(!frame.is_compound); // TODO: Figure this out

                // Add packet to order queue
                if let Some(ready) = self.order_channels[frame.order_channel as usize].insert(frame)
                {
                    for packet in ready {
                        self.process_unframed_packet(packet.body).await?;
                    }
                }

                continue;
            }

            if frame.is_compound {
                tracing::info!("Received compound");
                if let Some(p) = self.compound_collector.insert(frame.clone()) {
                    self.process_unframed_packet(p).await?
                }
            }

            self.process_unframed_packet(frame.body.clone()).await?;
        }

        Ok(())
    }

    /// Processes an unencapsulated game packet.
    async fn process_unframed_packet(&self, mut task: BytesMut) -> VexResult<()> {
        let bytes = task.as_ref();

        let packet_id = *task.first().expect("Game packet buffer was empty");
        match packet_id {
            DisconnectNotification::ID => {
                tracing::debug!("Session {:X} requested disconnect", self.guid);
                self.flag_for_close();
            }
            ConnectionRequest::ID => self.handle_connection_request(task).await?,
            NewIncomingConnection::ID => self.handle_new_incoming_connection(task).await?,
            OnlinePing::ID => self.handle_connected_ping(task).await?,
            GAME_PACKET_ID => self.handle_game_packet(task).await?,
            id => {
                tracing::info!("ID: {} {:?}", id, task.as_ref());
                todo!("Other game packet IDs")
            }
        }

        Ok(())
    }

    async fn handle_connection_request(&self, task: BytesMut) -> VexResult<()> {
        let request = ConnectionRequest::decode(task)?;
        let response = ConnectionRequestAccepted {
            client_address: self.address,
            request_time: request.time,
        }
            .encode()?;

        self.send_queue.insert(
            SendPriority::Medium,
            Frame::new(Reliability::Reliable, response),
        );
        Ok(())
    }

    async fn handle_new_incoming_connection(&self, task: BytesMut) -> VexResult<()> {
        let request = NewIncomingConnection::decode(task)?;
        Ok(())
    }

    async fn handle_connected_ping(&self, task: BytesMut) -> VexResult<()> {
        let ping = OnlinePing::decode(task)?;
        let pong = OnlinePong {
            ping_time: ping.time,
            pong_time: ping.time,
        };

        let pong = pong.encode()?;

        self.send_queue
            .insert(SendPriority::Low, Frame::new(Reliability::Unreliable, pong));
        Ok(())
    }

    async fn handle_game_packet(&self, mut task: BytesMut) -> VexResult<()> {
        vex_assert!(task.get_u8() == 0xfe);

        tracing::info!("Received game packet: {:x?}", task.as_ref());

        let length = task.get_var_u32()?;
        let remaining = task.remaining();
        vex_assert!(remaining >= length as usize,
            format!("Packet body size is less than specified. Specified {length} bytes, but received {remaining}")
        );

        let header = Header::decode(&mut task)?;
        match header.id {
            RequestNetworkSettings::ID => self.handle_request_network_settings(task).await,
            _ => todo!("Other game packets"),
        }
    }

    async fn handle_request_network_settings(&self, mut task: BytesMut) -> VexResult<()> {
        let request = RequestNetworkSettings::decode(task)?;
        tracing::debug!("{request:?}");
        Ok(())
    }

    /// Processes an acknowledgement received from the client.
    ///
    /// This function unregisters the specified packet IDs from the recovery queue.
    async fn process_ack(&self, task: BytesMut) -> VexResult<()> {
        let ack = Ack::decode(task)?;
        self.recovery_queue.confirm(&ack.records);

        Ok(())
    }

    /// Processes a negative acknowledgement received from the client.
    ///
    /// This function makes sure the packet is retrieved from the recovery queue and sent to the
    /// client again.
    async fn process_nack(&self, task: BytesMut) -> VexResult<()> {
        let nack = Nack::decode(task)?;
        let batch = self.recovery_queue.recover(&nack.records);
        tracing::info!("Recovered packets: {:?}", nack.records);

        self.send_queue.insert_batch(SendPriority::Medium, batch);
        Ok(())
    }

    /// Performs tasks not related to packet processing
    async fn tick(self: &Arc<Self>) -> VexResult<()> {
        let current_tick = self.current_tick.fetch_add(1, Ordering::SeqCst);

        // Session has timed out
        if Instant::now().duration_since(*self.last_update.read()) > SESSION_TIMEOUT {
            self.flag_for_close();
            tracing::info!("Session timed out");
        }

        self.flush_send_queue(current_tick).await?;
        Ok(())
    }

    pub fn flag_for_close(&self) {
        self.active.cancel();
    }

    /// Returns whether the session is currently active.
    ///
    /// If this returns false, any remaining associated processes should be stopped as soon as possible.
    #[inline]
    pub fn is_active(&self) -> bool {
        !self.active.is_cancelled()
    }

    #[inline]
    pub fn get_guid(&self) -> i64 {
        self.guid
    }

    async fn flush_send_queue(&self, tick: u64) -> VexResult<()> {
        // TODO: Handle errors properly
        if let Some(frames) = self.send_queue.flush(SendPriority::High) {
            self.send_frames(frames).await?;
        }

        if tick % 2 == 0 {
            if let Some(frames) = self.send_queue.flush(SendPriority::Medium) {
                self.send_frames(frames).await?;
            }
        }

        if tick % 4 == 0 {
            if let Some(frames) = self.send_queue.flush(SendPriority::Low) {
                self.send_frames(frames).await?;
            }
        }

        Ok(())
    }

    async fn send_frames(&self, frames: Vec<Frame>) -> VexResult<()> {
        // TODO: Handle errors properly
        let max_batch_size = self.mtu as usize - std::mem::size_of::<FrameBatch>();
        let mut batch = FrameBatch {
            batch_number: self
                .last_assigned_batch_number
                .fetch_add(1, Ordering::SeqCst),
            frames: vec![],
        };

        for mut frame in frames {
            let frame_size = frame.body.len() + std::mem::size_of::<Frame>();

            if frame_size > self.mtu as usize {
                todo!("Create compound");
            }
            if frame.reliability.is_ordered() {
                let order_index =
                    self.order_channels[frame.order_channel as usize].get_server_index();
                frame.order_index = order_index;
            }
            if frame.reliability.is_sequenced() {
                let sequence_index = self
                    .last_assigned_sequence_index
                    .fetch_add(1, Ordering::SeqCst);
                frame.sequence_index = sequence_index;
            }
            if frame.reliability.is_reliable() {
                frame.reliable_index = self.acknowledgment_index.fetch_add(1, Ordering::SeqCst);
                self.recovery_queue.insert(frame.clone());
            }

            if batch.estimate_size() + frame_size < max_batch_size {
                batch.frames.push(frame);
            } else {
                let encoded = batch.encode()?;

                // TODO: Add IPv6 support
                self.ipv4_socket.send_to(&encoded, self.address).await?;

                batch = FrameBatch {
                    batch_number: self
                        .last_assigned_batch_number
                        .fetch_add(1, Ordering::SeqCst),
                    frames: vec![],
                };
            }
        }

        // Send remaining packets not sent by loop
        if !batch.frames.is_empty() {
            let encoded = batch.encode()?;
            // TODO: Add IPv6 support
            self.ipv4_socket.send_to(&encoded, self.address).await?;
        }

        Ok(())
    }

    /// Called by the [`SessionTracker`] to forward packets from the network service to
    /// the session corresponding to the client.
    fn forward(self: &Arc<Self>, buffer: BytesMut) {
        self.receive_queue.push(buffer);
    }
}

/// Keeps track of all sessions on the server.
#[derive(Debug)]
pub struct SessionTracker {
    /// Whether the server is running.
    /// Once this token is cancelled, the tracker will cancel all the sessions' individual tokens.
    global_token: CancellationToken,
    /// Map of all tracked sessions, listed by IP address.
    session_list: Arc<DashMap<SocketAddr, Arc<Session>>>,
    /// Maximum amount of sessions that this tracker will accept.
    max_session_count: usize,
}

impl SessionTracker {
    /// Creates a new session tracker.
    pub fn new(
        global_token: CancellationToken,
        max_session_count: usize,
    ) -> VexResult<SessionTracker> {
        let session_list = Arc::new(DashMap::new());
        {
            let session_list = session_list.clone();
            tokio::spawn(async move {
                SessionTracker::garbage_collector(session_list).await;
            });
        }

        Ok(SessionTracker {
            global_token,
            session_list,
            max_session_count,
        })
    }

    /// Creates a new session and adds it to the tracker.
    pub fn add_session(
        &self,
        ipv4_socket: Arc<UdpSocket>,
        address: SocketAddr,
        mtu: u16,
        client_guid: i64,
    ) {
        let session = Session::new(ipv4_socket, address, mtu, client_guid);
        self.session_list.insert(address, session);
    }

    /// Forwards a packet from the network service to the correct session.
    pub fn forward_packet(&self, packet: RawPacket) -> VexResult<()> {
        self.session_list
            .get(&packet.address)
            .map(|r| {
                let session = r.value();
                session.forward(packet.buffer);
            })
            .ok_or(vex_error!(
                InvalidRequest,
                "Attempted to forward packet for non-existent session"
            ))
    }

    /// Returns how many clients are currently connected this tracker.
    pub fn session_count(&self) -> usize {
        self.session_list.len()
    }

    /// Returns the maximum amount of sessions this tracker will allow.
    pub fn max_session_count(&self) -> usize {
        self.max_session_count
    }

    async fn garbage_collector(session_list: Arc<DashMap<SocketAddr, Arc<Session>>>) -> ! {
        let mut interval = tokio::time::interval(GARBAGE_COLLECT_INTERVAL);
        loop {
            session_list.retain(|_, session| -> bool { session.is_active() });

            interval.tick().await;
        }
    }
}
