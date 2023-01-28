use termion::event::Key;
use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, Gauge, Widget, Text, Paragraph, SelectableList};
use tui::Terminal;

use pulse::context::Context;
use std::sync::atomic;
use std::sync::{Arc, Mutex};

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
    app.card_view_data.close_keybinding_popup();
}

pub fn draw<T: tui::backend::Backend>(frame: &mut tui::terminal::Frame<T>, rect: Rect, app: &mut App) {

    let mut constraints: Vec<tui::layout::Constraint> = app.card_list.values().map(|card| Constraint::Length(2 + card.profiles.len() as u16)).collect();
    constraints.push(Constraint::Min(0));

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(rect);

    for (i, card) in app.card_list.values().enumerate() {

        let title = format!(" {} ", card.display_name());

        let title_style = if card.index == app.card_list.get_selected().expect("No selected entry while drawing").index {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let mut block = Block::default()
            .title(&title)
            .title_style(title_style)
            .borders(Borders::ALL);
        // .border_style(Style::default().fg(Color::White))
        // .style(Style::default().bg(Color::Black))
        block.render(frame, chunks[i]);

        let list = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(1); card.profiles.len()])
            // .split(chunks[i]);
            .split(block.inner(chunks[i]));

        for (j, profile) in card.profiles.iter().enumerate() {
            let mut style = Style::default();
            if let Some(selected_index) = card.selected_profile_index {
                if selected_index == j {
                    style = Style::default().fg(Color::Red)
                }
            }
            if let Some(active_index) = card.active_profile_index {
                if active_index == j {
                    style = Style::default().fg(Color::Green)
                }
            }
            Paragraph::new([Text::raw(format!(" {}", profile.display_name()))].iter())
                .style(style)
                .render(frame, list[j]);
            }

        // let profile_names: Vec<&str> = card.profiles.iter().map(|p| p.description.as_ref()).collect();
        // SelectableList::default()
        //     .items(&profile_names)
        //     .select(card.selected_profile_index)
        //     // .style(Style::default().fg(Color::Yellow))
        //     .highlight_style(Style::default().fg(Color::Green).modifier(Modifier::ITALIC))
        //     .render(&mut frame, block.inner(chunks[i]));
    }

    if app.card_view_data.keybinding_popup_open {
        draw_keybinding_popup(frame, rect, app);
    }
}

pub fn draw_keybinding_popup<T: tui::backend::Backend>(frame: &mut tui::terminal::Frame<T>, rect: Rect, app: &mut App) {

    let keys = vec![
        ( "F1 through F5", "Change tab"),
        ( "?", "Hotkeys"),
        ( "Esc", "Close popup"),
        ( "j/down  k/up", "Movement"),
        ( "+  -", "Select profile for current card"),
        ( "Return", "Confirm profile"),
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

    if app.card_view_data.keybinding_popup_open {
        handle_key_event_keybinding_popup(key, app, context);
    } else {
        handle_key_event_main(key, app, context);
    }
}

pub fn handle_key_event_main(key: Key, app: &mut App, context: &Context) {

    match key {
        Key::Char('?') => {
            app.card_view_data.open_keybinding_popup();
            app.redraw = true;
            return;
        }
        _ => {}
    }

    if let Some(card) = app.card_list.get_selected_mut() {
        match key {
            Key::Char('j') | Key::Down => {
                app.card_list.select_next();
            }
            Key::Char('k') | Key::Up => {
                app.card_list.select_prev();
            }
            Key::Char('+') => {
                if let Some(selected_profile_index) = card.selected_profile_index {
                    let new_index = selected_profile_index + 1;
                    let new_index = new_index.min(card.profiles.len() - 1);
                    card.selected_profile_index = Some(new_index);
                } else {
                    assert_eq!(card.profiles.len(), 0);
                }
            }
            Key::Char('-') => {
                if let Some(selected_profile_index) = card.selected_profile_index {
                    let new_index = if selected_profile_index == 0 { 0 } else { selected_profile_index-1 };
                    card.selected_profile_index = Some(new_index);
                } else {
                    assert_eq!(card.profiles.len(), 0);
                }
            }
            Key::Char('\n') => {
                if let Some(selected_profile_index) = card.selected_profile_index {
                    context.introspect().set_card_profile_by_index(card.index, &card.profiles[selected_profile_index].name, None);
                }
            }
            _ => {}
        }
    }
}

pub fn handle_key_event_keybinding_popup(key: Key, app: &mut App, context: &Context) {
    match key {
        Key::Esc => {
            app.card_view_data.close_keybinding_popup();
            app.redraw = true;
        }
        _ => {}
    }
}
