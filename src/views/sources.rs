use termion::event::Key;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Widget};
use tui::Terminal;

use pulse::context::Context;
use std::sync::atomic;
use std::sync::{Arc, Mutex};

use pulse::def::SourceState;

use crate::App;

#[derive(Default)]
pub struct ViewData {
}

pub fn entered(app: &mut App) {
}

pub fn draw<T: tui::backend::Backend>(frame: &mut tui::terminal::Frame<T>, rect: Rect, app: &mut App) {

    let mut constraints = vec![Constraint::Length(3); app.source_list.filtered_len(|x| !(x.is_monitor() && app.hide_monitors))];
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(rect);

    for (i, source) in app.source_list.filtered_values(|x| !(x.is_monitor() && app.hide_monitors)).enumerate() {
        let vol = source.volume.avg();
        let volume_ratio = vol.0 as f64 / pulse::volume::Volume::NORMAL.0 as f64;
        let mut label = format!("{:.0}%", volume_ratio * 100f64);
        if source.mute {
            label += " (muted)";
        }

        let title = format!(" {} ", source.display_name());

        let invalid = source.mute || source.state == SourceState::Suspended;

        let color = if source.index == app.source_list.get_selected().expect("No selected entry while drawing").index {
            if invalid { Color::Gray } else { Color::Green }
        } else if invalid {
            Color::DarkGray
        } else if source.state == SourceState::Idle {
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
}

pub fn handle_key_event(key: Key, app: &mut App, context: &Context) {

    if let Some(source) = app.source_list.get_selected() {
        match key {
            Key::Char('j') => {
                if app.hide_monitors {
                    app.source_list.filtered_select_next(|x| !x.is_monitor());
                } else {
                    app.source_list.select_next();
                }
            }
            Key::Char('k') => {
                if app.hide_monitors {
                    app.source_list.filtered_select_prev(|x| !x.is_monitor());
                } else {
                    app.source_list.select_prev();
                }
            }
            Key::Char('m') => {
                if app.hide_monitors && source.is_monitor() { return; }
                context.introspect().set_source_mute_by_index(source.index, !source.mute, None);
            }
            Key::Char('h') => {
                if app.hide_monitors && source.is_monitor() { return; }
                let mut new_vol = source.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_source_volume_by_index(source.index, &new_vol, None);
            }
            Key::Char('l') => {
                if app.hide_monitors && source.is_monitor() { return; }
                let mut new_vol = source.volume.clone();
                new_vol.increase(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_source_volume_by_index(source.index, &new_vol, None);
            }
            Key::Char('H') => {
                if app.hide_monitors && source.is_monitor() { return; }
                let mut new_vol = source.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_BIG});
                context.introspect().set_source_volume_by_index(source.index, &new_vol, None);
            }
            Key::Char('L') => {
                if app.hide_monitors && source.is_monitor() { return; }
                let mut new_vol = source.volume.clone();
                new_vol.increase(pulse::volume::Volume{0: crate::VOLUME_STEP_BIG});
                context.introspect().set_source_volume_by_index(source.index, &new_vol, None);
            }
            Key::Ctrl('h') => {
                if app.hide_monitors && source.is_monitor() { return; }
                let mut new_vol = source.volume.clone();
                new_vol.mute(new_vol.len());
                context.introspect().set_source_volume_by_index(source.index, &new_vol, None);
            }
            Key::Ctrl('l') => {
                if app.hide_monitors && source.is_monitor() { return; }
                let mut new_vol = source.volume.clone();
                new_vol.reset(new_vol.len());
                context.introspect().set_source_volume_by_index(source.index, &new_vol, None);
            }
            _ => {}
        }
    }
}
