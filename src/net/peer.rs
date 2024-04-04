use std::thread::JoinHandle;

use eframe::egui::Context;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use super::{Action, Event, NetworkError, NetworkTask, Username};

static CHANNEL_SIZE: usize = 100;

/// Peer to peer network implementation.
pub struct NetworkHost {
    join_handle: JoinHandle<()>,
    receiver: Receiver<Event>,
    sender: Sender<Action>,
}

impl NetworkHost {
    /// Create a new network host.
    pub fn new(context: Context, name: Username, port: u16) -> Self {
        let (sender, action_receiver) = channel(CHANNEL_SIZE);
        let (event_sender, receiver) = channel(CHANNEL_SIZE);
        let task = NetworkTask::new(action_receiver, event_sender, name, context, port);
        let join_handle = std::thread::spawn(move || task.run());

        Self {
            join_handle,
            receiver,
            sender,
        }
    }

    /// Broadcast message to receive peers.
    pub fn refresh_hosts(&self) -> Result<(), NetworkError> {
        println!("Refreshing hosts");
        Ok(self.sender.blocking_send(Action::Broadcast)?)
    }

    /// Disconnect from network and clean up resources.
    pub fn disconnect(self) -> Result<(), NetworkError> {
        if !self.sender.is_closed() {
            self.sender.blocking_send(Action::Disconnect)?;
        }
        self.join_handle
            .join()
            .map_err(|_| NetworkError::TaskPanic)?;
        Ok(())
    }

    /// Poll for network events.
    pub fn poll_event(&mut self) -> Option<Event> {
        self.receiver.try_recv().ok()
    }
}
