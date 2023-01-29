use termion::event::Key;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Widget, Paragraph, Text};
use tui::Terminal;

use pulse::context::Context;
use std::sync::atomic;
use std::sync::{Arc, Mutex};

use pulse::def::SinkState;

use crate::App;

#[derive(Default)]
pub struct ViewData {
    keybinding_popup_open: bool,
}

impl ViewData {
    pub fn open_keybinding_popup(&mut self) {
        self.keybinding_popup_open = true;
    }

    pub fn close_keybinding_popup(&mut self) {
        self.keybinding_popup_open = false;
    }
}

pub fn entered(app: &mut App) {
    app.sink_view_data.close_keybinding_popup();
}

pub fn draw<T: tui::backend::Backend>(frame: &mut tui::terminal::Frame<T>, rect: Rect, app: &mut App) {

    let mut constraints = vec![Constraint::Length(3); app.sink_list.len()];
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(rect);

    for (i, sink) in app.sink_list.values().enumerate() {
        // let volume_ratio = VolumeLinear::from(sink.volume.avg()).0;
        let vol = sink.volume.avg();
        let volume_ratio = vol.0 as f64 / pulse::volume::Volume::NORMAL.0 as f64;
        let mut label = format!("{:.0}%", volume_ratio * 100f64);
        if sink.mute {
            label += " (muted)";
        }

        let title = format!(" {} ", sink.display_name());

        let invalid = sink.mute || sink.state == SinkState::Suspended;

        let color = if sink.index == app.sink_list.get_selected().expect("No selected entry while drawing").index {
            if invalid { Color::Gray } else { Color::Green }
        } else if invalid {
            Color::DarkGray
        } else if sink.state == SinkState::Idle {
            Color::Red
        } else {
            Color::Yellow
        };

        Gauge::default()
            .block(Block::default().title(&title).borders(Borders::ALL))
            .style(Style::default().fg(color))
            .ratio(volume_ratio.min(1.0))
            .label(&label)
            .render(frame, chunks[i]);
        }

    if app.sink_view_data.keybinding_popup_open {
        draw_keybinding_popup(frame, rect, app);
    }
}

pub fn draw_keybinding_popup<T: tui::backend::Backend>(frame: &mut tui::terminal::Frame<T>, rect: Rect, app: &mut App) {

    let keys = vec![
        ( "F1 through F5", "Change tab"),
        ( "Tab", "Cycle tabs"),
        ( "q  crtl-c", "Quit"),
        ( "?", "Hotkeys"),
        ( "Esc", "Close popup"),
        ( "j/down  k/up", "Movement"),
        ( "^  1 through 0", "Audio level shortcut"),
        ( "m", "Toggle mute"),
        ( "h  l", "Volume down / up"),
        ( "H  L", "Volume down / up (10% steps)"),
        ( "ctrl-H  ctrl-L", "Volume 0% / 100%"),
        ( "D", "Unload owner module (remove sink)"),
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

    if app.sink_view_data.keybinding_popup_open {
        handle_key_event_keybinding_popup(key, app, context);
    } else {
        handle_key_event_main(key, app, context);
    }
}

pub fn handle_key_event_main(key: Key, app: &mut App, context: &Context) {

    match key {
        Key::Char('?') => {
            app.sink_view_data.open_keybinding_popup();
            app.redraw = true;
            return;
        }
        _ => {}
    }

    if let Some(sink) = app.sink_list.get_selected() {
        match key {
            Key::Char('j') | Key::Down => {
                app.sink_list.select_next();
            }
            Key::Char('k') | Key::Up => {
                app.sink_list.select_prev();
            }
            Key::Char('m') => {
                context.introspect().set_sink_mute_by_index(sink.index, !sink.mute, None);
            }
            Key::Char('h') | Key::Left => {
                let mut new_vol = sink.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_sink_volume_by_index(sink.index, &new_vol, None);
            }
            Key::Char('l') | Key::Right => {
                let mut new_vol = sink.volume.clone();
                new_vol.increase(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_sink_volume_by_index(sink.index, &new_vol, None);
            }
            Key::Char('H') => {
                let mut new_vol = sink.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_BIG});
                context.introspect().set_sink_volume_by_index(sink.index, &new_vol, None);
            }
            Key::Char('L') => {
                let mut new_vol = sink.volume.clone();
                new_vol.increase(pulse::volume::Volume{0: crate::VOLUME_STEP_BIG});
                context.introspect().set_sink_volume_by_index(sink.index, &new_vol, None);
            }
            Key::Ctrl('h') => {
                let mut new_vol = sink.volume.clone();
                new_vol.mute(new_vol.len());
                context.introspect().set_sink_volume_by_index(sink.index, &new_vol, None);
            }
            Key::Ctrl('l') => {
                let mut new_vol = sink.volume.clone();
                new_vol.reset(new_vol.len());
                context.introspect().set_sink_volume_by_index(sink.index, &new_vol, None);
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
                let mut new_vol = sink.volume.clone();
                new_vol.set(new_vol.len(), pulse::volume::Volume{0: pulse::volume::Volume::NORMAL.0 / 10 * factor});
                context.introspect().set_sink_volume_by_index(sink.index, &new_vol, None);
            }
            Key::Char('D') => {
                if let Some(owner_module_id) = sink.owner_module {
                    context.introspect().unload_module(owner_module_id, |_| {});
                }
            }
            _ => {}
        }
    }
}

pub fn handle_key_event_keybinding_popup(key: Key, app: &mut App, context: &Context) {
    match key {
        Key::Esc => {
            app.sink_view_data.close_keybinding_popup();
            app.redraw = true;
        }
        _ => {}
    }
}
