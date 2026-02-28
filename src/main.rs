mod aries_logo;
mod gemini;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Widget},
};
use std::io;

#[derive(Debug, Default)]
enum ScreenKind {
    #[default]
    Title,
    Browse,
    Search,
}

#[derive(Debug, Default)]
pub struct App {
    current_screen: ScreenKind,
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
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn render_title_page(&self, area: Rect, buf: &mut Buffer) {
        let bg_color = Color::Rgb(10, 10, 16);
        let fg_color = Color::Rgb(216, 166, 87);
        buf.set_style(area, Style::default().bg(bg_color));
        let mut lines = vec![];
        for line in aries_logo::LOGO.lines() {
            lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(fg_color),
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
            "Press h to view the help",
            Style::default().fg(fg_color),
        ));
        let vertical_bottom = Rect {
            y: area.y + area.height - 1,
            height: 1,
            ..area
        };
        Paragraph::new(help).render(vertical_bottom, buf);
    }
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        match self.current_screen {
            ScreenKind::Title => self.render_title_page(area, buf),
            _ => {},
        }
    }
}

fn main() -> io::Result<()> {
    ratatui::run(|terminal| App::default().run(terminal))
}
