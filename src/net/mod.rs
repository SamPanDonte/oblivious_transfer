use std::net::SocketAddr;

use p256::Scalar;
use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

pub use connection::*;
use crypto::*;
use message::*;
pub use peer::*;
use task::*;

mod connection;
mod crypto;
mod message;
mod peer;
mod task;

type Result<T> = std::result::Result<T, NetworkError>;

#[derive(Debug, Error)]
pub enum NetworkError {
    #[error("Network task has ended")]
    TaskClosed,
    #[error("Network task has panicked")]
    TaskPanic,
    #[error("Failed to create socket: {0}. Terminating network task.")]
    SocketBindError(std::io::Error),
    #[error("Error from socket: {0}")]
    SocketError(#[from] std::io::Error),
    #[error("Received incorrect packet: {0}")]
    MessageError(#[from] MessageError),
    #[error("Local IP address not found: {0}")]
    LocalIpNotFound(#[from] local_ip_address::Error),
    #[error("Error while accessing network interfaces: {0}")]
    InternetInterfaceError(#[from] network_interface::Error),
    #[error("Failed to retrieve local broadcast address")]
    BroadcastAddressNotFound,
    #[error("Received incorrect message from {0}")]
    IncorrectMessage(SocketAddr),
}

impl From<SendError<Action>> for NetworkError {
    fn from(_: SendError<Action>) -> Self {
        Self::TaskClosed
    }
}

/// Events received from socket.
#[derive(Debug)]
pub enum Event {
    Error(NetworkError),
    Connected(Peer),
    Disconnected(SocketAddr),
    Message(SocketAddr, String),
}

/// Actions user can perform.
#[derive(Debug)]
enum Action {
    Broadcast,
    Disconnect,
    Send(SocketAddr, UserMessage, UserMessage, Option<Scalar>),
}
