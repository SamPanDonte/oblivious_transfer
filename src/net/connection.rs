use std::net::SocketAddr;
use std::ops::Deref;

use thiserror::Error;

/// Error in creating username.
#[derive(Debug, Error)]
pub enum UsernameError {
    #[error("Username cannot be empty")]
    UsernameEmpty,
    #[error("Username cannot have more than 100 character")]
    UsernameTooLong,
}

/// Peer username. Has between 1 and 100 characters.
#[repr(transparent)]
#[derive(Clone, Debug)]
pub struct Username(String);

impl Username {
    /// Create a new username. Username must have between 1 and 100 characters.
    pub fn new(name: String) -> Result<Self, UsernameError> {
        name.try_into()
    }
}

impl From<Username> for String {
    fn from(value: Username) -> Self {
        value.0
    }
}

impl TryFrom<String> for Username {
    type Error = UsernameError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.len() {
            0 => Err(UsernameError::UsernameEmpty),
            1..=100 => Ok(Self(value)),
            _ => Err(UsernameError::UsernameTooLong),
        }
    }
}

impl Deref for Username {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Peer to peer network user.
#[derive(Clone, Debug)]
pub struct Peer {
    address: SocketAddr,
    name: Option<Username>,
}

impl Peer {
    /// Create a new peer.
    pub fn new(address: SocketAddr) -> Self {
        Self {
            address,
            name: None,
        }
    }

    /// Create a new peer with name.
    pub(super) fn new_with_name(address: SocketAddr, name: Username) -> Self {
        Self {
            address,
            name: Some(name),
        }
    }

    /// Get the address of the peer.
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Get the name of the peer.
    pub fn name(&self) -> Option<&Username> {
        self.name.as_ref()
    }
}
