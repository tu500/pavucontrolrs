use std::io;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;
use termion::input::TermRead;
use ratatui::backend::TermionBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Gauge, Widget, Tabs};
use ratatui::Terminal;

use crate::{App, AppView};
use crate::views;

type FinalTerminal = ratatui::terminal::Terminal<
                     ratatui::backend::TermionBackend<
                     termion::screen::AlternateScreen<
                     termion::input::MouseTerminal<
                     termion::raw::RawTerminal<
                     std::io::Stdout
                     >>>>>;

pub fn setup_terminal() -> Result<FinalTerminal, std::io::Error> {
    let stdout = std::io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = stdout.into_alternate_screen()?;
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    return Ok(terminal);
}

pub fn draw_frame<T: ratatui::backend::Backend>(terminal: &mut ratatui::Terminal<T>, app: &mut App) {
    let mut do_redraw = match app.view {
        AppView::SinkInputs    => app.sink_input_list.reset_changed(),
        AppView::SourceOutputs => app.source_output_list.reset_changed(),
        AppView::Sinks         => app.sink_list.reset_changed(),
        AppView::Sources       => app.source_list.reset_changed(),
        AppView::Cards         => app.card_list.reset_changed(),
    };

    if app.redraw {
        do_redraw = true;
        app.redraw = false;
    }

    if !do_redraw {
        return;
    }

    let _ = terminal.draw(|f| {

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints(vec![Constraint::Length(3), Constraint::Length(1), Constraint::Min(0)])
            .split(f.size());

        Tabs::new(["Sink Inputs", "Source Output", "Sinks", "Sources", "Cards"])
            .block(Block::bordered().title(" Tabs "))
            .highlight_style(Style::default().fg(Color::Yellow))
            .divider(ratatui::symbols::DOT)
            .select(app.view as usize)
            .render(chunks[0], f.buffer_mut());

        match app.view {
            AppView::SinkInputs    => views::sink_inputs::draw(f, chunks[2], app),
            AppView::SourceOutputs => views::source_outputs::draw(f, chunks[2], app),
            AppView::Sinks         => views::sinks::draw(f, chunks[2], app),
            AppView::Sources       => views::sources::draw(f, chunks[2], app),
            AppView::Cards         => views::cards::draw(f, chunks[2], app),
        };
    });
}

#[derive(Default)]
pub struct ClearingWidget {
}

impl Widget for ClearingWidget {
    fn render(self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer) {
        for x in area.left()..area.right() {
            for y in area.top()..area.bottom() {
                buf.get_mut(x, y).set_symbol(" ");
            }
        }
    }
}
