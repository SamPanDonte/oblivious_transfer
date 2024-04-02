use thiserror::Error;

static MAGIC_NUMBER: &[u8] = b"OTMP"; // Oblivious Transfer Message Protocol

/// Protocol message parse error.
#[derive(Debug, Error)]
pub enum ProtocolMessageParseError {
    #[error("Magic number is missing")]
    MissingMagicNumber,
    #[error("Magic number is invalid")]
    InvalidMagicNumber,
    #[error("Message type is missing")]
    MissingMessageType,
    #[error("Message type is invalid")]
    InvalidMessageType,
    #[error("Message length is missing")]
    MissingMessageLength,
    #[error("Message length is invalid")]
    InvalidMessageLength,
    #[error("Message is invalid UTF-8: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("Broadcast name is empty")]
    BroadcastNameEmpty,
    #[error("Broadcast name is too long")]
    BroadcastNameTooLong,
}

/// Protocol messages.
pub enum ProtocolMessage {
    Broadcast(String),
}

impl From<ProtocolMessage> for Vec<u8> {
    fn from(value: ProtocolMessage) -> Self {
        match value {
            ProtocolMessage::Broadcast(message) => {
                let mut buffer = Vec::with_capacity(4 + 1 + 1 + message.len());
                buffer.extend_from_slice(MAGIC_NUMBER);
                buffer.push(0);
                buffer.push(message.len() as u8);
                buffer.extend_from_slice(message.as_bytes());
                buffer
            }
        }
    }
}

impl TryFrom<&[u8]> for ProtocolMessage {
    type Error = ProtocolMessageParseError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if value.len() < 4 {
            return Err(ProtocolMessageParseError::MissingMagicNumber);
        }

        if &value[..4] != MAGIC_NUMBER {
            return Err(ProtocolMessageParseError::InvalidMagicNumber);
        }

        if value.len() < 5 {
            return Err(ProtocolMessageParseError::MissingMessageType);
        }

        if value.len() < 7 {
            return Err(ProtocolMessageParseError::MissingMessageLength);
        }

        let size = usize::from_be_bytes([0, 0, 0, 0, 0, 0, value[5], value[6]]);

        if value.len() != 7 + size {
            return Err(ProtocolMessageParseError::InvalidMessageLength);
        }

        match value[4] {
            0 => match size {
                0 => Err(ProtocolMessageParseError::BroadcastNameEmpty),
                1..=100 => Ok(ProtocolMessage::Broadcast(String::from_utf8(
                    value[7..].to_vec(),
                )?)),
                _ => Err(ProtocolMessageParseError::BroadcastNameTooLong),
            },
            _ => Err(ProtocolMessageParseError::InvalidMessageType),
        }
    }
}
