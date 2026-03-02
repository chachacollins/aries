mod aries_logo;
mod gemini;
mod ui;
use std::io;
use crate::ui::App;
fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}
