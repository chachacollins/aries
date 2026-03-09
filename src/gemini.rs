use openssl::ssl::SslConnector;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::str::Lines;

#[derive(Debug)]
pub struct Heading {
    pub title: String,
    pub level: u8,
}

#[derive(Debug)]
pub struct Link {
    pub alt: Option<String>,
    pub link: String,
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

fn parse_link_line(line: &str) -> LineType {
    let mut link = String::new();
    let mut alt = String::new();
    let mut is_link = true;
    let mut first_whitespace = true;
    for c in line.chars() {
        match c {
            ' ' => {
                if first_whitespace {
                    first_whitespace = false;
                } else {
                    is_link = false;
                }
            }
            _ => {
                if is_link {
                    link.push(c);
                } else {
                    alt.push(c);
                }
            }
        }
    }
    let alt = if alt.len() > 0 { Some(alt) } else { None };
    let link = Link { alt, link };
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
                let pref = line_bytes.iter().nth(0).unwrap();
                match pref {
                    b'#' => self.output.push(parse_heading_line(line)),
                    b'=' => self.output.push(parse_link_line(line)),
                    b'*' => self.output.push(parse_list_line(line)),
                    b'>' => self.output.push(parse_quote_line(line)),
                    _ => self.output.push(parse_text_line(line)),
                }
            } else {
                if line_bytes[0..3] == *b"```" {
                    self.state = ParserState::Normal;
                } else {
                    let mut pref_lines = Vec::new();
                    pref_lines.push(line.to_string());
                    self.output.push(LineType::Preformat(pref_lines));
                }
            }
        }
    }

    pub fn get_lines(self) -> Vec<LineType> {
        self.output
    }
}

#[derive(Debug, PartialEq)]
pub enum ReqErr {
    MalformedUrl(String),
    SslConnection,
    HostConnection,
    Read,
    Write,
}

//TODO: refactor this error reporting
pub fn make_request(connector: &SslConnector, url: &str) -> Result<String, ReqErr> {
    if !url.starts_with("gemini://") {
        return Err(ReqErr::MalformedUrl(url.to_string()));
    }
    let hostname = match url.strip_prefix("gemini://").unwrap().split('/').next() {
        Some(h) => h,
        None => return Err(ReqErr::MalformedUrl(url.to_string())),
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
    if let Err(_) = stream.read_to_end(&mut res) {
        return Err(ReqErr::Read);
    }
    Ok(String::from_utf8_lossy(&res).to_string())
}
