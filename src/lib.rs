#[cfg(feature = "gui")]
pub use gui::run;
#[cfg(feature = "tui")]
pub use tui::run;

#[cfg(all(feature = "gui", feature = "tui"))]
compile_error!("features `gui` and `tui` are mutually exclusive");

#[cfg(feature = "gui")]
mod gui;
mod net;
#[cfg(feature = "tui")]
mod tui;

#[derive(Debug)]
struct UiContext {
    #[cfg(feature = "gui")]
    ctx: eframe::egui::Context,
}

impl UiContext {
    #[cfg(feature = "gui")]
    fn new(ctx: eframe::egui::Context) -> Self {
        Self { ctx }
    }

    #[cfg(feature = "gui")]
    fn request_repaint(&self) {
        self.ctx.request_repaint();
    }

    #[cfg(feature = "tui")]
    fn request_repaint(&self) {}
}
