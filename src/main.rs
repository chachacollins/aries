mod aries_logo;
mod gemini;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Widget},
};
use std::io;

#[derive(Debug, Default, PartialEq)]
enum Page {
    #[default]
    Title,
    Browse,
    Search,
    Help,
}

const BG_COLOR: Color = Color::Rgb(10, 10, 16);
const FG_COLOR: Color = Color::Rgb(216, 166, 87);

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

#[derive(Debug, Default)]
pub struct App {
    current_page: Page,
    help_triggered: bool,
    exit: bool,
}

impl App {
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        let key_code = key_event.code;
        match key_code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('h') => {
                if key_event.modifiers.contains(KeyModifiers::CONTROL) {
                    self.help_triggered = !self.help_triggered;
                }
            }
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
    fn render_title_page(&self, area: Rect, buf: &mut Buffer) {
        buf.set_style(area, Style::default().bg(BG_COLOR));
        let mut lines = vec![];
        for line in aries_logo::LOGO.lines() {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(FG_COLOR),
            )));
        }
        let logo_height = lines.len() as u16;
        let logo = Text::from(lines);
        let vertical_center = Rect {
            y: area.y + (area.height / 2).saturating_sub(logo_height / 2),
            height: logo_height,
            ..area
        };
        Paragraph::new(logo).centered().render(vertical_center, buf);
        let help = Line::from(Span::styled(
            "Press <Ctrl-h> to view the help",
            Style::default().fg(Color::Rgb(197,197,197)),
        ));
        let vertical_bottom = Rect {
            y: area.y + area.height - 1,
            height: 1,
            ..area
        };
        Paragraph::new(help).render(vertical_bottom, buf);
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
            Line::from("<Ctrl-g> - Toggle the url bar"),
        ];
        let text = Text::from(lines);
        Paragraph::new(text).block(block).render(area, buf);
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.current_page {
            Page::Title => self.render_title_page(area, buf),
            _ => {}
        }
        if self.help_triggered {
            self.render_help_page(area, buf);
        }
    }
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}
