use termion::event::Key;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Widget};
use tui::Terminal;

use pulse::context::Context;
use std::sync::atomic;
use std::sync::{Arc, Mutex};

use crate::App;

pub fn draw<T: tui::backend::Backend>(frame: &mut tui::terminal::Frame<T>, rect: Rect, app: &mut App) {

    let mut constraints = vec![Constraint::Length(3); app.sink_input_list.len()];
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(rect);

    for (i, stream) in app.sink_input_list.values().enumerate() {
        let vol = stream.volume.avg();
        let volume_ratio = vol.0 as f64 / pulse::volume::VOLUME_NORM.0 as f64;
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
}

pub fn handle_key_event(key: Key, app: &mut App, context: &Context) {

    match key {
        Key::Ctrl('k') => {
            for stream in app.sink_input_list.values() {
                if stream.corked {
                    context.introspect().kill_sink_input(stream.index, |_| {});
                }
            }
            return;
        }
        _ => {}
    }

    if let Some(stream) = app.sink_input_list.get_selected() {
        match key {
            Key::Char('j') => {
                app.sink_input_list.select_next();
            }
            Key::Char('k') => {
                app.sink_input_list.select_prev();
            }
            Key::Char('m') => {
                context.introspect().set_sink_input_mute(stream.index, !stream.mute, None);
            }
            Key::Char('K') => {
                context.introspect().kill_sink_input(stream.index, |_| {});
            }
            Key::Char('h') => {
                let mut new_vol = stream.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_sink_input_volume(stream.index, &new_vol, None);
            }
            Key::Char('l') => {
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
                new_vol.mute(new_vol.len() as u32);
                context.introspect().set_sink_input_volume(stream.index, &new_vol, None);
            }
            Key::Ctrl('l') => {
                let mut new_vol = stream.volume.clone();
                new_vol.reset(new_vol.len() as u32);
                context.introspect().set_sink_input_volume(stream.index, &new_vol, None);
            }
            _ => {}
        }
    }
}
