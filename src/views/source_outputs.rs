use termion::event::Key;
use ratatui::backend::TermionBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect, Margin};
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, Borders, Gauge, Widget, Paragraph};
use ratatui::text::Text;
use ratatui::Terminal;

use pulse::context::Context;
use std::sync::atomic;
use std::sync::{Arc, Mutex};

use crate::App;

#[derive(Default)]
pub struct ViewData {
    source_popup_open: bool,
    keybinding_popup_open: bool,
    source_index_selected: u32,
}

impl ViewData {
    pub fn open_source_popup(&mut self, entry: &crate::SourceOutputEntry) {
        self.source_popup_open = true;
        self.source_index_selected = entry.source_index;
    }

    pub fn close_source_popup(&mut self) {
        self.source_popup_open = false;
    }

    pub fn open_keybinding_popup(&mut self) {
        self.keybinding_popup_open = true;
    }

    pub fn close_keybinding_popup(&mut self) {
        self.keybinding_popup_open = false;
    }
}

pub fn entered(app: &mut App) {
    app.source_output_view_data.close_source_popup();
    app.source_output_view_data.close_keybinding_popup();
}

pub fn draw(frame: &mut ratatui::terminal::Frame, rect: Rect, app: &mut App) {

    let mut constraints = vec![Constraint::Length(3); app.source_output_list. filtered_len(
        |x| !((x.source_index == 0xffffffff || app.source_list.get(x.source_index).map(|x| x.is_monitor()).unwrap_or(false)) && app.hide_monitors)
    )];
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(rect);

    for (i, stream) in app.source_output_list.filtered_values(
        |x| !((x.source_index == 0xffffffff || app.source_list.get(x.source_index).map(|x| x.is_monitor()).unwrap_or(false)) && app.hide_monitors)
    ).enumerate() {
        let vol = stream.volume.avg();
        let volume_ratio = vol.0 as f64 / pulse::volume::Volume::NORMAL.0 as f64;
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
            .block(Block::bordered().title(name))
            .gauge_style(Style::default().fg(color))
            .ratio(volume_ratio.min(1.0))
            .label(&label)
            .render(chunks[i], frame.buffer_mut());
        }

    if app.source_output_view_data.source_popup_open {
        draw_source_popup(frame, rect, app);
    }

    if app.source_output_view_data.keybinding_popup_open {
        draw_keybinding_popup(frame, rect, app);
    }
}

pub fn draw_source_popup(frame: &mut ratatui::terminal::Frame, rect: Rect, app: &mut App) {

    let focused_stream = match app.source_output_list.get_selected() {
        None => { app.source_output_view_data.close_source_popup(); return; },
        Some(x) => x,
    };

    let rect = rect.inner(&Margin::new(4, 4));
    crate::draw::ClearingWidget::default()
        .render(rect, frame.buffer_mut());

    let block = Block::bordered().title(" Change Source ");
    let inner = block.inner(rect); // save inner rectangle size for list, as block.render consumes
                                   // the block
    block.render(rect, frame.buffer_mut());

    let list = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); app.source_list.filtered_len(|source| !source.is_monitor())])
        .split(inner);

    for (j, source) in app.source_list.filtered_values(|source| !source.is_monitor()).enumerate() {
        let mut style = Style::default();
        if app.source_output_view_data.source_index_selected == source.index {
            style = Style::default().fg(Color::Red)
        }
        if focused_stream.source_index == source.index {
            style = Style::default().fg(Color::Green)
        }
        Paragraph::new(Text::raw(format!(" {} ", source.display_name())))
            .style(style)
            .render(list[j], frame.buffer_mut());
        }
}

pub fn draw_keybinding_popup(frame: &mut ratatui::terminal::Frame, rect: Rect, app: &mut App) {

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
        ( "i  return", "Choose source for selected stream"),
        ( "K", "Kill stream"),
        ( "ctrl-k", "Kill all non-running streams"),
    ];

    let rect = rect.inner(&Margin::new(4, 4));
    crate::draw::ClearingWidget::default()
        .render(rect, frame.buffer_mut());

    let block = Block::bordered().title(" Keybindings ");
    let inner = block.inner(rect); // save inner rectangle size for list, as block.render consumes
                                   // the block
    block.render(rect, frame.buffer_mut());

    let list = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Length(1); keys.len()])
        .split(inner);

    for (j, (key, desc)) in keys.iter().enumerate() {
        Paragraph::new(Text::raw(format!(" {:^17} {}", key, desc)))
                .render(list[j], frame.buffer_mut());
    }
}

pub fn handle_key_event(key: Key, app: &mut App, context: &Context) {

    if app.source_output_view_data.keybinding_popup_open {
        handle_key_event_keybinding_popup(key, app, context);
    } else if app.source_output_view_data.source_popup_open {
        handle_key_event_source_popup(key, app, context);
    } else {
        handle_key_event_main(key, app, context);
    }
}

pub fn handle_key_event_main(key: Key, app: &mut App, context: &Context) {

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
        Key::Char('?') => {
            app.source_output_view_data.open_keybinding_popup();
            app.redraw = true;
            return;
        }
        _ => {}
    }

    if let Some(stream) = app.source_output_list.get_selected() {
        match key {
            Key::Char('j') | Key::Down => {
                if app.hide_monitors {
                    app.source_output_list.filtered_select_next(filter);
                } else {
                    app.source_output_list.select_next();
                }
            }
            Key::Char('k') | Key::Up => {
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
            Key::Char('h') | Key::Left => {
                if app.hide_monitors && !filter(stream) { return; }
                let mut new_vol = stream.volume.clone();
                new_vol.decrease(pulse::volume::Volume{0: crate::VOLUME_STEP_SMALL});
                context.introspect().set_source_output_volume(stream.index, &new_vol, None);
            }
            Key::Char('l') | Key::Right => {
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
                new_vol.mute(new_vol.len());
                context.introspect().set_source_output_volume(stream.index, &new_vol, None);
            }
            Key::Ctrl('l') => {
                if app.hide_monitors && !filter(stream) { return; }
                let mut new_vol = stream.volume.clone();
                new_vol.reset(new_vol.len());
                context.introspect().set_source_output_volume(stream.index, &new_vol, None);
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
                context.introspect().set_source_output_volume(stream.index, &new_vol, None);
            }
            Key::Char('\n') |
            Key::Char('i') => {
                app.source_output_view_data.open_source_popup(stream);
                app.redraw = true;
            }
            _ => {}
        }
    }
}

pub fn handle_key_event_source_popup(key: Key, app: &mut App, context: &Context) {

    let stream = match app.source_output_list.get_selected() {
        Some(stream) => stream,
        None => {
            app.source_output_view_data.close_source_popup();
            return;
        }
    };

    match key {
        Key::Esc => {
            app.source_output_view_data.close_source_popup();
            app.redraw = true;
        }
        Key::Char('\n') => {
            context.introspect().move_source_output_by_index(stream.index, app.source_output_view_data.source_index_selected, None);
            app.source_output_view_data.close_source_popup();
            app.redraw = true;
        }
        Key::Char('j') | Key::Down => {
            if let Some(k) = app.source_list.filtered_next_key(
                    app.source_output_view_data.source_index_selected,
                    |source| !source.is_monitor()) {
                app.source_output_view_data.source_index_selected = k;
                app.redraw = true;
            }
        }
        Key::Char('k') | Key::Up => {
            if let Some(k) = app.source_list.filtered_prev_key(
                    app.source_output_view_data.source_index_selected,
                    |source| !source.is_monitor()) {
                app.source_output_view_data.source_index_selected = k;
                app.redraw = true;
            }
        }
        _ => {}
    }
}

pub fn handle_key_event_keybinding_popup(key: Key, app: &mut App, context: &Context) {
    match key {
        Key::Esc => {
            app.source_output_view_data.close_keybinding_popup();
            app.redraw = true;
        }
        _ => {}
    }
}
