use std::io;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;
use termion::input::TermRead;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Widget};
use tui::Terminal;

use crate::{App, AppView};
use crate::views;

pub fn setup_terminal() -> Result<tui::terminal::Terminal<tui::backend::TermionBackend<termion::screen::AlternateScreen<termion::input::MouseTerminal<termion::raw::RawTerminal<std::io::Stdout>>>>>, failure::Error> {
    let stdout = std::io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    return Ok(terminal);
}

pub fn draw_frame<T: tui::backend::Backend>(terminal: &mut tui::Terminal<T>, app: &mut App) {
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

    match app.view {
        AppView::SinkInputs    => views::sink_inputs::draw(terminal, app),
        AppView::SourceOutputs => views::source_outputs::draw(terminal, app),
        AppView::Sinks         => views::sinks::draw(terminal, app),
        AppView::Sources       => views::sources::draw(terminal, app),
        AppView::Cards         => views::cards::draw(terminal, app),
    };
}
