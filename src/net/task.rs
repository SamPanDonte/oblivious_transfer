use std::collections::HashMap;
use std::net::SocketAddr;

use eframe::egui::Context;
use local_ip_address::local_ip;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, warn};

use super::{Action, Event, Message, MessageState, NetworkError, OTMPSocket, Peer, Username};

#[derive(Debug)]
pub(super) struct NetworkTask {
    states: HashMap<SocketAddr, MessageState>,
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
            states: HashMap::new(),
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

    async fn on_packet(&mut self, message: Message, addr: SocketAddr) -> Result<(), NetworkError> {
        match message {
            Message::BroadcastGreet(name) => {
                if local_ip()? != addr.ip() {
                    let peer = Peer::new_with_name(addr, name);
                    self.send_event(Event::Connected(peer)).await;

                    let message = Message::BroadcastResponse(self.name.clone());
                    self.socket.send_to(message, addr).await?;
                }
                Ok(())
            }
            Message::BroadcastResponse(name) => {
                let peer = Peer::new_with_name(addr, name);
                self.send_event(Event::Connected(peer)).await;
                Ok(())
            }
            Message::BroadcastBye => {
                if local_ip()? != addr.ip() {
                    self.send_event(Event::Disconnected(addr)).await;
                }
                Ok(())
            }
            Message::Greet(point) => {
                let (response, state) = MessageState::on_greeting(point);
                self.states.insert(addr, state);
                let response = Message::Response(response);
                self.socket.send_to(response, addr).await?;
                Ok(())
            }
            Message::Response(point) => match self.states.remove(&addr) {
                Some(state) => {
                    let (m0, m1) = state
                        .on_response(point)
                        .map_err(|_| NetworkError::IncorrectMessage(addr))?;
                    self.socket.send_to(Message::Data(m0, m1), addr).await?;
                    Ok(())
                }
                None => Err(NetworkError::IncorrectMessage(addr)),
            },
            Message::Data(m0, m1) => match self.states.remove(&addr) {
                Some(state) => {
                    let message = state
                        .on_messages(m0, m1)
                        .map_err(|_| NetworkError::IncorrectMessage(addr))?;
                    self.send_event(Event::Message(addr, message)).await;
                    Ok(())
                }
                None => Err(NetworkError::IncorrectMessage(addr)),
            },
        }
    }

    async fn on_action(&mut self, action: Action) -> Result<(), NetworkError> {
        match action {
            Action::Broadcast => {
                let message = Message::BroadcastGreet(self.name.clone());
                self.socket.broadcast(message).await
            }
            Action::Disconnect => self.socket.broadcast(Message::BroadcastBye).await,
            Action::Send(addr, m0, m1) => {
                let (message, state) = MessageState::send_message(m0, m1);
                self.states.insert(addr, state);
                self.socket.send_to(Message::Greet(message), addr).await?;
                Ok(())
            }
        }
    }
}

async fn send_event(sender: &Sender<Event>, event: Event) {
    if let Err(send_error) = sender.send(event).await {
        error!("Failed to send error event: {send_error}");
    }
}
