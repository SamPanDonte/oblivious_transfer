use std::net::SocketAddr;
use std::ops::{Deref, Range};

use eframe::egui::TextBuffer;
use thiserror::Error;

/// Error in creating username.
#[derive(Debug, Error)]
pub enum UsernameError {
    #[error("Username cannot be empty")]
    Empty,
    #[error("Username cannot have more than 100 characters")]
    TooLong,
}

/// Peer username. Has between 1 and 100 characters.
#[repr(transparent)]
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
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
            0 => Err(UsernameError::Empty),
            1..=100 => Ok(Self(value)),
            _ => Err(UsernameError::TooLong),
        }
    }
}

impl Deref for Username {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::fmt::Display for Username {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Peer to peer network user.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
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

impl std::fmt::Display for Peer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{name} ({})", self.address.ip()),
            None => write!(f, "{}", self.address),
        }
    }
}

/// Error in creating a message.
#[derive(Debug, Error)]
pub enum UserMessageError {
    #[error("Message cannot have more than 1000 characters")]
    TooLong,
}

/// Message sent between peers. Has between less than 1000 characters.
#[repr(transparent)]
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct UserMessage(String);

impl Deref for UserMessage {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<UserMessage> for String {
    fn from(value: UserMessage) -> Self {
        value.0
    }
}

impl TryFrom<String> for UserMessage {
    type Error = UserMessageError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() <= 1000 {
            Ok(Self(value))
        } else {
            Err(UserMessageError::TooLong)
        }
    }
}

impl std::fmt::Display for UserMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TextBuffer for UserMessage {
    fn is_mutable(&self) -> bool {
        true
    }

    fn as_str(&self) -> &str {
        self.0.as_str()
    }

    fn insert_text(&mut self, text: &str, char_index: usize) -> usize {
        let text = &text[..text.len().min(1000 - self.0.len())];
        self.0.insert_text(text, char_index)
    }

    fn delete_char_range(&mut self, char_range: Range<usize>) {
        self.0.delete_char_range(char_range);
    }
}
