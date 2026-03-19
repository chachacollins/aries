use openssl::ssl::SslConnector;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::Lines;

#[derive(Debug)]
pub struct Heading {
    pub title: String,
    pub level: u8,
}

#[derive(Debug, Clone)]
pub struct Link {
    pub alt: Option<String>,
    pub link: String,
    pub f_char: String,
    pub is_relative: bool,
}

#[derive(Debug)]
pub enum LineType {
    Text(String),
    Link(Link),
    Heading(Heading),
    List(String),
    Quote(String),
    Preformat(Vec<String>),
}

#[derive(PartialEq)]
enum ParserState {
    Normal,
    Preformated,
}

pub struct Parser<'a> {
    lines: Lines<'a>,
    output: Vec<LineType>,
    state: ParserState,
}

fn parse_heading_line(line: &str) -> LineType {
    if line.starts_with("###") {
        LineType::Heading(Heading {
            level: 3,
            title: line.to_string(),
        })
    } else if line.starts_with("##") {
        LineType::Heading(Heading {
            level: 2,
            title: line.to_string(),
        })
    } else {
        LineType::Heading(Heading {
            level: 1,
            title: line.to_string(),
        })
    }
}

const F_CHARS: [&str; 300] = [
    "ab", "ac", "bc", "ad", "bd", "cd", "ae", "be", "ce", "de", "af", "bf", "cf", "df", "ef", "ag",
    "bg", "cg", "dg", "eg", "fg", "ah", "bh", "ch", "dh", "eh", "fh", "gh", "ai", "bi", "ci", "di",
    "ei", "fi", "gi", "hi", "aj", "bj", "cj", "dj", "ej", "fj", "gj", "hj", "ij", "ak", "bk", "ck",
    "dk", "ek", "fk", "gk", "hk", "ik", "jk", "al", "bl", "cl", "dl", "el", "fl", "gl", "hl", "il",
    "jl", "kl", "am", "bm", "cm", "dm", "em", "fm", "gm", "hm", "im", "jm", "km", "lm", "an", "bn",
    "cn", "dn", "en", "fn", "gn", "hn", "in", "jn", "kn", "ln", "mn", "ao", "bo", "co", "do", "eo",
    "fo", "go", "ho", "io", "jo", "ko", "lo", "mo", "no", "ap", "bp", "cp", "dp", "ep", "fp", "gp",
    "hp", "ip", "jp", "kp", "lp", "mp", "np", "op", "aq", "bq", "cq", "dq", "eq", "fq", "gq", "hq",
    "iq", "jq", "kq", "lq", "mq", "nq", "oq", "pq", "ar", "br", "cr", "dr", "er", "fr", "gr", "hr",
    "ir", "jr", "kr", "lr", "mr", "nr", "or", "pr", "qr", "as", "bs", "cs", "ds", "es", "fs", "gs",
    "hs", "is", "js", "ks", "ls", "ms", "ns", "os", "ps", "qs", "rs", "at", "bt", "ct", "dt", "et",
    "ft", "gt", "ht", "it", "jt", "kt", "lt", "mt", "nt", "ot", "pt", "qt", "rt", "st", "au", "bu",
    "cu", "du", "eu", "fu", "gu", "hu", "iu", "ju", "ku", "lu", "mu", "nu", "ou", "pu", "qu", "ru",
    "su", "tu", "av", "bv", "cv", "dv", "ev", "fv", "gv", "hv", "iv", "jv", "kv", "lv", "mv", "nv",
    "ov", "pv", "qv", "rv", "sv", "tv", "uv", "aw", "bw", "cw", "dw", "ew", "fw", "gw", "hw", "iw",
    "jw", "kw", "lw", "mw", "nw", "ow", "pw", "qw", "rw", "sw", "tw", "uw", "vw", "ax", "bx", "cx",
    "dx", "ex", "fx", "gx", "hx", "ix", "jx", "kx", "lx", "mx", "nx", "ox", "px", "qx", "rx", "sx",
    "tx", "ux", "vx", "wx", "ay", "by", "cy", "dy", "ey", "fy", "gy", "hy", "iy", "jy", "ky", "ly",
    "my", "ny", "oy", "py", "qy", "ry", "sy", "ty", "uy", "vy", "wy", "xy",
];

fn parse_link_line(line: &str, f_char_i: usize) -> LineType {
    let line = line.strip_prefix("=>").unwrap();
    let mut link = String::new();
    let mut alt = String::new();
    let parts: Vec<_> = line.split_whitespace().collect();
    if !parts.is_empty() {
        link = parts[0].to_string();
    }
    if parts.len() > 1 {
        alt = parts[1..].join(" ");
    }
    let alt = if !alt.is_empty() { Some(alt) } else { None };
    let is_relative = !link.starts_with("gemini://");
    let link = Link {
        alt,
        link,
        f_char: F_CHARS[f_char_i].to_string(),
        is_relative,
    };
    LineType::Link(link)
}

fn parse_quote_line(line: &str) -> LineType {
    LineType::Quote(line.to_string())
}

fn parse_text_line(line: &str) -> LineType {
    LineType::Text(line.to_string())
}

fn parse_list_line(line: &str) -> LineType {
    LineType::List(line.to_string())
}

impl<'a> Parser<'a> {
    pub fn new(gemtext: &'a str) -> Self {
        Self {
            lines: gemtext.lines(),
            output: Vec::new(),
            state: ParserState::Normal,
        }
    }

    pub fn parse_gemtext(&mut self) {
        //TODO: handle statuses
        let _status_line = self
            .lines
            .next()
            .expect("The server should always return a status line");
        let mut f_char_i = 0;
        for line in self.lines.by_ref() {
            let line_bytes = line.as_bytes();
            if line.len() < 3 {
                self.output.push(LineType::Text(String::new()));
                continue;
            }
            if self.state == ParserState::Normal {
                if line_bytes[0..3] == *b"```" {
                    self.state = ParserState::Preformated;
                    continue;
                }
                let pref = line_bytes.first().unwrap();
                match pref {
                    b'#' => self.output.push(parse_heading_line(line)),
                    b'=' => {
                        self.output.push(parse_link_line(line, f_char_i));
                        f_char_i += 1;
                    }
                    b'*' => self.output.push(parse_list_line(line)),
                    b'>' => self.output.push(parse_quote_line(line)),
                    _ => self.output.push(parse_text_line(line)),
                }
            } else if line_bytes[0..3] == *b"```" {
                self.state = ParserState::Normal;
            } else {
                let pref_lines = vec![line.to_string()];
                self.output.push(LineType::Preformat(pref_lines));
            }
        }
    }

    pub fn get_lines(self) -> Vec<LineType> {
        self.output
    }
}

#[derive(Debug, PartialEq)]
pub enum ReqErr {
    MalformedUrl,
    SslConnection,
    HostConnection,
    Read,
    Write,
}

//TODO: refactor this error reporting
pub fn make_request(connector: &SslConnector, url: &str) -> Result<String, ReqErr> {
    if !url.starts_with("gemini://") {
        return Err(ReqErr::MalformedUrl);
    }
    let hostname = match url.strip_prefix("gemini://").unwrap().split('/').next() {
        Some(h) => h,
        None => return Err(ReqErr::MalformedUrl),
    };
    let stream = match TcpStream::connect(format!("{hostname}:1965")) {
        Ok(s) => s,
        Err(_) => return Err(ReqErr::HostConnection),
    };
    let mut stream = match connector.connect(hostname, stream) {
        Ok(s) => s,
        Err(_) => return Err(ReqErr::SslConnection),
    };
    match stream.write_all(format!("{url}\r\n").as_bytes()) {
        Ok(_) => {}
        Err(_) => return Err(ReqErr::Write),
    }
    let mut res = vec![];
    if stream.read_to_end(&mut res).is_err() {
        return Err(ReqErr::Read);
    }
    Ok(String::from_utf8_lossy(&res).to_string())
}
