use std::collections::hash_map::Entry;
use std::net::SocketAddr;

use eframe::egui::ahash::HashMap;
use eframe::egui::{
    Align, CentralPanel, Layout, ScrollArea, TextEdit, TopBottomPanel, Ui, ViewportBuilder,
    ViewportId, Widget, WidgetText,
};
use egui_tiles::{Behavior, SimplificationOptions, Tabs, Tile, TileId, Tiles, Tree, UiResponse};

use crate::net::{Peer, UserMessage};

use super::DemoPane;

/// Panel to display messages. Allows for sending and receiving messages and taking out and in tiles.
#[derive(Debug)]
pub struct MessagePanel {
    messages: HashMap<SocketAddr, Messages>,
    windows: HashMap<TileId, Pane>,
    tree: Tree<Pane>,
    action: Action,
    root: TileId,
}

impl MessagePanel {
    /// Add a message to the panel.
    pub fn on_message(&mut self, peer: &Peer, message: String) {
        let message = Message::Received(message);
        get_entry(&mut self.messages, peer).data.push(message);
    }

    /// Open a tile for the peer.
    pub fn open_tile(&mut self, peer: Peer) {
        let pane = Pane::Message(MessagePane::new(peer));
        let id = self.tree.tiles.insert_pane(pane);
        self.tree.move_tile_to_container(id, self.root, 0, true);
    }

    /// Show the message panel. Returns data if a message is sent or received.
    pub fn show(&mut self, ui: &mut Ui) -> Option<(SocketAddr, UserMessage, UserMessage)> {
        let mut behaviour = Behaviour(&mut self.messages, &mut self.action);
        self.tree.ui(&mut behaviour, ui);
        self.show_windows(ui);

        let mut action = Action::None;
        std::mem::swap(&mut action, &mut self.action);

        match action {
            Action::Send(addr, m0, m1) => Some((addr, m0, m1)),
            Action::CloseWindow(id) => {
                self.windows.remove(&id);
                None
            }
            Action::TakeOut(id) => {
                if let Some(Tile::Pane(pane)) = self.tree.tiles.remove(id) {
                    self.windows.insert(id, pane);
                }
                None
            }
            Action::TakeIn(id) => {
                if let Some(pane) = self.windows.remove(&id) {
                    let id = self.tree.tiles.insert_pane(pane);
                    self.tree.move_tile_to_container(id, self.root, 0, true);
                }
                None
            }
            Action::Close(id) => {
                self.tree.tiles.remove(id);
                None
            }
            Action::None => None,
        }
    }

    /// Close all tiles.
    pub fn close_all(&mut self) {
        let tiles_iter = self.tree.tiles.tiles();
        let ids: Vec<TileId> = tiles_iter
            .filter_map(|tile| {
                if let Tile::Pane(pane) = tile {
                    if let Pane::Message(_) = pane {
                        return self.tree.tiles.find_pane(pane);
                    }
                }
                None
            })
            .collect();
        for id in ids {
            self.tree.tiles.remove(id);
        }
    }

    fn show_windows(&mut self, ui: &mut Ui) {
        for (id, pane) in &mut self.windows {
            let title = format!("Oblivious transfer chat: {}", pane.title());
            ui.ctx().show_viewport_immediate(
                ViewportId::from_hash_of(id),
                ViewportBuilder::default().with_title(title),
                |ctx, _| {
                    TopBottomPanel::top("top_panel").show(ctx, |ui| {
                        ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                            if ui.button("⤵").clicked() {
                                self.action = Action::TakeIn(*id);
                            }
                        });
                    });
                    CentralPanel::default().show(ctx, |ui| {
                        let action = pane.show(ui, *id, &mut self.messages);
                        if let Action::None = self.action {
                            self.action = action;
                        }
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        self.action = Action::CloseWindow(*id);
                    }
                },
            );
        }
    }
}

impl Default for MessagePanel {
    fn default() -> Self {
        let mut tiles = Tiles::default();
        let demo_id = tiles.insert_pane(Pane::Demo(Default::default()));
        let root = tiles.insert_tab_tile(vec![demo_id]);
        let tree = Tree::new("messages_tree", root, tiles);
        Self {
            messages: Default::default(),
            windows: Default::default(),
            tree,
            action: Default::default(),
            root,
        }
    }
}

struct Behaviour<'a>(&'a mut HashMap<SocketAddr, Messages>, &'a mut Action);

impl<'a> Behavior<Pane> for Behaviour<'a> {
    fn pane_ui(&mut self, ui: &mut Ui, id: TileId, pane: &mut Pane) -> UiResponse {
        let action = pane.show(ui, id, self.0);
        if let Action::None = self.1 {
            *self.1 = action;
        }
        UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> WidgetText {
        pane.title().into()
    }

    fn top_bar_right_ui(
        &mut self,
        tiles: &Tiles<Pane>,
        ui: &mut Ui,
        _: TileId,
        tabs: &Tabs,
        _: &mut f32,
    ) {
        if let Some(id) = &tabs.active {
            if let Some(Tile::Pane(Pane::Message(_))) = tiles.get(*id) {
                ui.add_space(8.0);
                if ui.button("✖").clicked() {
                    *self.1 = Action::Close(*id);
                }
                if ui.button("⤴").clicked() {
                    *self.1 = Action::TakeOut(*id);
                }
            }
        }
    }

    fn simplification_options(&self) -> SimplificationOptions {
        SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
}

#[derive(Debug)]
pub struct Messages {
    data: Vec<Message>,
    peer: Peer,
}

impl Messages {
    fn new(peer: Peer) -> Self {
        Self {
            data: Default::default(),
            peer,
        }
    }
}

#[derive(Debug)]
enum Message {
    Received(String),
    Sent(String, String),
}

#[derive(Debug, Eq, PartialEq)]
enum Pane {
    Message(MessagePane),
    Demo(Box<DemoPane>),
}

impl Pane {
    fn show(&mut self, ui: &mut Ui, id: TileId, d: &mut HashMap<SocketAddr, Messages>) -> Action {
        match self {
            Pane::Message(pane) => pane.show(ui, id, get_entry(d, &pane.peer)),
            Pane::Demo(pane) => {
                pane.draw(ui);
                Action::None
            }
        }
    }

    fn title(&self) -> String {
        match self {
            Pane::Message(pane) => pane.peer.to_string(),
            Pane::Demo(_) => "Demo".to_string(),
        }
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
enum Action {
    Send(SocketAddr, UserMessage, UserMessage),
    CloseWindow(TileId),
    TakeOut(TileId),
    TakeIn(TileId),
    Close(TileId),
    #[default]
    None,
}

#[derive(Debug, Eq, PartialEq)]
struct MessagePane {
    peer: Peer,
    m0: UserMessage,
    m1: UserMessage,
}

impl MessagePane {
    fn new(peer: Peer) -> Self {
        Self {
            peer,
            m0: Default::default(),
            m1: Default::default(),
        }
    }
}

impl MessagePane {
    pub(super) fn show(&mut self, ui: &mut Ui, id: TileId, messages: &mut Messages) -> Action {
        let peer = &messages.peer;
        let mut result = Default::default();

        let panel_id = format!("bottom_panel_{peer}_{id:?}");
        TopBottomPanel::bottom(panel_id).show_inside(ui, |ui| {
            ui.with_layout(Layout::right_to_left(Align::BOTTOM), |ui| {
                if ui.button("Send").clicked() {
                    let mut new_m0 = UserMessage::default();
                    let mut new_m1 = UserMessage::default();

                    std::mem::swap(&mut self.m0, &mut new_m0);
                    std::mem::swap(&mut self.m1, &mut new_m1);

                    let message = Message::Sent(new_m0.to_string(), new_m1.to_string());
                    messages.data.push(message);

                    result = Action::Send(peer.address(), new_m0, new_m1);
                }
                ui.vertical(|ui| {
                    TextEdit::singleline(&mut self.m0)
                        .desired_width(ui.available_width())
                        .ui(ui);
                    TextEdit::singleline(&mut self.m1)
                        .desired_width(ui.available_width())
                        .ui(ui);
                });
            });
        });

        ScrollArea::vertical().show(ui, |ui| {
            ui.vertical(|ui| {
                for message in &messages.data {
                    match message {
                        Message::Received(message) => {
                            ui.horizontal(|ui| {
                                ui.label(format!("{peer}:"));
                                ui.label(message);
                                ui.add_space(ui.available_width());
                            });
                        }
                        Message::Sent(m0, m1) => {
                            ui.horizontal(|ui| {
                                ui.label("Me:");
                                ui.vertical(|ui| {
                                    ui.label(m0);
                                    ui.label(m1);
                                });
                                ui.add_space(ui.available_width());
                            });
                        }
                    }
                }
            });
        });

        result
    }
}

fn get_entry<'a>(messages: &'a mut HashMap<SocketAddr, Messages>, peer: &Peer) -> &'a mut Messages {
    match messages.entry(peer.address()) {
        Entry::Occupied(entry) => entry.into_mut(),
        Entry::Vacant(entry) => entry.insert(Messages::new(peer.clone())),
    }
}
