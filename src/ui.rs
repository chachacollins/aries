use crate::aries_logo;
use crate::gemini;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use openssl::ssl::SslConnector;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use std::io;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

#[derive(Debug, PartialEq)]
enum Page {
    Title,
    Browse,
}

const BG_COLOR: Color = Color::Rgb(10, 10, 16);
const FG_COLOR: Color = Color::Rgb(235, 219, 178);
const HEADING1_COLOR: Color = Color::Rgb(234, 105, 98);
const HEADING2_COLOR: Color = Color::Rgb(216, 166, 87);
const HEADING3_COLOR: Color = Color::Rgb(125, 174, 163);
const LINK_COLOR: Color = Color::Rgb(137, 180, 130);
const QUOTE_COLOR: Color = Color::Rgb(146, 131, 116);
const LIST_COLOR: Color = Color::Rgb(130, 131, 116);

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[derive(Debug, PartialEq, Default)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[derive(Default, Debug)]
struct Url {
    input: Input,
    input_mode: InputMode,
    input_area: Rect,
}

#[derive(Debug)]
pub struct App {
    ssl_connection: SslConnector,
    current_page: Page,
    help_triggered: bool,
    event: Event,
    exit: bool,
    url: Url,
    page_content: Vec<gemini::LineType>,
}

impl App {
    pub fn new(ssl_connection: SslConnector) -> Self {
        Self {
            ssl_connection,
            current_page: Page::Title,
            help_triggered: false,
            event: Event::FocusLost,
            url: Url::default(),
            exit: false,
            page_content: Vec::new(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| {
                self.calculate_url_input_size(frame.area());
                self.draw(frame)
            })?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let width = self.url.input_area.width.max(3) - 3;
        let scroll = self.url.input.visual_scroll(width as usize);
        if self.url.input_mode == InputMode::Editing {
            let x = self.url.input.visual_cursor().max(scroll) - scroll + 1;
            frame.set_cursor_position((self.url.input_area.x + x as u16, self.url.input_area.y + 1))
        }
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        self.event = event::read()?;
        match self.event {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match self.url.input_mode {
            InputMode::Normal => {
                let key_code = key_event.code;
                match key_code {
                    KeyCode::Char('c') => {
                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            self.exit();
                        }
                    }
                    KeyCode::Char('h') => {
                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            self.help_triggered = !self.help_triggered;
                        }
                    }
                    KeyCode::Char('g') => {
                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            self.url.input_mode = InputMode::Editing;
                        }
                    }
                    _ => {}
                }
            }
            InputMode::Editing => match key_event.code {
                KeyCode::Enter => self.make_request(),
                KeyCode::Esc => self.stop_editing(),
                _ => {
                    self.url.input.handle_event(&self.event);
                }
            },
        }
    }

    fn stop_editing(&mut self) {
        self.url.input_mode = InputMode::Normal;
    }

    fn make_request(&mut self) {
        let url = self.url.input.value_and_reset();
        self.stop_editing();
        //TODO: remove this unwrap and report the error to the user
        let res = gemini::make_request(&self.ssl_connection, &url).unwrap();
        let mut parser = gemini::Parser::new(&res);
        parser.parse_gemtext();
        self.page_content = parser.get_lines();
        self.current_page = Page::Browse;
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn calculate_url_input_size(&mut self, area: Rect) {
        let width = (area.width as f32 * 0.80) as u16;
        let x = (area.width - width) / 2;
        self.url.input_area = Rect {
            y: 1,
            x,
            width,
            height: 3,
        };
    }

    fn render_title_page(&self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, Style::default().bg(BG_COLOR));
        let mut lines = vec![];
        for line in aries_logo::LOGO.lines() {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::Red),
            )));
        }
        let logo_height = lines.len() as u16;
        let logo = Text::from(lines);
        let vertical_center = Rect {
            y: area.y + (area.height / 2).saturating_sub(logo_height / 2),
            height: logo_height,
            ..area
        };
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(137, 180, 130)))
            .style(Style::default().bg(BG_COLOR))
            .render(area, buf);
        Paragraph::new(logo).centered().render(vertical_center, buf);
        let help = Line::from(Span::styled(
            "Press <Ctrl-h> to view the help",
            Style::default().fg(Color::Rgb(197, 197, 197)),
        ));
        let vertical_bottom = Rect {
            y: area.y + area.height - 2,
            x: 1,
            height: 1,
            ..area
        };
        Paragraph::new(help).render(vertical_bottom, buf);
    }

    fn style_heading(&self, heading: &gemini::Heading) -> Line<'_> {
        match heading.level {
            1 => Line::from(Span::styled(
                heading.title.clone(),
                Style::default().fg(HEADING1_COLOR),
            )),
            2 => Line::from(Span::styled(
                heading.title.clone(),
                Style::default().fg(HEADING2_COLOR),
            )),
            3 => Line::from(Span::styled(
                heading.title.clone(),
                Style::default().fg(HEADING3_COLOR),
            )),
            _ => unreachable!(),
        }
    }

    fn style_link(&self, link: String) -> Line<'_> {
        Line::from(Span::styled(link, Style::default().fg(LINK_COLOR)))
    }

    fn style_quote(&self, quote: String) -> Line<'_> {
        Line::from(Span::styled(quote, Style::default().fg(QUOTE_COLOR)))
    }

    fn style_text(&self, text: String) -> Line<'_> {
        Line::from(Span::styled(text, Style::default().fg(FG_COLOR)))
    }

    fn style_list(&self, list: String) -> Line<'_> {
        Line::from(Span::styled(list, Style::default().fg(LIST_COLOR)))
    }

    fn style_line_types(&self) -> Vec<Line<'_>> {
        use gemini::LineType::*;
        let mut lines = vec![];
        for line in self.page_content.iter() {
            match line {
                Heading(heading) => lines.push(self.style_heading(heading)),
                Link(link) => lines.push(self.style_link(link.to_string())),
                List(list) => lines.push(self.style_list(list.to_string())),
                Quote(quote) => lines.push(self.style_quote(quote.to_string())),
                Preformat(preformated) => {
                    for formated in preformated {
                        lines.push(Line::from(formated.to_string()))
                    }
                }
                Text(text) => lines.push(self.style_text(text.to_string())),
            }
        }
        lines
    }

    fn render_browse_page(&self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, Style::default().bg(BG_COLOR));
        let lines = self.style_line_types();
        let text_height = lines.len() as u16;
        let text = Text::from(lines);
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(137, 180, 130)))
            .style(Style::default().bg(BG_COLOR))
            .render(area, buf);
        let area = Rect {
            x: 1,
            y: 1,
            width: area.width - 3,
            ..area
        };
        ////TODO: add scrolling using vim motions
        Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .render(area, buf);
    }

    fn render_help_page(&self, area: Rect, buf: &mut Buffer) {
        let area = centered_rect(60, 30, area);
        Clear.render(area, buf);
        let block = Block::default()
            .title("Help".bold().into_centered_line())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(137, 180, 130)))
            .style(Style::default().bg(BG_COLOR));
        let lines = vec![
            Line::from("<Ctrl-h> - Toggle the help menu"),
            Line::from("<Ctrl-g> - Start editing the url"),
            Line::from("<Esc>    - Stop editing the url"),
            Line::from("<Ctrl-c> - Quit the app"),
        ];
        let text = Text::from(lines);
        Paragraph::new(text)
            .style(Style::default().bg(BG_COLOR).fg(FG_COLOR))
            .block(block)
            .render(area, buf);
    }

    fn render_input(&mut self, buf: &mut Buffer) {
        Clear.render(self.url.input_area, buf);
        let width = self.url.input_area.width.max(3) - 3;
        let scroll = self.url.input.visual_scroll(width as usize);
        let style = match self.url.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => FG_COLOR.into(),
        };
        let block = Block::default()
            .title("url".bold().into_centered_line())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(125, 174, 163)))
            .style(Style::default().bg(BG_COLOR));
        let input = Paragraph::new(self.url.input.value())
            .style(style)
            .scroll((0, scroll as u16))
            .block(block);
        input.render(self.url.input_area, buf);
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.current_page {
            Page::Title => self.render_title_page(area, buf),
            Page::Browse => self.render_browse_page(area, buf),
        }
        if self.help_triggered {
            self.render_help_page(area, buf);
        }
        if self.url.input_mode == InputMode::Editing {
            self.render_input(buf);
        }
    }
}
