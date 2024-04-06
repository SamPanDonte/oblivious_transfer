use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use local_ip_address::local_ip;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use thiserror::Error;
use tokio::net::UdpSocket;
use tracing::{info, warn};

use super::{NetworkError, Username, UsernameError};

static MAGIC_NUMBER: &[u8] = b"OTMP"; // Oblivious Transfer Message Protocol
static HEADER_SIZE: usize = 7; // 4 - magic number, 1 - message type, 2 - message length

/// Protocol message parse error.
#[derive(Debug, Error)]
pub enum MessageError {
    #[error("Header bytes are missing")]
    MissingHeaderBytes,
    #[error("Magic number is invalid")]
    InvalidMagicNumber,
    #[error("Message type is invalid")]
    InvalidMessageType,
    #[error("Message length is invalid")]
    InvalidMessageLength,
    #[error("Message is invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("Greeting name is invalid: {0}")]
    InvalidUsername(#[from] UsernameError),
}

/// Protocol messages.
#[derive(Debug)]
pub enum Message {
    BroadcastGreet(Username),
    BroadcastResponse(Username),
    BroadcastBye,
}

impl Message {
    /// Convert a message to bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.into()
    }
}

fn buffer(type_byte: u8, size: usize) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(HEADER_SIZE + size);
    buffer.extend_from_slice(MAGIC_NUMBER);
    buffer.push(type_byte);
    buffer.extend_from_slice(&(size as u16).to_be_bytes());
    buffer
}

impl From<Message> for Vec<u8> {
    fn from(value: Message) -> Self {
        match value {
            Message::BroadcastGreet(username) => {
                let mut buffer = buffer(0, username.len());
                buffer.extend_from_slice(username.as_bytes());
                buffer
            }
            Message::BroadcastResponse(username) => {
                let mut buffer = buffer(1, username.len());
                buffer.extend_from_slice(username.as_bytes());
                buffer
            }
            Message::BroadcastBye => buffer(2, 0),
        }
    }
}

impl TryFrom<&[u8]> for Message {
    type Error = MessageError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < HEADER_SIZE {
            return Err(MessageError::MissingHeaderBytes);
        }

        if &value[..MAGIC_NUMBER.len()] != MAGIC_NUMBER {
            return Err(MessageError::InvalidMagicNumber);
        }

        let size = usize::from_be_bytes([0, 0, 0, 0, 0, 0, value[5], value[6]]);

        if value.len() != HEADER_SIZE + size {
            return Err(MessageError::InvalidMessageLength);
        }

        match value[4] {
            0 => {
                let name = String::from_utf8(value[HEADER_SIZE..].to_vec())?;
                Ok(Message::BroadcastGreet(Username::new(name)?))
            }
            1 => {
                let name = String::from_utf8(value[HEADER_SIZE..].to_vec())?;
                Ok(Message::BroadcastResponse(Username::new(name)?))
            }
            2 => match size {
                0 => Ok(Message::BroadcastBye),
                _ => Err(MessageError::InvalidMessageLength),
            },
            _ => Err(MessageError::InvalidMessageType),
        }
    }
}

/// Oblivious Transfer Message Protocol socket.
#[derive(Debug)]
pub(super) struct OTMPSocket(UdpSocket, u16);

impl OTMPSocket {
    /// Bind to a port.
    /// The Socket is set to broadcast mode.
    pub async fn bind(port: u16) -> Result<Self, std::io::Error> {
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port);
        let socket = UdpSocket::bind(address).await?;
        socket.set_broadcast(true)?;
        Ok(Self(socket, port))
    }

    /// Send a message to a specific address.
    pub async fn send_to(&self, message: Message, address: SocketAddr) -> Result<(), Error> {
        info!("Sending message: {message:?} to address: {address}");
        let bytes = message.into_bytes();
        let size = self.0.send_to(&bytes, address).await?;
        if size != bytes.len() {
            warn!("Failed to send all bytes to address: {address}");
            return Err(Error::new(ErrorKind::Other, "Failed to send all bytes"));
        }
        Ok(())
    }

    /// Broadcast a message.
    pub async fn broadcast(&self, message: Message) -> Result<(), NetworkError> {
        self.send_to(message, get_broadcast(self.1)?).await?;
        Ok(())
    }

    /// Receive a message with the sender address.
    pub async fn recv_from(&self) -> Result<(Message, SocketAddr), NetworkError> {
        let mut buffer = vec![0; 1024];
        let (size, address) = self.0.recv_from(&mut buffer).await?;
        let message = Message::try_from(&buffer[..size])?;
        info!("Received message: {message:?} from address: {address}");
        Ok((message, address))
    }
}

fn get_broadcast(port: u16) -> Result<SocketAddr, NetworkError> {
    let local_address = local_ip()?;

    for interface in NetworkInterface::show()? {
        for address in interface.addr {
            if address.ip() == local_address {
                return address
                    .broadcast()
                    .map(|addr| SocketAddr::new(addr, port))
                    .ok_or(NetworkError::BroadcastAddressNotFound);
            }
        }
    }

    Err(NetworkError::BroadcastAddressNotFound)
}
