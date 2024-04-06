use std::io::{Error, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use local_ip_address::local_ip;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use p256::elliptic_curve::sec1::{EncodedPoint, FromEncodedPoint, ToEncodedPoint};
use p256::{NistP256, ProjectivePoint as CurvePoint};
use thiserror::Error;
use tokio::net::UdpSocket;
use tracing::{info, warn};

use super::{CryptoError, NetworkError, Username, UsernameError};

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
    #[error("Crypto error: {0}")]
    InvalidCrypto(#[from] CryptoError),
}

/// Protocol messages.
#[derive(Debug)]
pub enum Message {
    BroadcastGreet(Username),
    BroadcastResponse(Username),
    BroadcastBye,
    Greet(CurvePoint),
    Response(CurvePoint),
    Data(Vec<u8>, Vec<u8>),
}

impl Message {
    /// Convert a message to bytes.
    pub fn into_bytes(self) -> Vec<u8> {
        self.into()
    }
}

fn buffer(type_byte: u8, data: &[u8]) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(HEADER_SIZE + data.len());
    buffer.extend_from_slice(MAGIC_NUMBER);
    buffer.push(type_byte);
    buffer.extend_from_slice(&(data.len() as u16).to_be_bytes());
    buffer.extend_from_slice(data);
    buffer
}

fn point_to_bytes(point: CurvePoint) -> Vec<u8> {
    let encoded = point.to_encoded_point(true);
    encoded.as_bytes().to_vec()
}

fn bytes_to_point(bytes: &[u8]) -> Result<CurvePoint, CryptoError> {
    let encoded =
        EncodedPoint::<NistP256>::from_bytes(bytes).map_err(|_| CryptoError::InvalidPoint)?;
    let option = CurvePoint::from_encoded_point(&encoded);
    if option.is_some().into() {
        Ok(option.unwrap())
    } else {
        Err(CryptoError::InvalidPoint)
    }
}

impl From<Message> for Vec<u8> {
    fn from(value: Message) -> Self {
        match value {
            Message::BroadcastGreet(username) => buffer(0, username.as_bytes()),
            Message::BroadcastResponse(username) => buffer(1, username.as_bytes()),
            Message::BroadcastBye => buffer(2, &[]),
            Message::Greet(point) => buffer(3, &point_to_bytes(point)),
            Message::Response(point) => buffer(4, &point_to_bytes(point)),
            Message::Data(m0, m1) => {
                let mut buf = Vec::with_capacity(2 + m0.len() + m1.len());
                buf.extend_from_slice(&(m0.len() as u16).to_be_bytes());
                buf.extend_from_slice(&m0);
                buf.extend_from_slice(&m1);
                buffer(5, &buf)
            }
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
            3 => Ok(Message::Greet(bytes_to_point(&value[HEADER_SIZE..])?)),
            4 => Ok(Message::Response(bytes_to_point(&value[HEADER_SIZE..])?)),
            5 => {
                let mut len = [0; 8];
                len[6] = value[HEADER_SIZE];
                len[7] = value[HEADER_SIZE + 1];
                let len = usize::from_be_bytes(len);

                if len > size - 2 {
                    return Err(MessageError::InvalidMessageLength);
                }

                let m0 = value[HEADER_SIZE + 2..HEADER_SIZE + 2 + len].to_vec();
                let m1 = value[HEADER_SIZE + 2 + len..].to_vec();
                Ok(Message::Data(m0, m1))
            }
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
        let mut buffer = [0; 2048];
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
