use std::net::SocketAddr;

use thiserror::Error;
use tokio::sync::mpsc::error::SendError;

pub use connection::*;
use message::*;
pub use peer::*;
use task::*;

mod connection;
mod message;
mod peer;
mod task;

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
}

/// Actions user can perform.
#[derive(Debug)]
enum Action {
    Broadcast,
    Disconnect,
}
