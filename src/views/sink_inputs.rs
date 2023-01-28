use termion::event::Key;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Widget, Paragraph, Text};
use tui::Terminal;

use pulse::context::Context;
use std::sync::atomic;
use std::sync::{Arc, Mutex};

use crate::App;

#[derive(Default)]
pub struct ViewData {
    sink_popup_open: bool,
    keybinding_popup_open: bool,
    sink_index_selected: u32,
}

impl ViewData {
    pub fn open_sink_popup(&mut self, entry: &crate::SinkInputEntry) {
        self.sink_popup_open = true;
        self.sink_index_selected = entry.sink_index;
    }

    pub fn close_sink_popup(&mut self) {
        self.sink_popup_open = false;
    }

    pub fn open_keybinding_popup(&mut self) {
        self.keybinding_popup_open = true;
    }

    pub fn close_keybinding_popup(&mut self) {
        self.keybinding_popup_open = false;
    }
}

pub fn entered(app: &mut App) {
    app.sink_input_view_data.close_sink_popup();
    app.sink_input_view_data.close_keybinding_popup();
}

pub fn draw<T: tui::backend::Backend>(frame: &mut tui::terminal::Frame<T>, rect: Rect, app: &mut App) {

    let mut constraints = vec![Constraint::Length(3); app.sink_input_list.len()];
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(rect);

    for (i, stream) in app.sink_input_list.values().enumerate() {
        let vol = stream.volume.avg();
        let volume_ratio = vol.0 as f64 / pulse::volume::Volume::NORMAL.0 as f64;
        let mut label = format!("{:.0}%", volume_ratio * 100f64);
        if stream.mute {
            label += " (muted)";
        }

        let display_name = stream.display_name();
        let sink_name = app.sink_list.get(stream.sink_index).map(|s| s.display_name()).unwrap_or(String::from("?"));
        let name = format!(" {}  ->  {} ", display_name, sink_name);

        let invalid = stream.mute || !stream.has_volume || stream.corked;

        let color = if stream.index == app.sink_input_list.get_selected().expect("No selected entry while drawing").index {
            if invalid { Color::Gray } else { Color::Green }
        } else if invalid {
            Color::DarkGray
        } else {
            Color::Yellow
        };

        Gauge::default()
            .block(Block::default().title(&name).borders(Borders::ALL))
            .style(Style::default().fg(color))
            .ratio(volume_ratio.min(1.0))
            .label(&label)
            .render(frame, chunks[i]);
    }

    if app.sink_input_view_data.sink_popup_open {
        draw_sink_popup(frame, rect, app);
    }

    if app.sink_input_view_data.keybinding_popup_open {
        draw_keybinding_popup(frame, rect, app);
    }
}

pub fn draw_sink_popup<T: tui::backend::Backend>(frame: &mut tui::terminal::Frame<T>, rect: Rect, app: &mut App) {

    let focused_stream = match app.sink_input_list.get_selected() {
        None => { app.sink_input_view_data.close_sink_popup(); return; },
        Some(x) => x,
    };

    let rect = rect.inner(4);
    crate::draw::ClearingWidget::default()
        .render(frame, rect);

    let mut block = Block::default().title(" Change Sink ").borders(Borders::ALL);
    block.render(frame, rect);

    let list = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); app.sink_list.len()])
        .split(block.inner(rect));

    for (j, sink) in app.sink_list.values().enumerate() {
        let mut style = Style::default();
        if app.sink_input_view_data.sink_index_selected == sink.index {
            style = Style::default().fg(Color::Red)
        }
        if focused_stream.sink_index == sink.index {
            style = Style::default().fg(Color::Green)
        }
        Paragraph::new([Text::raw(format!(" {} ", sink.display_name()))].iter())
            .style(style)
            .render(frame, list[j]);
        }
}

pub fn draw_keybinding_popup<T: tui::backend::Backend>(frame: &mut tui::terminal::Frame<T>, rect: Rect, app: &mut App) {

    let keys = vec![
        ( "F1 through F5", "Change tab"),
        ( "?", "Hotkeys"),
        ( "Esc", "Close popup"),
        ( "j/down  k/up", "Movement"),
        ( "^  1 through 0", "Audio level shortcut"),
        ( "m", "Toggle mute"),
        ( "h  l", "Volume down / up"),
        ( "H  L", "Volume down / up (10% steps)"),
        ( "ctrl-H  ctrl-L", "Volume 0% / 100%"),
        ( "i  return", "Choose sink for selected stream"),
        ( "K", "Kill stream"),
        ( "ctrl-k", "Kill all non-running streams"),
    ];

    let rect = rect.inner(4);
    crate::draw::ClearingWidget::default()
        .render(frame, rect);

    let mut block = Block::default().title(" Keybindings ").borders(Borders::ALL);
    block.render(frame, rect);

    let list = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); keys.len()])
        .split(block.inner(rect));

    for (j, (key, desc)) in keys.iter().enumerate() {
        Paragraph::new([Text::raw(format!(" {:^17} {}", key, desc))].iter())
                .render(frame, list[j]);
    }
}

pub fn handle_key_event(key: Key, app: &mut App, context: &Context) {

    if app.sink_input_view_data.keybinding_popup_open {
        handle_key_event_keybinding_popup(key, app, context);
    } else if app.sink_input_view_data.sink_popup_open {
        handle_key_event_sink_popup(key, app, context);
    } else {
        handle_key_event_main(key, app, context);
    }
}

pub fn handle_key_event_main(key: Key, app: &mut App, context: &Context) {

    match key {
        Key::Ctrl('k') => {
            for stream in app.sink_input_list.values() {
                if stream.corked {
                    context.introspect().kill_sink_input(stream.index, |_| {});
                }
            }
            return;
        }
        Key::Char('?') => {
            app.sink_input_view_data.open_keybinding_popup();
            app.redraw = true;
            return;
        }
        _ => {}
    }

    if let Some(stream) = app.sink_input_list.get_selected() {
        match key {
            Key::Char('j') | Key::Down => {
                app.sink_input_list.select_next();
            }
            Key::Char('k') | Key::Up => {
                app.sink_input_list.select_prev();
            }
            Key::Char('m') => {
                context.introspect().set_sink_input_mute(stream.index, !stream.mute, None);
            }
            Key::Char('K') => {
                context.introspect().kill_sink_input(stream.index, |_| {});
            }
            Key::Char('h') | Key::Left => {
                let mut new_vol = stream.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_sink_input_volume(stream.index, &new_vol, None);
            }
            Key::Char('l') | Key::Right => {
                let mut new_vol = stream.volume.clone();
                new_vol.increase(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_sink_input_volume(stream.index, &new_vol, None);
            }
            Key::Char('H') => {
                let mut new_vol = stream.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_BIG});
                context.introspect().set_sink_input_volume(stream.index, &new_vol, None);
            }
            Key::Char('L') => {
                let mut new_vol = stream.volume.clone();
                new_vol.increase(pulse::volume::Volume{0: crate::VOLUME_STEP_BIG});
                context.introspect().set_sink_input_volume(stream.index, &new_vol, None);
            }
            Key::Ctrl('h') => {
                let mut new_vol = stream.volume.clone();
                new_vol.mute(new_vol.len());
                context.introspect().set_sink_input_volume(stream.index, &new_vol, None);
            }
            Key::Ctrl('l') => {
                let mut new_vol = stream.volume.clone();
                new_vol.reset(new_vol.len());
                context.introspect().set_sink_input_volume(stream.index, &new_vol, None);
            }
            Key::Char('^')
                | Key::Char('1')
                | Key::Char('2')
                | Key::Char('3')
                | Key::Char('4')
                | Key::Char('5')
                | Key::Char('6')
                | Key::Char('7')
                | Key::Char('8')
                | Key::Char('9')
                | Key::Char('0') => {

                let factor = match key {
                    Key::Char('^') => 0,
                    Key::Char('1') => 1,
                    Key::Char('2') => 2,
                    Key::Char('3') => 3,
                    Key::Char('4') => 4,
                    Key::Char('5') => 5,
                    Key::Char('6') => 6,
                    Key::Char('7') => 7,
                    Key::Char('8') => 8,
                    Key::Char('9') => 9,
                    Key::Char('0') => 10,
                    _ => 0,
                };
                let mut new_vol = stream.volume.clone();
                new_vol.set(new_vol.len(), pulse::volume::Volume{0: pulse::volume::Volume::NORMAL.0 / 10 * factor});
                context.introspect().set_sink_input_volume(stream.index, &new_vol, None);
            }
            Key::Char('\n') |
            Key::Char('i') => {
                app.sink_input_view_data.open_sink_popup(stream);
                app.redraw = true;
            }
            _ => {}
        }
    }
}

pub fn handle_key_event_sink_popup(key: Key, app: &mut App, context: &Context) {

    let stream = match app.sink_input_list.get_selected() {
        Some(stream) => stream,
        None => {
            app.sink_input_view_data.close_sink_popup();
            return;
        }
    };

    match key {
        Key::Esc => {
            app.sink_input_view_data.close_sink_popup();
            app.redraw = true;
        }
        Key::Char('\n') => {
            context.introspect().move_sink_input_by_index(stream.index, app.sink_input_view_data.sink_index_selected, None);
            app.sink_input_view_data.close_sink_popup();
            app.redraw = true;
        }
        Key::Char('j') | Key::Down => {
            if let Some(k) = app.sink_list.next_key(app.sink_input_view_data.sink_index_selected) {
                app.sink_input_view_data.sink_index_selected = k;
                app.redraw = true;
            }
        }
        Key::Char('k') | Key::Up => {
            if let Some(k) = app.sink_list.prev_key(app.sink_input_view_data.sink_index_selected) {
                app.sink_input_view_data.sink_index_selected = k;
                app.redraw = true;
            }
        }
        _ => {}
    }
}

pub fn handle_key_event_keybinding_popup(key: Key, app: &mut App, context: &Context) {
    match key {
        Key::Esc => {
            app.sink_input_view_data.close_keybinding_popup();
            app.redraw = true;
        }
        _ => {}
    }
}
