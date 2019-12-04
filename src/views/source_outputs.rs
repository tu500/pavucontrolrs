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

    let mut constraints = vec![Constraint::Length(3); app.source_output_list. filtered_len(
        |x| !(app.source_list.get(x.source_index).expect("SourceOutputEntry.source_index not in list").is_monitor() && app.hide_monitors)
    )];
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(rect);

    for (i, stream) in app.source_output_list.filtered_values(
        |x| !(app.source_list.get(x.source_index).expect("SourceOutputEntry.source_index not in list").is_monitor() && app.hide_monitors)
    ).enumerate() {
        let vol = stream.volume.avg();
        let volume_ratio = vol.0 as f64 / pulse::volume::VOLUME_NORM.0 as f64;
        let mut label = format!("{:.0}%", volume_ratio * 100f64);
        if stream.mute {
            label += " (muted)";
        }

        let display_name = stream.display_name();
        let source_name = app.source_list.get(stream.source_index).map(|s| s.display_name()).unwrap_or(String::from("?"));
        let name = format!(" {}  ->  {} ", display_name, source_name);

        let invalid = stream.mute || !stream.has_volume || stream.corked;

        let color = if stream.index == app.source_output_list.get_selected().expect("No selected entry while drawing").index {
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

    let source_list = &app.source_list; // XXX
    let hide_monitors = app.hide_monitors;
    let filter = |source_output: &crate::SourceOutputEntry| !(source_list.get(source_output.source_index).expect("SourceOutputEntry.source_index not in list").is_monitor() && hide_monitors);

    match key {
        Key::Ctrl('k') => {
            for stream in app.source_output_list.values() {
                if stream.corked {
                    context.introspect().kill_source_output(stream.index, |_| {});
                }
            }
            return;
        }
        _ => {}
    }

    if let Some(stream) = app.source_output_list.get_selected() {
        match key {
            Key::Char('j') => {
                if app.hide_monitors {
                    app.source_output_list.filtered_select_next(filter);
                } else {
                    app.source_output_list.select_next();
                }
            }
            Key::Char('k') => {
                if app.hide_monitors {
                    app.source_output_list.filtered_select_prev(filter);
                } else {
                    app.source_output_list.select_prev();
                }
            }
            Key::Char('m') => {
                if app.hide_monitors && !filter(stream) { return; }
                context.introspect().set_source_output_mute(stream.index, !stream.mute, None);
            }
            Key::Char('K') => {
                if app.hide_monitors && !filter(stream) { return; }
                context.introspect().kill_source_output(stream.index, |_| {});
            }
            Key::Char('h') => {
                if app.hide_monitors && !filter(stream) { return; }
                let mut new_vol = stream.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_source_output_volume(stream.index, &new_vol, None);
            }
            Key::Char('l') => {
                if app.hide_monitors && !filter(stream) { return; }
                let mut new_vol = stream.volume.clone();
                new_vol.increase(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_source_output_volume(stream.index, &new_vol, None);
            }
            Key::Char('H') => {
                if app.hide_monitors && !filter(stream) { return; }
                let mut new_vol = stream.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_BIG});
                context.introspect().set_source_output_volume(stream.index, &new_vol, None);
            }
            Key::Char('L') => {
                if app.hide_monitors && !filter(stream) { return; }
                let mut new_vol = stream.volume.clone();
                new_vol.increase(pulse::volume::Volume{0: crate::VOLUME_STEP_BIG});
                context.introspect().set_source_output_volume(stream.index, &new_vol, None);
            }
            Key::Ctrl('h') => {
                if app.hide_monitors && !filter(stream) { return; }
                let mut new_vol = stream.volume.clone();
                new_vol.mute(new_vol.len() as u32);
                context.introspect().set_source_output_volume(stream.index, &new_vol, None);
            }
            Key::Ctrl('l') => {
                if app.hide_monitors && !filter(stream) { return; }
                let mut new_vol = stream.volume.clone();
                new_vol.reset(new_vol.len() as u32);
                context.introspect().set_source_output_volume(stream.index, &new_vol, None);
            }
            _ => {}
        }
    }
}
