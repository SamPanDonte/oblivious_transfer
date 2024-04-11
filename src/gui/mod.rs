use app::*;
use demo_pane::*;
pub use message_panel::*;
pub use peer_panel::*;
pub use top_panel::*;

mod app;
mod demo_pane;
mod message_panel;
mod peer_panel;
mod top_panel;

/// Run app.
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    eframe::run_native(
        "Oblivious Transfer Protocol",
        Default::default(),
        Box::new(|_| Box::<App>::default()),
    )?;
    Ok(())
}
