use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use eframe::egui::Context;
use local_ip_address::local_ip;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use tokio::net::UdpSocket;
use tokio::select;
use tokio::sync::mpsc::{Receiver, Sender};
use tracing::{error, instrument, warn};

use super::{Action, Event, Message, NetworkError, Peer, Username};

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
    #[instrument]
    pub async fn run(mut self) {
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), self.port);
        let socket = match UdpSocket::bind(address).await {
            Ok(socket) => socket,
            Err(error) => {
                warn!("Unable to create socket: {error}");
                self.send_error(NetworkError::SocketBindError(error)).await;
                return;
            }
        };

        if let Err(error) = socket.set_broadcast(true) {
            warn!("Unable to set broadcast: {error}");
            self.send_error(NetworkError::SocketBindError(error)).await;
            return;
        }

        let mut buffer = [0; 2048];
        loop {
            let result = select! {
                result = socket.recv_from(&mut buffer) => match result {
                    Ok((size, sender)) => self.on_packet(&socket, &buffer[..size], sender).await,
                    Err(error) => Err(error.into())
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

    #[instrument]
    async fn send_event(&self, event: Event) {
        if let Err(send_error) = self.sender.send(event).await {
            error!("Failed to send error event: {send_error}");
        }
        self.context.request_repaint();
    }
    
    async fn on_packet(
        &mut self,
        socket: &UdpSocket,
        buffer: &[u8],
        sender: SocketAddr,
    ) -> Result<(), NetworkError> {
        let message = Message::try_from(buffer)?;
        match message {
            Message::BroadcastGreet(name) => {
                let peer = Peer::new_with_name(sender, name);
                self.send_event(Event::Connected(peer)).await;

                let message = Message::BroadcastResponse(self.name.clone()).into_bytes();
                socket.send_to(&message, sender).await?;

                Ok(())
            }
            Message::BroadcastResponse(name) => {
                let peer = Peer::new_with_name(sender, name);
                self.send_event(Event::Connected(peer)).await;
                Ok(())
            }
            Message::BroadcastBye => {
                self.send_event(Event::Disconnected(sender)).await;
                Ok(())
            }
        }
    }

    async fn on_action(&mut self, socket: &UdpSocket, action: Action) -> Result<(), NetworkError> {
        match action {
            Action::Broadcast => {
                let message = Message::BroadcastGreet(self.name.clone()).into_bytes();
                socket.send_to(&message, get_broadcast(self.port)?).await?;
                Ok(())
            }
            Action::Disconnect => {
                let message = Message::BroadcastBye.into_bytes();
                socket.send_to(&message, get_broadcast(self.port)?).await?;
                Ok(())
            }
        }
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
