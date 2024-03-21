#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
extern crate libpulse_binding as pulse;
// extern crate termion;

// mod event;

use std::io;

use std::thread;
use std::sync::atomic;
use std::rc::Rc;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use std::ops::Deref;
use pulse::mainloop::standard::Mainloop;
use pulse::context::Context;
use pulse::context::introspect;
// use pulse::stream::Stream;
use pulse::proplist::Proplist;
use pulse::mainloop::standard::IterateResult;
use pulse::callbacks::ListResult;
use pulse::def::Retval;
use pulse::error::PAErr;
use pulse::context::subscribe::{Facility, Operation, InterestMaskSet};
use pulse::volume::{VolumeLinear, ChannelVolumes};

use termion::event::Key;
use termion::input::TermRead;

//use byteorder::{NativeEndian, WriteBytesExt};

// use crate::util::event::{Event, Events};

mod selecting_map;
use selecting_map::SelectingMap;

mod draw;
use draw::{draw_frame, setup_terminal};

mod views;


pub const VOLUME_STEP_SMALL: u32 = 655;
pub const VOLUME_STEP_BIG: u32 = 6554;


pub struct SinkInputEntry {
    index:           u32,
    name:            String,
    sink_index:      u32,
    volume:          ChannelVolumes,
    mute:            bool,
    corked:          bool,
    has_volume:      bool,
    volume_writable: bool,
    proplist:        pulse::proplist::Proplist,
}

impl SinkInputEntry {
    fn display_name(&self) -> String {
        let mut name = String::from(&self.name);
        if let Some(app_name) = self.proplist.get_str("application.name") {
            name = format!("{} [{}]", name, app_name);
        }

        // if Some("firefox") == self.proplist.get_str("application.process.binary").as_ref().map(|x| &x[..]) {
        //         name = format!("{} [{}]", self.name, "firefox");
        // }
        if let Some(bin_name) = self.proplist.get_str("application.process.binary") {
            if bin_name == "firefox" {
                name = format!("{} [{}]", self.name, "firefox");
            }
        }
        return name;
    }
}

impl From<&introspect::SinkInputInfo<'_>> for SinkInputEntry {
    fn from(entry: &introspect::SinkInputInfo) -> SinkInputEntry {
        SinkInputEntry {
            index:           entry.index,
            name:            String::from(entry.name.as_ref().expect("SinkInputInfo without name").as_ref()),
            sink_index:      entry.sink,
            volume:          entry.volume,
            mute:            entry.mute,
            corked:          entry.corked,
            has_volume:      entry.has_volume,
            volume_writable: entry.volume_writable,
            proplist:        entry.proplist.clone(),
        }
    }
}

pub struct SourceOutputEntry {
    index:           u32,
    name:            String,
    source_index:    u32,
    volume:          ChannelVolumes,
    mute:            bool,
    corked:          bool,
    has_volume:      bool,
    volume_writable: bool,
    proplist:        pulse::proplist::Proplist,
}

impl SourceOutputEntry {
    fn display_name(&self) -> String {
        let mut name = String::from(&self.name);
        if let Some(app_name) = self.proplist.get_str("application.name") {
            name = format!("{} [{}]", name, app_name);
        }
        return name;
    }
}

impl From<&introspect::SourceOutputInfo<'_>> for SourceOutputEntry {
    fn from(entry: &introspect::SourceOutputInfo) -> SourceOutputEntry {
        SourceOutputEntry {
            index:           entry.index,
            name:            String::from(entry.name.as_ref().expect("SourceOutputInfo without name").as_ref()),
            source_index:    entry.source,
            volume:          entry.volume,
            mute:            entry.mute,
            corked:          entry.corked,
            has_volume:      entry.has_volume,
            volume_writable: entry.volume_writable,
            proplist:        entry.proplist.clone(),
        }
    }
}

pub struct SinkEntry {
    index:        u32,
    name:         String,
    description:  String,
    volume:       ChannelVolumes,
    mute:         bool,
    state:        pulse::def::SinkState,
    owner_module: Option<u32>,
    proplist:     pulse::proplist::Proplist,
    ports:        Vec<PortInfo>,
}

impl SinkEntry {
    fn display_name(&self) -> String {
        return String::from(&self.description);
    }
}

impl From<&introspect::SinkInfo<'_>> for SinkEntry {
    fn from(entry: &introspect::SinkInfo) -> SinkEntry {
        SinkEntry {
            index:        entry.index,
            name:         String::from(entry.name.as_ref().expect("SinkInfo without name").as_ref()),
            description:  String::from(entry.description.as_ref().expect("SinkInfo without description").as_ref()),
            volume:       entry.volume,
            mute:         entry.mute,
            state:        entry.state,
            owner_module: entry.owner_module,
            proplist:     entry.proplist.clone(),
            ports:        entry.ports.iter().map(|x| PortInfo::from(x)).collect(),
        }
    }
}

pub struct SourceEntry {
    index:           u32,
    name:            String,
    description:     String,
    volume:          ChannelVolumes,
    mute:            bool,
    monitor_of_sink: Option<u32>,
    state:           pulse::def::SourceState,
    owner_module:    Option<u32>,
    proplist:        pulse::proplist::Proplist,
    ports:           Vec<PortInfo>,
}

impl SourceEntry {
    fn display_name(&self) -> String {
        return String::from(&self.description);
    }

    fn is_monitor(&self) -> bool {
        return self.monitor_of_sink != None;
    }
}

impl From<&introspect::SourceInfo<'_>> for SourceEntry {
    fn from(entry: &introspect::SourceInfo) -> SourceEntry {
        SourceEntry {
            index:           entry.index,
            name:            String::from(entry.name.as_ref().expect("SourceInfo without name").as_ref()),
            description:     String::from(entry.description.as_ref().expect("SourceInfo without description").as_ref()),
            volume:          entry.volume,
            mute:            entry.mute,
            monitor_of_sink: entry.monitor_of_sink,
            state:           entry.state,
            owner_module:    entry.owner_module,
            proplist:        entry.proplist.clone(),
            ports:           entry.ports.iter().map(|x| PortInfo::from(x)).collect(),
        }
    }
}

pub struct CardEntry {
    index:                  u32,
    name:                   String,
    proplist:               pulse::proplist::Proplist,
    ports:                  Vec<PortInfo>,
    profiles:               Vec<ProfileInfo>,
    active_profile_index:   Option<usize>,
    selected_profile_index: Option<usize>,

    // active_profile_name: Option<String>,
}

impl CardEntry {
    fn display_name(&self) -> String {
        match self.proplist.get_str("device.description") {
            Some(desc) => return desc,
            None       => return String::from(&self.name)
        };
    }
}

impl From<&introspect::CardInfo<'_>> for CardEntry {
    fn from(entry: &introspect::CardInfo) -> CardEntry {

        let active_index = entry.active_profile.as_ref().map(|active_profile| {
            let active_name = active_profile.name.as_ref().expect("Active profile without name");
            let active_index = entry.profiles.iter().position(|p| p.name.as_ref().expect("Card profile without name") == active_name.as_ref());
            active_index.expect("Active card profile not found in profile list")
        });

        CardEntry {
            index:                  entry.index,
            name:                   String::from(entry.name.as_ref().expect("CardInfo without name").as_ref()),
            proplist:               entry.proplist.clone(),
            ports:                  entry.ports.iter().map(|x| PortInfo::from(x)).collect(),
            profiles:               entry.profiles.iter().map(|x| ProfileInfo::from(x)).collect(),
            active_profile_index:   active_index,
            selected_profile_index: active_index,
            // active_profile_name: entry.active_profile.as_ref().map(|x| String::from(x.name.as_ref().expect("Active profile without name").as_ref())),
        }
    }
}

#[derive(Clone)]
struct PortInfo {
    name:        String,
    description: String,
    priority:    u32,
    available:   pulse::def::PortAvailable,
    // additional for CardPorts: direction, proplist, latency_offset, profiles
}

impl From<&introspect::SinkPortInfo<'_>> for PortInfo {
    fn from(info: &introspect::SinkPortInfo) -> PortInfo {
        PortInfo {
            name:        String::from(info.name.as_ref().expect("SinkPortInfo without name").as_ref()),
            description: String::from(info.description.as_ref().expect("SinkPortInfo without description").as_ref()),
            priority:    info.priority,
            available:   info.available,
        }
    }
}

impl From<&introspect::SourcePortInfo<'_>> for PortInfo {
    fn from(info: &introspect::SourcePortInfo) -> PortInfo {
        PortInfo {
            name:        String::from(info.name.as_ref().expect("SourcePortInfo without name").as_ref()),
            description: String::from(info.description.as_ref().expect("SourcePortInfo without description").as_ref()),
            priority:    info.priority,
            available:   info.available,
        }
    }
}

impl From<&introspect::CardPortInfo<'_>> for PortInfo {
    fn from(info: &introspect::CardPortInfo) -> PortInfo {
        PortInfo {
            name:        String::from(info.name.as_ref().expect("CardPortInfo without name").as_ref()),
            description: String::from(info.description.as_ref().expect("CardPortInfo without description").as_ref()),
            priority:    info.priority,
            available:   info.available,
        }
    }
}

#[derive(Clone)]
struct ProfileInfo {
    name:        String,
    description: String,
    n_sinks:     u32,
    n_sources:   u32,
    priority:    u32,
    available:   bool,
}

impl ProfileInfo {
    fn display_name(&self) -> &str {
        return &self.description;
    }
}

impl From<&introspect::CardProfileInfo<'_>> for ProfileInfo {
    fn from(info: &introspect::CardProfileInfo) -> ProfileInfo {
        ProfileInfo {
            name:        String::from(info.name.as_ref().expect("CardProfileInfo2 without name").as_ref()),
            description: String::from(info.description.as_ref().expect("CardProfileInfo2 without description").as_ref()),
            priority:    info.priority,
            n_sinks:     info.n_sinks,
            n_sources:   info.n_sources,
            available:   info.available,
        }
    }
}


#[derive(Clone, Copy)]
enum AppView {
    SinkInputs,
    SourceOutputs,
    Sinks,
    Sources,
    Cards,
}

pub struct App {
    sink_input_list:         SelectingMap<u32, SinkInputEntry>,
    source_output_list:      SelectingMap<u32, SourceOutputEntry>,
    sink_list:               SelectingMap<u32, SinkEntry>,
    source_list:             SelectingMap<u32, SourceEntry>,
    card_list:               SelectingMap<u32, CardEntry>,
    sink_input_view_data:    views::sink_inputs::ViewData,
    source_output_view_data: views::source_outputs::ViewData,
    sink_view_data:          views::sinks::ViewData,
    source_view_data:        views::sources::ViewData,
    card_view_data:          views::cards::ViewData,
    redraw:                  bool,
    view:                    AppView,
    hide_monitors:           bool,
    quit_request:            bool,
}

impl App {
    fn new() -> App {
        App {
            sink_input_list:         SelectingMap::new(),
            source_output_list:      SelectingMap::new(),
            sink_list:               SelectingMap::new(),
            source_list:             SelectingMap::new(),
            card_list:               SelectingMap::new(),
            sink_input_view_data:    Default::default(),
            source_output_view_data: Default::default(),
            sink_view_data:          Default::default(),
            source_view_data:        Default::default(),
            card_view_data:          Default::default(),
            redraw:                  true,
            view:                    AppView::SinkInputs,
            hide_monitors:           true,
            quit_request:            false,
        }
    }
}


fn mainloop_iter(mainloop: &mut Mainloop, timeout: Option<pulse::time::MicroSeconds>) -> IterateResult
{
    if let Result::Err(pae) = mainloop.prepare(timeout) {
        if pae == PAErr(-2) // just a quit request
        {
            return IterateResult::Quit(Retval(0)); // no way to find out retval :(
        }
        return IterateResult::Err(pae);
    }
    if let Result::Err(pae) = mainloop.poll() {
        return IterateResult::Err(pae);
    }
    match mainloop.dispatch() {
        Result::Ok(v) => {
            return IterateResult::Success(v);
        }
        Result::Err(pae) => {
            return IterateResult::Err(pae);
        }
    }
}


fn main() {
    let app = Arc::new(Mutex::new(App::new()));
    // app.lock().unwrap().view = AppView::Cards;

    // Connect to PA
    let mut proplist = Proplist::new().expect("Proplist init failed");
    proplist.set_str(pulse::proplist::properties::APPLICATION_NAME, "Pavucontrolrs")
        .expect("Proplist setup failed");

    let mainloop = Rc::new(RefCell::new(Mainloop::new()
        .expect("Failed to create mainloop")));

    let context = Arc::new(Mutex::new(Context::new_with_proplist(
        mainloop.borrow().deref(),
        "FooAppContext",
        &proplist
        ).expect("Failed to create new context")));

    context.lock().unwrap().connect(None, pulse::context::FlagSet::NOFLAGS, None)
        .expect("Failed to connect context");

    // Wait for context to be ready
    loop {
        match mainloop.borrow_mut().iterate(false) {
            IterateResult::Quit(_) |
            IterateResult::Err(_) => {
                eprintln!("iterate state was not success, quitting...");
                return;
            },
            IterateResult::Success(_) => {},
        }
        match context.lock().unwrap().get_state() {
            pulse::context::State::Ready => { break; },
            pulse::context::State::Failed |
            pulse::context::State::Terminated => {
                eprintln!("context state failed/terminated, quitting...");
                return;
            },
            _ => {},
        }
    }


    // Retrieve initial sinkinput list
    context.lock().unwrap().introspect().get_sink_input_info_list({
        let app = Arc::clone(&app);
        move |listres| {

            match listres {
                ListResult::Item(i) => {
                    let mut app = app.lock().unwrap();
                    app.sink_input_list.update(i.index, SinkInputEntry::from(i));
                                    // println!("{}", i.proplist.to_string().unwrap());
                },
                ListResult::End |
                ListResult::Error => {},
            }

        }
    });

    // Retrieve initial sinkinput list
    context.lock().unwrap().introspect().get_source_output_info_list({
        let app = Arc::clone(&app);
        move |listres| {

            match listres {
                ListResult::Item(i) => {
                    let mut app = app.lock().unwrap();
                    app.source_output_list.update(i.index, SourceOutputEntry::from(i));
                },
                ListResult::End |
                ListResult::Error => {},
            }

        }
    });

    // Retrieve initial sink list
    context.lock().unwrap().introspect().get_sink_info_list({
        let app = Arc::clone(&app);
        move |listres| {

            match listres {
                ListResult::Item(i) => {
                    let mut app = app.lock().unwrap();
                    app.sink_list.update(i.index, SinkEntry::from(i));
                                    // println!("{}", i.proplist.to_string().unwrap());
                },
                ListResult::End |
                ListResult::Error => {},
            }

        }
    });

    // Retrieve initial source list
    context.lock().unwrap().introspect().get_source_info_list({
        let app = Arc::clone(&app);
        move |listres| {

            match listres {
                ListResult::Item(i) => {
                    let mut app = app.lock().unwrap();
                    app.source_list.update(i.index, SourceEntry::from(i));
                },
                ListResult::End |
                ListResult::Error => {},
            }

        }
    });

    // Retrieve initial card list
    context.lock().unwrap().introspect().get_card_info_list({
        let app = Arc::clone(&app);
        move |listres| {

            match listres {
                ListResult::Item(i) => {
                    let mut app = app.lock().unwrap();
                    app.card_list.update(i.index, CardEntry::from(i));
                    // let app = app.lock().unwrap();
                    // println!("CARD: ");
                    // println!("{}", i.proplist.to_string().unwrap());
                    // for p in i.profiles.iter() {
                    //     println!("{} {} {} {}", p.name.as_ref().unwrap(), p.priority, p.description.as_ref().unwrap(), p.available);
                    // }
                    // for port in i.ports.iter() {
                    //     println!("{} {} {}", port.name.as_ref().unwrap(), port.priority, port.description.as_ref().unwrap());
                    //     println!("{}", port.proplist.to_string().unwrap());
                    // }
                },
                ListResult::End |
                ListResult::Error => {},
            }

        }
    });

    // Subscribe to change events
    let interest = InterestMaskSet::SINK_INPUT |
        InterestMaskSet::SINK |
        InterestMaskSet::SOURCE_OUTPUT |
        InterestMaskSet::SOURCE |
        InterestMaskSet::CARD;
    context.lock().unwrap().subscribe(
        interest,
        |_| {}
    );

    context.lock().unwrap().set_subscribe_callback(Some(Box::new({
        let app = Arc::clone(&app);
        let context = Arc::clone(&context);
        move |facility, operation, index| {

            let app = Arc::clone(&app);
            let facility = facility.expect("Subscribed event without Facility value");
            let operation = operation.expect("Subscribed event without Operation value");
            match facility {

                Facility::SinkInput => {
                    match operation {
                        Operation::New | Operation::Changed => {
                            context.lock().unwrap().introspect().get_sink_input_info(index,
                                move |listres| {
                                    if let ListResult::Item(i) = listres {
                                        let mut app = app.lock().unwrap();
                                        app.sink_input_list.update(i.index, SinkInputEntry::from(i));
                                    }
                                }
                            );
                        },
                        Operation::Removed => {
                            let mut app = app.lock().unwrap();
                            app.sink_input_list.remove(index);
                        }
                    }
                }

                Facility::SourceOutput => {
                    match operation {
                        Operation::New | Operation::Changed => {
                            context.lock().unwrap().introspect().get_source_output_info(index,
                                move |listres| {
                                    if let ListResult::Item(i) = listres {
                                        let mut app = app.lock().unwrap();
                                        app.source_output_list.update(i.index, SourceOutputEntry::from(i));
                                    }
                                }
                            );
                        },
                        Operation::Removed => {
                            let mut app = app.lock().unwrap();
                            app.source_output_list.remove(index);
                        }
                    }
                }

                Facility::Sink => {
                    match operation {
                        Operation::New | Operation::Changed => {
                            context.lock().unwrap().introspect().get_sink_info_by_index(index,
                                move |listres| {
                                    if let ListResult::Item(i) = listres {
                                        let mut app = app.lock().unwrap();
                                        app.sink_list.update(i.index, SinkEntry::from(i));
                                    }
                                }
                            );
                        },
                        Operation::Removed => {
                            let mut app = app.lock().unwrap();
                            app.sink_list.remove(index);
                        }
                    }
                }

                Facility::Source => {
                    match operation {
                        Operation::New | Operation::Changed => {
                            context.lock().unwrap().introspect().get_source_info_by_index(index,
                                move |listres| {
                                    if let ListResult::Item(i) = listres {
                                        let mut app = app.lock().unwrap();
                                        app.source_list.update(i.index, SourceEntry::from(i));
                                    }
                                }
                            );
                        },
                        Operation::Removed => {
                            let mut app = app.lock().unwrap();
                            app.source_list.remove(index);
                        }
                    }
                }

                Facility::Card => {
                    match operation {
                        Operation::New | Operation::Changed => {
                            context.lock().unwrap().introspect().get_card_info_by_index(index,
                                move |listres| {
                                    if let ListResult::Item(i) = listres {
                                        let mut app = app.lock().unwrap();
                                        app.card_list.update(i.index, CardEntry::from(i));
                                    }
                                }
                            );
                        },
                        Operation::Removed => {
                            let mut app = app.lock().unwrap();
                            app.card_list.remove(index);
                        }
                    }
                }

                _ => {}
            }

        }
    })));


    // // Terminal initialization
    let mut terminal = match setup_terminal() {
        Ok(v)  => { v }
        Err(_) => { return; }
    };

    // Start event/input handling thread
    thread::spawn({
        let app = Arc::clone(&app);
        let context = Arc::clone(&context);
        move || {
            event_handler_thread(&app, &context);
        }
    });

    // Run PA mainloop
    loop {

        let timeout = pulse::time::MicroSeconds(10000); // in microseconds

        match mainloop_iter(&mut mainloop.borrow_mut(), Some(timeout)) {
            IterateResult::Quit(_) => {
                return;
            }
            IterateResult::Err(_) => {
                eprintln!("iterate state was not success, quitting...");
                return;
            },
            IterateResult::Success(_) => {},
        }

        draw_frame(&mut terminal, &mut app.lock().unwrap());

        // {
        //     let mut app = app.lock().unwrap();
        //
        //     if app.redraw {
        //         app.redraw = false;
        //
        //         if let Some(selected) = app.sink_input_list.get_selected() {
        //             println!("Selected: {} {}", selected.index, selected.name);
        //         }
        //
        //         for se in app.sink_input_list.values() {
        //             println!("{} {}", se.index, se.name);
        //         }
        //         println!("");
        //     }
        // }

        if app.lock().unwrap().quit_request {
            mainloop.borrow_mut().quit(Retval(0));
        }

    }
}

fn event_handler_thread(app: &Mutex<App>, context: &Mutex<Context>) {
    let stdin = io::stdin();
    for evt in stdin.keys() {
        match evt {
            Ok(key) => {
                handle_key_event(key, app, context);
            }
            Err(_) => {}
        }
    }
}

fn handle_key_event(key: Key, app: &Mutex<App>, context: &Mutex<Context>) {

    let mut app = app.lock().unwrap();
    let context = context.lock().unwrap();

    match key {
        Key::Ctrl('c') |
        Key::Char('q') => {
            app.quit_request = true;
            return;
        }
        _ => {}
    }

    match key {
        Key::F(1) => { app.view = AppView::SinkInputs;    views::sink_inputs::entered(&mut app);    app.redraw = true; return; }
        Key::F(2) => { app.view = AppView::SourceOutputs; views::source_outputs::entered(&mut app); app.redraw = true; return; }
        Key::F(3) => { app.view = AppView::Sinks;         views::sinks::entered(&mut app);          app.redraw = true; return; }
        Key::F(4) => { app.view = AppView::Sources;       views::sources::entered(&mut app);        app.redraw = true; return; }
        Key::F(5) => { app.view = AppView::Cards;         views::cards::entered(&mut app);          app.redraw = true; return; }
        _ => {}
    }

    if key == Key::Char('\t') {
        match app.view {
            AppView::SinkInputs    => { app.view = AppView::SourceOutputs; views::source_outputs::entered(&mut app); }
            AppView::SourceOutputs => { app.view = AppView::Sinks;         views::sinks::entered(&mut app);          }
            AppView::Sinks         => { app.view = AppView::Sources;       views::sources::entered(&mut app);        }
            AppView::Sources       => { app.view = AppView::Cards;         views::cards::entered(&mut app);          }
            AppView::Cards         => { app.view = AppView::SinkInputs;    views::sink_inputs::entered(&mut app);    }
        }
        app.redraw = true;
        return;
    }

    if key == Key::Char('M') {
        app.hide_monitors = !app.hide_monitors;
        if app.hide_monitors {
            app.source_list.filtered_select_next_else_prev(|x| !x.is_monitor());

            {
                let app = &mut *app; // XXX
                let s_list = &app.source_list;
                app.source_output_list.filtered_select_next_else_prev(|x|
                    !(x.source_index == 0xffffffff || s_list.get(x.source_index).map(|x| x.is_monitor()).unwrap_or(false))
                );
            }
        }
        app.redraw = true;
        return;
    }

    match app.view {
        AppView::SinkInputs    => { views::sink_inputs::handle_key_event(key, &mut app, &context); }
        AppView::SourceOutputs => { views::source_outputs::handle_key_event(key, &mut app, &context); }
        AppView::Sinks         => { views::sinks::handle_key_event(key, &mut app, &context); }
        AppView::Sources       => { views::sources::handle_key_event(key, &mut app, &context); }
        AppView::Cards         => { views::cards::handle_key_event(key, &mut app, &context); }
    }
}
