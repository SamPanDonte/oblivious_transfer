use eframe::egui::Ui;
use local_ip_address::local_ip;
use tracing::error;

use crate::net::{NetworkError, NetworkHost, Username};
use crate::UiContext;

static PORT: u16 = 12345;

/// The top panel of the GUI.
#[derive(Debug, Default)]
pub struct TopPanel(TopPanelInner);

#[derive(Debug)]
enum TopPanelInner {
    Network(NetworkHost),
    Username(String),
}

enum Action {
    None,
    Connect(Username),
    Disconnect(String),
}

impl TopPanel {
    /// Draw the top panel of the GUI.
    pub fn draw(&mut self, ui: &mut Ui) -> Result<(), NetworkError> {
        let mut action = Action::None;
        ui.horizontal(|ui| match &mut self.0 {
            TopPanelInner::Network(network_host) => {
                let name = network_host.name();
                let ip = local_ip()
                    .map(|ip| ip.to_string())
                    .unwrap_or("Cannot find address".to_string());

                ui.label(format!("Connected as: {name} ({ip})"));
                if ui.button("Disconnect").clicked() {
                    action = Action::Disconnect(name.to_string());
                }
            }
            TopPanelInner::Username(username) => {
                ui.label("Username:");
                ui.text_edit_singleline(username);
                ui.set_enabled(Username::try_from(username.clone()).is_ok());
                if ui.button("Connect").clicked() {
                    let mut name = String::new();
                    std::mem::swap(username, &mut name);
                    action = Action::Connect(Username::try_from(name).unwrap());
                }
            }
        });

        match action {
            Action::Connect(username) => {
                let ctx = UiContext::new(ui.ctx().clone());
                self.0 = TopPanelInner::Network(NetworkHost::new(ctx, username, PORT));
            }
            Action::Disconnect(username) => {
                let mut inner = TopPanelInner::Username(username);
                std::mem::swap(&mut self.0, &mut inner);
                if let TopPanelInner::Network(network_host) = inner {
                    network_host.disconnect()?;
                }
            }
            Action::None => {}
        }

        Ok(())
    }

    /// Get the network host if it is connected.
    pub fn get_network_host(&mut self) -> Option<&mut NetworkHost> {
        if let TopPanelInner::Network(network_host) = &mut self.0 {
            Some(network_host)
        } else {
            None
        }
    }

    /// Clean up resources on exit.
    pub fn on_exit(&mut self) {
        let mut host = TopPanelInner::Username(String::new());
        std::mem::swap(&mut host, &mut self.0);
        if let TopPanelInner::Network(host) = host {
            if let Err(err) = host.disconnect() {
                error!("{err}");
            }
        }
    }
}

impl Default for TopPanelInner {
    fn default() -> Self {
        Self::Username(Default::default())
    }
}
