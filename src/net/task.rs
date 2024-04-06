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
    socket: OTMPSocket,
    context: Context,
    name: Username,
}

impl NetworkTask {
    /// Run task blocking current thread.
    #[tokio::main(flavor = "current_thread")]
    pub async fn run(
        receiver: Receiver<Action>,
        sender: Sender<Event>,
        name: Username,
        context: Context,
        port: u16,
    ) {
        let socket = match OTMPSocket::bind(port).await {
            Ok(socket) => socket,
            Err(error) => {
                warn!("Unable to create socket: {error}");
                send_event(&sender, Event::Error(NetworkError::SocketBindError(error))).await;
                return;
            }
        };

        let task = Self {
            receiver,
            sender,
            socket,
            context,
            name,
        };

        task.main_loop().await;
    }

    async fn main_loop(mut self) {
        let mut running = true;
        while running {
            let result = select! {
                result = self.socket.recv_from() => match result {
                    Ok((message, sender)) => self.on_packet(message, sender).await,
                    Err(error) => Err(error)
                },
                action = self.receiver.recv() => match action {
                    Some(action) => {
                        if let Action::Disconnect = action {
                            running = false;
                        }
                        self.on_action(action).await
                    }
                    None => {
                        error!("Action channel closed before disconnect");
                        running = false;
                        Ok(())
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
        send_event(&self.sender, event).await;
        self.context.request_repaint();
    }

    async fn on_packet(&self, message: Message, sender: SocketAddr) -> Result<(), NetworkError> {
        match message {
            Message::BroadcastGreet(name) => {
                if local_ip()? == sender.ip() {
                    let peer = Peer::new_with_name(sender, name);
                    self.send_event(Event::Connected(peer)).await;

                    let message = Message::BroadcastResponse(self.name.clone());
                    self.socket.send_to(message, sender).await?;
                }
                Ok(())
            }
            Message::BroadcastResponse(name) => {
                let peer = Peer::new_with_name(sender, name);
                self.send_event(Event::Connected(peer)).await;
                Ok(())
            }
            Message::BroadcastBye => {
                if local_ip()? != sender.ip() {
                    self.send_event(Event::Disconnected(sender)).await;
                }
                Ok(())
            }
        }
    }

    async fn on_action(&self, action: Action) -> Result<(), NetworkError> {
        match action {
            Action::Broadcast => {
                let message = Message::BroadcastGreet(self.name.clone());
                self.socket.broadcast(message).await
            }
            Action::Disconnect => self.socket.broadcast(Message::BroadcastBye).await,
        }
    }
}

async fn send_event(sender: &Sender<Event>, event: Event) {
    if let Err(send_error) = sender.send(event).await {
        error!("Failed to send error event: {send_error}");
    }
}
