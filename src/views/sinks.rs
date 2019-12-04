use termion::event::Key;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Widget};
use tui::Terminal;

use pulse::context::Context;
use std::sync::atomic;
use std::sync::{Arc, Mutex};

use pulse::def::SinkState;

use crate::App;

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
        let volume_ratio = vol.0 as f64 / pulse::volume::VOLUME_NORM.0 as f64;
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
}

pub fn handle_key_event(key: Key, app: &mut App, context: &Context) {

    if let Some(sink) = app.sink_list.get_selected() {
        match key {
            Key::Char('j') => {
                app.sink_list.select_next();
            }
            Key::Char('k') => {
                app.sink_list.select_prev();
            }
            Key::Char('m') => {
                context.introspect().set_sink_mute_by_index(sink.index, !sink.mute, None);
            }
            Key::Char('h') => {
                let mut new_vol = sink.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_sink_volume_by_index(sink.index, &new_vol, None);
            }
            Key::Char('l') => {
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
                new_vol.mute(new_vol.len() as u32);
                context.introspect().set_sink_volume_by_index(sink.index, &new_vol, None);
            }
            Key::Ctrl('l') => {
                let mut new_vol = sink.volume.clone();
                new_vol.reset(new_vol.len() as u32);
                context.introspect().set_sink_volume_by_index(sink.index, &new_vol, None);
            }
            _ => {}
        }
    }
}
