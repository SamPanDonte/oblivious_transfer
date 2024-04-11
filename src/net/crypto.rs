use libaes::Cipher;
use p256::elliptic_curve::{sec1::ToEncodedPoint, Field};
use p256::{ProjectivePoint as CurvePoint, Scalar};
use rand::{random, thread_rng};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::UserMessage;

/// Error in cryptography protocol.
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Received incorrect message type")]
    InvalidMessage,
    #[error("Received invalid curve point")]
    InvalidPoint,
}

/// State of the connection cryptography.
#[derive(Debug)]
pub(super) enum MessageState {
    GreetSent(Scalar, CurvePoint, UserMessage, UserMessage),
    GreetReceived([u8; 32], bool),
}

impl MessageState {
    /// Handle messages sent by the client.
    pub fn send_message(m0: UserMessage, m1: UserMessage, a: Option<Scalar>) -> (CurvePoint, Self) {
        let a = a.unwrap_or_else(|| Scalar::random(thread_rng()));
        let point = CurvePoint::GENERATOR * a;
        (point, MessageState::GreetSent(a, point, m0, m1))
    }

    /// On greeting message.
    pub fn on_greeting(point: CurvePoint) -> (CurvePoint, Self) {
        let b = Scalar::random(thread_rng());
        let c = random();

        let response = if c {
            point + CurvePoint::GENERATOR * b
        } else {
            CurvePoint::GENERATOR * b
        };

        (response, Self::GreetReceived(into_key(point * b), c))
    }

    /// On greeting response.
    pub fn on_response(self, other: CurvePoint) -> Result<(Vec<u8>, Vec<u8>), CryptoError> {
        match self {
            MessageState::GreetSent(a, point, m0, m1) => {
                let key0 = into_key(other * a);
                let key1 = into_key((other - point) * a);
                Ok((
                    Cipher::new_256(&key0).cbc_encrypt(&key0, m0.as_bytes()),
                    Cipher::new_256(&key1).cbc_encrypt(&key1, m1.as_bytes()),
                ))
            }
            MessageState::GreetReceived(_, _) => Err(CryptoError::InvalidMessage),
        }
    }

    /// On messages received.
    pub fn on_messages(self, m0: Vec<u8>, m1: Vec<u8>) -> Result<String, CryptoError> {
        match self {
            MessageState::GreetSent(_, _, _, _) => Err(CryptoError::InvalidMessage),
            MessageState::GreetReceived(key, c) => {
                let ciphertext = if c { m1 } else { m0 };
                let decoded = Cipher::new_256(&key).cbc_decrypt(&key, &ciphertext);
                String::from_utf8(decoded).map_err(|_| CryptoError::InvalidMessage)
            }
        }
    }
}

fn into_key(point: CurvePoint) -> [u8; 32] {
    Sha256::digest(point.to_encoded_point(false).as_bytes()).into()
}
