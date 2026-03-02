use openssl::ssl::{SslMethod, SslConnector,SslVerifyMode};
mod aries_logo;
mod gemini;
mod ui;
use std::io;
use crate::ui::App;
fn main() -> io::Result<()> {
    let mut configure = SslConnector::builder(SslMethod::tls()).unwrap();
    //NOTE: gemini uses TOFU which I'm too lazy to implement
    configure.set_verify(SslVerifyMode::NONE);
    let connector = configure.build();
    ratatui::run(|terminal| App::new(connector).run(terminal))
}
