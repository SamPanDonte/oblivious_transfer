use eframe::{egui, Frame};
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use tracing::{error, Level};

use oblivious_transfer::gui::{MessagePanel, PeerPanel, PeerPanelAction, TopPanel};
use oblivious_transfer::net::{Event, NetworkError, Peer};

#[derive(Default)]
struct Application {
    message_panel: MessagePanel,
    peer_panel: PeerPanel,
    top_panel: TopPanel,
    toast: Toasts,
}

impl eframe::App for Application {
    fn update(&mut self, ctx: &egui::Context, _: &mut Frame) {
        if let Some(host) = self.top_panel.get_network_host() {
            while let Some(event) = host.poll_event() {
                match event {
                    Event::Error(error) => {
                        self.toast.add(Toast {
                            kind: ToastKind::Error,
                            text: error.to_string().into(),
                            options: ToastOptions::default().duration_in_seconds(3.0),
                        });
                        if let NetworkError::SocketBindError(error) = error {
                            error!("Unable to connect: {error}");
                        }
                    }
                    Event::Connected(peer) => self.peer_panel.add_peer(peer),
                    Event::Disconnected(address) => self.peer_panel.remove_peer(&address),
                    Event::Message(addr, message) => {
                        let peer = self.peer_panel.get_peer(&addr).unwrap_or(Peer::new(addr));
                        self.message_panel.on_message(&peer, message.clone());
                        self.toast.add(Toast {
                            kind: ToastKind::Success,
                            text: message.into(),
                            options: ToastOptions::default().duration_in_seconds(3.0),
                        });
                    }
                }
            }
        }

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            if let Err(error) = self.top_panel.draw(ui) {
                self.toast.add(Toast {
                    kind: ToastKind::Error,
                    text: error.to_string().into(),
                    options: ToastOptions::default().duration_in_seconds(3.0),
                });
            }
        });

        if self.top_panel.get_network_host().is_some() {
            egui::SidePanel::left("peer_panel").show(ctx, |ui| match self.peer_panel.draw(ui) {
                PeerPanelAction::PeerClicked(peer) => {
                    self.message_panel.open_tile(peer.clone());
                }
                PeerPanelAction::RefreshPeers => {
                    if let Some(host) = self.top_panel.get_network_host() {
                        if let Err(error) = host.refresh_hosts() {
                            self.toast.add(Toast {
                                kind: ToastKind::Error,
                                text: error.to_string().into(),
                                options: ToastOptions::default().duration_in_seconds(3.0),
                            });
                        }
                    }
                }
                PeerPanelAction::None => {}
            });
        } else {
            self.peer_panel.clear_peers();
            self.message_panel.close_all();
        }

        let frame = egui::Frame::central_panel(&ctx.style())
            .outer_margin(egui::Margin::default())
            .inner_margin(egui::Margin::default());

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            if let Some((addr, m0, m1)) = self.message_panel.show(ui) {
                if let Some(host) = self.top_panel.get_network_host() {
                    if let Err(error) = host.send(m0, m1, addr) {
                        self.toast.add(Toast {
                            kind: ToastKind::Error,
                            text: error.to_string().into(),
                            options: ToastOptions::default().duration_in_seconds(3.0),
                        });
                    }
                }
            }
        });

        self.toast.show(ctx);
    }
}

fn main() -> Result<(), eframe::Error> {
    let subscriber = tracing_subscriber::FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish();
    tracing::subscriber::set_global_default(subscriber).unwrap();

    eframe::run_native(
        "Oblivious Transfer Protocol",
        Default::default(),
        Box::new(|_| Box::<Application>::default()),
    )
}
