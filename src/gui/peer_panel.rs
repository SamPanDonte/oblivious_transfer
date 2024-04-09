use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::str::FromStr;

use eframe::egui::{Button, ScrollArea, TextEdit, Ui, Vec2, Widget};

use crate::net::Peer;

/// Panel that shows the list of peers.
#[derive(Debug, Default)]
pub struct PeerPanel(BTreeMap<SocketAddr, Peer>, String);

/// Actions that can be performed on the peer panel.
pub enum PeerPanelAction<'a> {
    PeerClicked(&'a Peer),
    RefreshPeers,
    None,
}

impl PeerPanel {
    /// Draw the peer panel. Returns the peer that was clicked.
    pub fn draw(&mut self, ui: &mut Ui) -> PeerPanelAction {
        let mut action = PeerPanelAction::None;

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if ui.button("↻").on_hover_text("Refresh peers").clicked() {
                    action = PeerPanelAction::RefreshPeers;
                    self.clear_peers();
                }
                ui.label("Peers");
            });

            ui.horizontal(|ui| {
                let enabled = SocketAddr::from_str(&self.1).is_ok();
                if ui.add_enabled(enabled, Button::new("Add")).clicked() {
                    self.add_peer(Peer::new(SocketAddr::from_str(&self.1).unwrap()));
                    self.1.clear();
                }
                TextEdit::singleline(&mut self.1)
                    .hint_text("Peer address")
                    .desired_width(ui.available_width())
                    .ui(ui);
            });

            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                let size = Vec2::new(ui.available_width(), 0.0);
                for peer in self.0.values() {
                    let button = Button::new(peer.to_string()).frame(false).min_size(size);
                    if button.ui(ui).clicked() {
                        action = PeerPanelAction::PeerClicked(peer);
                    }
                }
            });
        });

        action
    }

    /// Add a peer to the panel.
    pub fn add_peer(&mut self, peer: Peer) {
        self.0.insert(peer.address(), peer);
    }

    /// Remove a peer from the panel.
    pub fn remove_peer(&mut self, address: &SocketAddr) {
        self.0.remove(address);
    }

    /// Clear all peers from the panel.
    pub fn clear_peers(&mut self) {
        self.0.clear();
    }

    /// Get peer by socket address.
    pub fn get_peer(&self, addr: &SocketAddr) -> Option<Peer> {
        self.0.get(addr).cloned()
    }
}
