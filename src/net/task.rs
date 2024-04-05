use std::net::SocketAddr;

use eframe::egui::Context;
use local_ip_address::local_ip;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, warn};

use super::{Action, Event, Message, NetworkError, OTMPSocket, Peer, Username};

#[derive(Debug)]
pub(super) struct NetworkTask {
    receiver: Receiver<Action>,
    sender: Sender<Event>,
    context: Context,
    name: Username,
    port: u16,
}

impl NetworkTask {
    /// Create a new network task.
    pub fn new(
        receiver: Receiver<Action>,
        sender: Sender<Event>,
        name: Username,
        context: Context,
        port: u16,
    ) -> Self {
        Self {
            receiver,
            sender,
            context,
            name,
            port,
        }
    }

    /// Run task blocking current thread.
    #[tokio::main(flavor = "current_thread")]
    pub async fn run(mut self) {
        let socket = match OTMPSocket::bind(self.port).await {
            Ok(socket) => socket,
            Err(error) => {
                warn!("Unable to create socket: {error}");
                self.send_error(NetworkError::SocketBindError(error)).await;
                return;
            }
        };

        loop {
            let result = select! {
                result = socket.recv_from() => match result {
                    Ok((message, sender)) => self.on_packet(&socket, message, sender).await,
                    Err(error) => Err(error)
                },
                action = self.receiver.recv() => match action {
                    Some(action) => self.on_action(&socket, action).await,
                    None => {
                        error!("Action channel closed before disconnect");
                        break;
                    }
                }
            };

            if let Err(error) = result {
                self.send_error(error).await;
            }
        }
    }

    async fn send_error(&self, error: NetworkError) {
        self.send_event(Event::Error(error)).await;
    }

    async fn send_event(&self, event: Event) {
        if let Err(send_error) = self.sender.send(event).await {
            error!("Failed to send error event: {send_error}");
        }
        self.context.request_repaint();
    }

    async fn on_packet(
        &mut self,
        socket: &OTMPSocket,
        message: Message,
        sender: SocketAddr,
    ) -> Result<(), NetworkError> {
        match message {
            Message::BroadcastGreet(name) => {
                if local_ip()? == sender.ip() {
                    return Ok(());
                }

                let peer = Peer::new_with_name(sender, name);
                self.send_event(Event::Connected(peer)).await;

                let message = Message::BroadcastResponse(self.name.clone());
                socket.send_to(message, sender).await?;

                Ok(())
            }
            Message::BroadcastResponse(name) => {
                let peer = Peer::new_with_name(sender, name);
                self.send_event(Event::Connected(peer)).await;
                Ok(())
            }
            Message::BroadcastBye => {
                if local_ip()? == sender.ip() {
                    return Ok(());
                }

                self.send_event(Event::Disconnected(sender)).await;
                Ok(())
            }
        }
    }

    async fn on_action(&mut self, socket: &OTMPSocket, action: Action) -> Result<(), NetworkError> {
        match action {
            Action::Broadcast => {
                let message = Message::BroadcastGreet(self.name.clone());
                socket.broadcast(message).await
            }
            Action::Disconnect => socket.broadcast(Message::BroadcastBye).await,
        }
    }
}
