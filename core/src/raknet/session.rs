use std::mem::MaybeUninit;
use std::net::SocketAddr;
use std::num::NonZeroU64;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::time::Instant;
use parking_lot::{Mutex, RwLock};
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, broadcast, OnceCell};
use tokio_util::sync::CancellationToken;
use util::bytes::MutableBuffer;
use crate::crypto::Encryptor;
use crate::instance::UdpController;

use crate::raknet::{CompoundCollector, OrderChannel, RecoveryQueue, SendQueues};

use super::{BroadcastPacket, RawPacket};

const ORDER_CHANNEL_COUNT: usize = 5;

static SESSION_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug)]
pub enum RakNetMessage {
    Connected,
    CreateEncryptor(Encryptor),
    Message(MutableBuffer),
    Disconnect
}

#[derive(Default)]
pub struct RakNetSessionBuilder {
    pub broadcast: Option<broadcast::Sender<BroadcastPacket>>,
    pub packet_receiver: Option<mpsc::Receiver<MutableBuffer>>,
    pub receiver: Option<mpsc::Receiver<RakNetMessage>>,
    pub sender: Option<mpsc::Sender<RakNetMessage>>,
    pub udp_controller: Option<Arc<UdpController>>,
    pub addr: Option<SocketAddr>,
    pub mtu: u16,
    pub guid: u64
}

impl RakNetSessionBuilder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn udp(&mut self, controller: Arc<UdpController>) -> &mut Self {
        self.udp_controller = Some(controller);
        self
    }

    #[inline]
    pub fn broadcast(&mut self, broadcast: broadcast::Sender<BroadcastPacket>) -> &mut Self {
        self.broadcast = Some(broadcast);
        self
    }

    #[inline]
    pub fn channel(&mut self, (sender, receiver): (mpsc::Sender<RakNetMessage>, mpsc::Receiver<RakNetMessage>)) -> &mut Self {
        self.receiver = Some(receiver);
        self.sender = Some(sender);
        self
    }

    #[inline]
    pub fn packet_receiver(&mut self, receiver: mpsc::Receiver<MutableBuffer>) -> &mut Self {
        self.packet_receiver = Some(receiver);
        self
    }

    #[inline]
    pub fn addr(&mut self, addr: SocketAddr) -> &mut Self {
        self.addr = Some(addr);
        self
    }

    #[inline]
    pub fn mtu(&mut self, mtu: u16) -> &mut Self {
        self.mtu = mtu;
        self
    }

    #[inline]
    pub fn guid(&mut self, guid: u64) -> &mut Self {
        self.guid = guid;
        self
    }

    #[inline]
    pub fn build(mut self) -> Arc<RakNetSession> {
        let packet_receiver = self.packet_receiver.take().unwrap();
        let session = Arc::new(RakNetSession::from(self));

        session.clone().start_tick_job();
        session.clone().start_packet_job(packet_receiver);
        session
    }
}

#[derive(Debug)]
pub struct RakNetSession {
    pub session_id: NonZeroU64,
    pub token: CancellationToken,
    pub broadcast: broadcast::Sender<BroadcastPacket>,
    /// IPv4 socket of the server.
    pub udp_controller: Arc<UdpController>,
    /// IP address of this session.
    pub addr: SocketAddr,
    /// Maximum packet size
    pub mtu: u16,
    /// Client-provided GUID.
    /// These IDs are randomly generated by Minecraft for each connection and are unreliable.
    /// They should not be used as unique identifiers, use the XUID instead.
    pub guid: u64,
    /// Timestamp of when the last packet was received from this client.
    pub last_update: RwLock<Instant>,
    /// Batch number last assigned by the server.
    pub batch_number: AtomicU32,
    /// Sequence index last assigned by the server.
    pub sequence_index: AtomicU32,
    /// Acknowledgment index last used by the server.
    pub ack_index: AtomicU32,
    /// Compound ID last used by the server.
    pub compound_id: AtomicU16,
    /// Latest sequence index that was received.
    /// Sequenced packets with sequence numbers less than this one will be discarded.
    pub client_batch_number: AtomicU32,
    /// Collects fragmented packets.
    pub compound_collector: CompoundCollector,
    /// Channels used to order packets.
    pub order_channels: [OrderChannel; ORDER_CHANNEL_COUNT],
    /// Keeps track of all packets that are waiting to be sent.
    pub send_queue: SendQueues,
    /// Packets that are ready to be acknowledged.
    pub confirmed_packets: Mutex<Vec<u32>>,
    /// Queue that stores packets in case they need to be recovered due to packet loss.
    pub recovery_queue: RecoveryQueue,
    /// Whether compression has been configured for this session.
    /// This is set to true after network settings have been sent to the client.
    pub compression_enabled: AtomicBool,
    pub current_tick: AtomicU64,
    pub encryptor: OnceCell<Encryptor>,
    pub sender: mpsc::Sender<RakNetMessage>,
    pub receiver: mpsc::Receiver<RakNetMessage>
}

impl RakNetSession {
    #[inline]
    pub fn is_active(&self) -> bool {
        !self.token.is_cancelled()
    }
}

impl From<RakNetSessionBuilder> for RakNetSession {
    fn from(builder: RakNetSessionBuilder) -> Self {
        /// SAFETY: MaybeUninit does not need initialisation.
        let mut order_channels: [MaybeUninit<OrderChannel>; ORDER_CHANNEL_COUNT] = unsafe {
            MaybeUninit::uninit().assume_init()
        };

        for channel in &mut order_channels {
            channel.write(OrderChannel::new());
        }

        // SAFETY: This transmute is safe because every array element has been initialised.
        let order_channels = unsafe {
            std::mem::transmute::<_, [OrderChannel; ORDER_CHANNEL_COUNT]>(order_channels)
        };

        Self {
            session_id: NonZeroU64::new(SESSION_ID_COUNTER.fetch_add(1, Ordering::Relaxed)).unwrap(),
            token: CancellationToken::new(),
            current_tick: AtomicU64::new(0),
            broadcast: builder.broadcast.unwrap(),
            udp_controller: builder.udp_controller.unwrap(),
            addr: builder.addr.unwrap(),
            mtu: builder.mtu,
            guid: builder.guid,
            last_update: RwLock::new(Instant::now()),
            batch_number: AtomicU32::new(0),
            sequence_index: AtomicU32::new(0),
            ack_index: AtomicU32::new(0),
            compound_id: AtomicU16::new(0),
            client_batch_number: AtomicU32::new(0),
            compound_collector: CompoundCollector::new(),
            order_channels,
            send_queue: SendQueues::new(),
            confirmed_packets: Mutex::new(Vec::new()),
            recovery_queue: RecoveryQueue::new(),
            compression_enabled: AtomicBool::new(false),
            encryptor: OnceCell::new(),
            sender: builder.sender.unwrap(),
            receiver: builder.receiver.unwrap()
        }
    }
}