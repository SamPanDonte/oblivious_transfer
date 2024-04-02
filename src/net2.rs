use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::thread::JoinHandle;

use eframe::egui::Context;
use tokio::net::UdpSocket;
use tokio::runtime::{Builder, Handle};
use tokio::sync::{mpsc, oneshot};

static MAGIC_NUMBER: &[u8] = b"OTMP"; // Oblivious Transfer Message Protocol

enum MessageType {
    Broadcast,
    HandshakeStart,
    HandshakeAnswer,
    Message,
    Unknown,
}

impl From<u8> for MessageType {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Broadcast,
            1 => Self::HandshakeStart,
            2 => Self::HandshakeAnswer,
            3 => Self::Message,
            _ => Self::Unknown,
        }
    }
}

/// Peer to peer network user.
pub struct User {
    address: SocketAddr,
    name: Option<String>,
}

impl User {
    /// Create a new user.
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            name: None,
        }
    }

    fn with_name(address: SocketAddr, name: Option<String>) -> Self {
        Self { address, name }
    }
}

/// Peer to peer network message.
pub struct Message {
    pub name: String,
}

/// Peer to peer network implementation.
pub struct NetworkHost {
    join_handle: JoinHandle<Result<(), oneshot::error::RecvError>>,
    message_receiver: mpsc::Receiver<Message>,
    message_sender: mpsc::Sender<Message>,
    host_receiver: mpsc::Receiver<User>,
    stop_runtime: oneshot::Sender<()>,
    context: Context,
    handle: Handle,
    name: String,
}

impl NetworkHost {
    /// Create a new network host.
    pub fn new(context: Context, name: String, port: u16) -> Result<Self, Box<dyn Error>> {
        let runtime = Builder::new_current_thread().enable_all().build()?;
        let handle = runtime.handle().clone();
        let (stop_runtime, receiver) = oneshot::channel();
        let (host_sender, host_receiver) = mpsc::channel(25);
        let (_message_sender2, message_receiver) = mpsc::channel(25);
        let (message_sender, mut message_receiver2) = mpsc::channel(25);
        let join_handle = std::thread::spawn(move || runtime.block_on(receiver));

        let context_clone = context.clone();

        handle.spawn(async move {
            let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
            let socket = Arc::new(UdpSocket::bind(address).await.unwrap()); // TODO: handle error
            let sender = socket.clone();

            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                loop {
                    match socket.recv_from(&mut buffer).await {
                        Ok((size, sender)) => {
                            if size < 5 {
                                continue;
                            }
                            if &buffer[..4] != MAGIC_NUMBER {
                                println!("received message not from protocol - ignore");
                            }
                            match buffer[5].into() {
                                MessageType::Broadcast => {
                                    let name = if size > 5 {
                                        String::from_utf8(buffer[6..size].to_vec()).ok()
                                    } else {
                                        None
                                    };
                                    host_sender
                                        .send(User::with_name(sender, name))
                                        .await
                                        .unwrap(); // TODO: handle error
                                    context_clone.request_repaint();
                                }
                                MessageType::HandshakeStart => todo!("handle handshake start"),
                                MessageType::HandshakeAnswer => todo!("handle handshake answer"),
                                MessageType::Message => todo!("handle message"),
                                _ => println!("received unknown message type - ignore"),
                            }
                        }
                        Err(error) => println!("error when listening on socket: {error}"),
                    }
                }
            });

            tokio::spawn(async move {
                loop {
                    let message: Message = message_receiver2.recv().await.unwrap(); // TODO: handle error
                    let mut buffer = Vec::with_capacity(5 + message.name.len());
                    buffer.extend_from_slice(MAGIC_NUMBER);
                    buffer.push(MessageType::Broadcast as u8);
                    buffer.extend_from_slice(message.name.as_bytes());
                    sender.send(&buffer).await.unwrap(); // TODO: handle error
                }
            });
        });

        Ok(Self {
            join_handle,
            message_receiver,
            message_sender,
            host_receiver,
            stop_runtime,
            context,
            handle,
            name,
        })
    }

    /// Stop the network host background thread and tokio runtime.
    pub fn stop(self) -> Result<(), Box<dyn Error>> {
        self.stop_runtime
            .send(())
            .map_err(|_| "error sending stop to runtime")?;
        self.join_handle
            .join()
            .expect("failed to join thread handle")?;
        Ok(())
    }

    /// Refresh the known host list.
    pub async fn refresh(&mut self) {
        todo!("refresh host list")
    }
}
