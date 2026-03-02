mod aries_logo;
mod gemini;
//mod ui;
use std::io;
//use crate::ui::App;
fn main() -> io::Result<()> {
    //ratatui::run(|terminal| App::default().run(terminal))
    let res = gemini::make_request("gemini://geminiprotocol.net/").unwrap();
    println!("{res}");
    Ok(())
}
