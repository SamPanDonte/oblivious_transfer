use std::error::Error;

use eframe::egui::{Align2, CentralPanel, Pos2, SidePanel, TopBottomPanel, WidgetText};
use eframe::glow::Context;
use eframe::{egui, Frame};
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use tracing::error;

use crate::net::{Event, Peer};

use super::{MessagePanel, PeerPanel, PeerPanelAction, TopPanel};

/// Gui application.
pub struct App {
    message_panel: MessagePanel,
    peer_panel: PeerPanel,
    top_panel: TopPanel,
    toast: Toasts,
}

impl Default for App {
    fn default() -> Self {
        Self {
            message_panel: Default::default(),
            peer_panel: Default::default(),
            top_panel: Default::default(),
            toast: Toasts::new().anchor(Align2::RIGHT_BOTTOM, Pos2::new(-10.0, -10.0)),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut Frame) {
        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            if let Err(err) = self.top_panel.draw(ui) {
                show_error(&mut self.toast, err);
            }
        });

        let client = match self.top_panel.get_network_host() {
            Some(client) => client,
            None => {
                self.peer_panel.clear_peers();
                self.message_panel.close_all();
                CentralPanel::default().show(ctx, |ui| self.message_panel.show(ui));
                return;
            }
        };

        while let Some(event) = client.poll_event() {
            match event {
                Event::Error(error) => show_error(&mut self.toast, error),
                Event::Connected(peer) => self.peer_panel.add_peer(peer),
                Event::Disconnected(address) => self.peer_panel.remove_peer(&address),
                Event::Message(addr, message) => {
                    let peer = self.peer_panel.get_peer(&addr).unwrap_or(Peer::new(addr));
                    self.message_panel.on_message(&peer, message.clone());
                    show_toast(&mut self.toast, ToastKind::Success, message);
                }
            }
        }

        SidePanel::left("peer_panel").show(ctx, |ui| match self.peer_panel.draw(ui) {
            PeerPanelAction::PeerClicked(peer) => self.message_panel.open_tile(peer.clone()),
            PeerPanelAction::RefreshPeers => {
                if let Err(err) = client.refresh_hosts() {
                    show_error(&mut self.toast, err);
                }
            }
            PeerPanelAction::None => {}
        });

        let frame = egui::Frame::central_panel(&ctx.style())
            .outer_margin(egui::Margin::default())
            .inner_margin(egui::Margin::default());

        CentralPanel::default().frame(frame).show(ctx, |ui| {
            if let Some((addr, m0, m1, a)) = self.message_panel.show(ui) {
                if let Err(err) = client.send(m0, m1, addr, a) {
                    show_error(&mut self.toast, err);
                }
            }
        });

        self.toast.show(ctx);
    }

    fn on_exit(&mut self, _: Option<&Context>) {
        self.top_panel.on_exit();
    }
}

fn show_error(toasts: &mut Toasts, error: impl Error) {
    error!("{error}");
    show_toast(toasts, ToastKind::Error, error.to_string());
}

fn show_toast(toasts: &mut Toasts, kind: ToastKind, text: impl Into<WidgetText>) {
    toasts.add(Toast {
        kind,
        text: text.into(),
        options: ToastOptions::default().duration_in_seconds(3.0),
    });
}
