use bytes::BytesMut;
use crossbeam::deque;
use std::net::SocketAddr;
use std::{
    net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use std::mem::MaybeUninit;
use std::time::Duration;
use crossbeam::deque::{Injector, Worker};

use tokio::net::UdpSocket;
use tokio::{task, time};
use tokio_util::sync::CancellationToken;

use crate::error::{VexError, VexResult};
use crate::network::SchedulerQueue;

const IPV4_LOCAL_ADDR: Ipv4Addr = Ipv4Addr::new(127, 0, 0, 1);
const IPV6_LOCAL_ADDR: Ipv6Addr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1);

const RECV_BUF_SIZE: usize = 4096;

const INCOMING_QUEUE_SIZE: usize = 25;
const LEAVING_QUEUE_SIZE: usize = 25;

const WORKER_COUNT: usize = 10;

pub struct RawPacket {
    buffer: BytesMut,
    address: SocketAddr,
}

pub struct NetworkManager {
    flag: CancellationToken,
    ipv4_socket: Arc<UdpSocket>,
    ipv6_socket: Arc<Option<UdpSocket>>,
}

impl NetworkManager {
    pub async fn start(
        ipv4_port: u16,
        ipv6_port: Option<u16>
    ) -> VexResult<()> {
        let token = CancellationToken::new();

        let ipv4_socket =
            Arc::new(UdpSocket::bind(SocketAddrV4::new(IPV4_LOCAL_ADDR, ipv4_port)).await?);

        let ipv6_socket = Arc::new(if let Some(port) = ipv6_port {
            Some(UdpSocket::bind(SocketAddrV6::new(IPV6_LOCAL_ADDR, port, 0, 0)).await?)
        } else {
            None
        });

        let (incoming_queue, local_queues) = SchedulerQueue::new();
        let incoming_queue = Arc::new(incoming_queue);

        let receiver_task = {
            let token = token.clone();
            let ipv4_socket = ipv4_socket.clone();
            let queue = incoming_queue.clone();

            tokio::spawn(async move {
                Self::v4_receiver_task(token, ipv4_socket, queue).await;
            })
        };

        let _ = tokio::join!(receiver_task);

        Ok(())
    }

    /// Receives packets from IPv4 clients and adds them to the receive queue
    async fn v4_receiver_task(
        token: CancellationToken,
        socket: Arc<UdpSocket>,
        scheduler: Arc<SchedulerQueue<RawPacket, WORKER_COUNT>>
    ) {
        let mut receive_buffer = [0u8; RECV_BUF_SIZE];

        while !token.is_cancelled() {
            // Wait on both the cancellation token and socket at the same time.
            // The token will immediately take over and stop the task when the server is shutting down.
            let (n, address) = tokio::select! {
                result = socket.recv_from(&mut receive_buffer) => {
                     match result {
                        Ok(r) => r,
                        Err(e) => {
                            tracing::error!("Failed to receive packet: {e:?}");
                            continue;
                        }
                    }
                },
                _ = token.cancelled() => {
                    break
                }
            };

            tracing::debug!("{n:?} bytes from {address:?}");
            scheduler.schedule_task(RawPacket {
                buffer: BytesMut::from(&receive_buffer[..n]),
                address,
            });
        }
    }
    //
    // /// Sends packets from the send queue
    // async fn sender_task(flag: Arc<AtomicBool>, socket: Arc<UdpSocket>) {
    //     while manager.is_active() {}
    // }
}
