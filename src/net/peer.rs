use std::net::SocketAddr;
use std::thread::{JoinHandle, spawn};

use eframe::egui::Context;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tracing::error;

use super::{Action, Event, NetworkError, NetworkTask, Result, UserMessage, Username};

static CHANNEL_SIZE: usize = 100;

/// Peer to peer network implementation.
#[derive(Debug)]
pub struct NetworkHost {
    join_handle: JoinHandle<()>,
    receiver: Receiver<Event>,
    sender: Sender<Action>,
    name: Username,
}

impl NetworkHost {
    /// Create a new network host.
    pub fn new(ctx: Context, name: Username, port: u16) -> Self {
        let (sender, action) = channel(CHANNEL_SIZE);
        let (event, receiver) = channel(CHANNEL_SIZE);
        let username = name.clone();
        let join_handle = spawn(move || NetworkTask::run(action, event, username, ctx, port));

        if let Err(error) = sender.blocking_send(Action::Broadcast) {
            error!("Failed to send initial broadcast event: {}", error);
        }

        Self {
            join_handle,
            receiver,
            sender,
            name,
        }
    }

    /// Broadcast message to receive peers.
    pub fn refresh_hosts(&self) -> Result<()> {
        Ok(self.sender.blocking_send(Action::Broadcast)?)
    }

    /// Disconnect from network and clean up resources.
    pub fn disconnect(self) -> Result<()> {
        if !self.sender.is_closed() {
            self.sender.blocking_send(Action::Disconnect)?;
        }
        self.join_handle
            .join()
            .map_err(|_| NetworkError::TaskPanic)?;
        Ok(())
    }

    /// Send a message to address.
    pub fn send(&mut self, m0: UserMessage, m1: UserMessage, addr: SocketAddr) -> Result<()> {
        self.sender.blocking_send(Action::Send(addr, m0, m1))?;
        Ok(())
    }

    /// Poll for network events.
    pub fn poll_event(&mut self) -> Option<Event> {
        self.receiver.try_recv().ok()
    }

    /// Get the username of the network host.
    pub fn name(&self) -> &str {
        &self.name
    }
}
