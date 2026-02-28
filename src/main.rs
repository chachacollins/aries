mod gemini;
fn main() {
    let res = gemini::request("gemini://geminiprotocol.net/").unwrap();
    println!("{res}");
}
