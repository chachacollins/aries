use crate::aries_art;
use crate::gemini;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use openssl::ssl::SslConnector;
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect, Size},
    prelude::StatefulWidget,
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use std::collections::HashMap;
use std::io;
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;
use tui_scrollview::{ScrollView, ScrollViewState, ScrollbarVisibility};

#[derive(Debug, PartialEq)]
enum Page {
    Title,
    Browse,
    Error(gemini::ReqErr),
}

const BG_COLOR: Color = Color::Rgb(10, 10, 16);
const FG_COLOR: Color = Color::Rgb(235, 219, 178);
const HEADING1_COLOR: Color = Color::Rgb(234, 105, 98);
const HEADING2_COLOR: Color = Color::Rgb(216, 166, 87);
const HEADING3_COLOR: Color = Color::Rgb(125, 174, 163);
const QUOTE_COLOR: Color = Color::Rgb(137, 180, 130);
const LINK_COLOR: Color = Color::Rgb(137, 180, 130);
const LIST_COLOR: Color = Color::Rgb(130, 131, 116);

#[derive(Debug, PartialEq, Default)]
enum InputMode {
    #[default]
    Normal,
    Editing,
    Follow,
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
    page_links: HashMap<String, gemini::Link>,
    page_url: String,
    f_chars: String,
    scroll_state: ScrollViewState,
}

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

fn sanitize_url(url: String) -> String {
    let mut url = url;
    if !url.starts_with("gemini://") {
        url.insert_str(0, "gemini://");
    }
    url.push('/');
    url
}

fn calculate_wrapped_height(lines: &[Line], width: u16) -> u16 {
    let mut total_height = 0u16;
    for line in lines {
        let line_width: usize = line
            .spans
            .iter()
            .map(|span| span.content.chars().count())
            .sum();
        if line_width == 0 {
            total_height += 1;
        } else {
            let rows = (line_width as u16).div_ceil(width);
            total_height += rows;
        }
    }
    total_height
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
            page_url: String::new(),
            page_content: Vec::new(),
            page_links: HashMap::new(),
            f_chars: String::new(),
            scroll_state: ScrollViewState::default(),
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
                    KeyCode::Char('f') => self.url.input_mode = InputMode::Follow,
                    KeyCode::Char('j') => self.scroll_state.scroll_down(),
                    KeyCode::Char('k') => self.scroll_state.scroll_up(),
                    KeyCode::Char('G') => self.scroll_state.scroll_to_bottom(),
                    KeyCode::Char('h') => {
                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            self.help_triggered = !self.help_triggered;
                        }
                    }
                    KeyCode::Char('g') => {
                        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                            self.url.input_mode = InputMode::Editing;
                        } else {
                            self.scroll_state.scroll_to_top();
                        }
                    }
                    _ => {}
                }
            }

            InputMode::Follow => match key_event.code {
                KeyCode::Esc => self.url.input_mode = InputMode::Normal,
                KeyCode::Char(a) => {
                    self.f_chars.push(a);
                    if self.f_chars.len() == 2 {
                        if let Some(link) = self.page_links.get(&self.f_chars) {
                            if link.is_relative {
                                self.page_url = self.page_url.clone() + &link.link;
                            } else {
                                self.page_url = link.link.clone();
                            }
                            self.f_chars.clear();
                            self.make_request();
                        }
                    }
                }
                _ => {}
            },

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

    fn collect_links(&mut self) {
        use gemini::LineType;
        for line in &self.page_content {
            match line {
                LineType::Link(link) => {
                    self.page_links.insert(link.f_char.clone(), link.clone());
                }
                _ => {}
            }
        }
    }

    fn make_request(&mut self) {
        if self.url.input_mode == InputMode::Follow {
            self.page_url = sanitize_url(self.page_url.clone());
        } else {
            self.page_url = sanitize_url(self.url.input.value_and_reset());
        }
        self.stop_editing();
        self.page_links.clear();
        match gemini::make_request(&self.ssl_connection, &self.page_url) {
            Ok(res) => {
                let mut parser = gemini::Parser::new(&res);
                parser.parse_gemtext();
                self.page_content = parser.get_lines();
                self.collect_links();
                self.current_page = Page::Browse;
            }
            Err(err) => {
                self.current_page = Page::Error(err);
            }
        }
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
        for line in aries_art::LOGO.lines() {
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
        let style = Style::default().add_modifier(Modifier::BOLD);
        match heading.level {
            1 => Line::from(Span::styled(
                heading.title.clone(),
                style.fg(HEADING1_COLOR),
            )),
            2 => Line::from(Span::styled(
                heading.title.clone(),
                style.fg(HEADING2_COLOR),
            )),
            3 => Line::from(Span::styled(
                heading.title.clone(),
                style.fg(HEADING3_COLOR),
            )),
            _ => unreachable!(),
        }
    }

    fn style_link(&self, link: &gemini::Link) -> Line<'_> {
        let style = Style::default()
            .fg(LINK_COLOR)
            .add_modifier(Modifier::ITALIC | Modifier::UNDERLINED)
            .underline_color(LINK_COLOR);
        let pref = "=> ".to_string();
        let mut display_link = Vec::new();
        display_link.push(Span::styled(pref, Style::default().fg(LINK_COLOR)));
        if let Some(alt) = &link.alt {
            display_link.push(Span::styled(alt.clone(), style));
        } else {
            display_link.push(Span::styled(link.link.clone(), style));
        }
        if self.url.input_mode == InputMode::Follow {
            let f_char = " ".to_owned() + &link.f_char.clone();
            display_link.push(f_char.fg(Color::Red));
        }
        Line::from(display_link)
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
        use gemini::LineType;
        let mut lines = vec![];
        for line in self.page_content.iter() {
            match line {
                LineType::Heading(heading) => lines.push(self.style_heading(heading)),
                LineType::Link(link) => lines.push(self.style_link(link)),
                LineType::List(list) => lines.push(self.style_list(list.to_string())),
                LineType::Quote(quote) => lines.push(self.style_quote(quote.to_string())),
                LineType::Preformat(preformated) => {
                    for formated in preformated {
                        lines.push(Line::from(formated.to_string()))
                    }
                }
                LineType::Text(text) => lines.push(self.style_text(text.to_string())),
            }
        }
        lines
    }

    fn render_browse_page(&mut self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, Style::default().bg(BG_COLOR));
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(137, 180, 130)))
            .style(Style::default().bg(BG_COLOR));
        let inner_area = block.inner(area);
        block.render(area, buf);
        let lines = self.style_line_types();
        let wrapped_height = calculate_wrapped_height(&lines, inner_area.width);
        let text = Text::from(lines);
        let mut scroll_view = ScrollView::new(Size::new(inner_area.width, wrapped_height))
            .vertical_scrollbar_visibility(ScrollbarVisibility::Never)
            .horizontal_scrollbar_visibility(ScrollbarVisibility::Never);
        let paragraph = Paragraph::new(text)
            .wrap(Wrap { trim: true })
            .style(Style::default().bg(BG_COLOR));
        scroll_view.render_widget(paragraph, Rect::new(0, 0, inner_area.width, wrapped_height));
        scroll_view.render(inner_area, buf, &mut self.scroll_state);
    }

    fn render_help_page(&self, area: Rect, buf: &mut Buffer) {
        let area = centered_rect(70, 30, area);
        Clear.render(area, buf);
        let block = Block::default()
            .title("Help".bold().into_centered_line())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(137, 180, 130)))
            .style(Style::default().bg(BG_COLOR));
        let lines = vec![
            Line::from("<Ctrl-h> - Toggle the help menu"),
            Line::from("<j>      - Scroll down text"),
            Line::from("<k>      - Scroll up text"),
            Line::from("<g>      - Scroll to the top of the text"),
            Line::from("<G>      - Scroll to the bottom of the text"),
            Line::from(
                "<f>      - follow links. Just type the characters in red next to a link to follow it",
            ),
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

    fn render_error_page(&self, area: Rect, buf: &mut Buffer, err: &gemini::ReqErr) {
        buf.set_style(area, Style::default().bg(BG_COLOR));
        let mut lines = vec![];
        for line in aries_art::ERROR.lines() {
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
        use gemini::ReqErr;
        let err_str = match err {
            ReqErr::SslConnection => "Could not create an ssl connection to host".to_string(),
            ReqErr::HostConnection => {
                format!(
                    "Could not establish connection to host. Are you sure the url:{} is valid?",
                    self.page_url
                )
            }
            ReqErr::MalformedUrl => {
                panic!(
                    "This should never happen: we have received a malformed url {}",
                    self.page_url
                );
            }
            ReqErr::Read => "Could not read contents to a buffer. Buy more ram lol".to_string(),
            ReqErr::Write => {
                "Could not write to the host. Might be a connection error somewhere.".to_string()
            }
        };
        let vertical_bottom = Rect {
            y: vertical_center.y + vertical_center.height,
            x: ((area.width - err_str.len() as u16) / 2) as u16,
            height: 1,
            ..area
        };
        let err_line = Line::from(Span::styled(err_str, Style::default().fg(Color::Red)));
        Paragraph::new(err_line).render(vertical_bottom, buf);
    }

    fn render_input(&mut self, buf: &mut Buffer) {
        Clear.render(self.url.input_area, buf);
        let width = self.url.input_area.width.max(3) - 3;
        let scroll = self.url.input.visual_scroll(width as usize);
        let style = match self.url.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => FG_COLOR.into(),
            InputMode::Follow => todo!("Technically we shouldn't even render the url bar"),
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
        match &self.current_page {
            Page::Title => self.render_title_page(area, buf),
            Page::Browse => self.render_browse_page(area, buf),
            Page::Error(err) => self.render_error_page(area, buf, err),
        }
        if self.help_triggered {
            self.render_help_page(area, buf);
        }
        if self.url.input_mode == InputMode::Editing {
            self.render_input(buf);
        }
    }
}
