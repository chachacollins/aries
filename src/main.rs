use openssl::ssl::{SslConnector, SslMethod, SslVerifyMode};
mod aries_art;
mod gemini;
mod ui;
use crate::ui::App;
use std::io;
fn main() -> io::Result<()> {
    let mut configure = SslConnector::builder(SslMethod::tls()).unwrap();
    //NOTE: gemini uses TOFU which I'm too lazy to implement
    configure.set_verify(SslVerifyMode::NONE);
    let connector = configure.build();
    ratatui::run(|terminal| App::new(connector).run(terminal))
}
