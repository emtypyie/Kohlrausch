#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

slint::include_modules!();
mod app_state;
mod board;
mod config;
mod scene;

use app_state::SelectionState;
use clap::{Parser, Subcommand};
use config::{load_or_create_default, load_macro_library, save_macro_library, save_profile, KeyAction, MacroDefinition, MacroLibrary, MacroLoopMode, MacroStep, MacroTiming, MouseActionKind, Profile};
use scene::Camera;
use slint::{Image, Rgba8Pixel, SharedPixelBuffer, Timer, TimerMode};
use std::{cell::{Cell, RefCell}, error::Error, rc::Rc, time::{Duration, Instant}};

const SCENE_WIDTH: usize = 840;
const SCENE_HEIGHT: usize = 540;

#[derive(Debug, Parser)]
#[command(name = "kohlrausch", version, about = "Kohlrausch Konfig macropad configuration")]
struct Cli {
    /// Open the Config Studio workspace
    #[arg(long, visible_alias = "c")]
    config: bool,
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    Device { #[command(subcommand)] command: DeviceCommand },
    Profiles { #[command(subcommand)] command: ProfileCommand },
    Flash { #[arg(long)] dry_run: bool },
}
#[derive(Debug, Subcommand)]
enum DeviceCommand { Status }
#[derive(Debug, Subcommand)]
enum ProfileCommand { List }

fn attach_parent_console() {
    #[cfg(windows)]
    if std::env::args_os().len() > 1 {
        use windows_sys::Win32::System::Console::{
            AttachConsole, ATTACH_PARENT_PROCESS,
        };

        unsafe {
            let _ = AttachConsole(ATTACH_PARENT_PROCESS);
        }
    }
}

fn write_cli_output(text: &str, is_error: bool) {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::Console::{
            GetConsoleMode, GetStdHandle, WriteConsoleW, STD_ERROR_HANDLE, STD_OUTPUT_HANDLE,
        };
        use windows_sys::Win32::Storage::FileSystem::WriteFile;

        let handle = GetStdHandle(if is_error { STD_ERROR_HANDLE } else { STD_OUTPUT_HANDLE });
        if handle.is_null() || handle as isize == -1 {
            return;
        }

        let mut mode = 0;
        if GetConsoleMode(handle, &mut mode) != 0 {
            let wide: Vec<u16> = text.encode_utf16().collect();
            let mut written = 0;
            let _ = WriteConsoleW(handle, wide.as_ptr(), wide.len() as u32, &mut written, std::ptr::null());
        } else {
            let mut written = 0;
            let _ = WriteFile(handle, text.as_ptr(), text.len() as u32, &mut written, std::ptr::null_mut());
        }
    }
    #[cfg(not(windows))]
    if is_error {
        eprint!("{text}");
    } else {
        print!("{text}");
    }
}

fn run_cli(command: Command) {
    match command {
        Command::Device { command: DeviceCommand::Status } => write_cli_output("Device status: no compatible Kohlrausch device is connected.\n", false),
        Command::Profiles { command: ProfileCommand::List } => match config::load_profiles() {
            Ok(profiles) => for profile in profiles { write_cli_output(&format!("{} ({}) · {} assignments\n", profile.name, profile.profile_id, profile.assignments.len()), false); },
            Err(error) => { write_cli_output(&format!("Profile storage error: {error}\n"), true); std::process::exit(1); }
        },
        Command::Flash { dry_run: true } => write_cli_output("No firmware operation initiated: no compatible device is connected.\n", false),
        Command::Flash { dry_run: false } => { write_cli_output("Firmware update unavailable: no compatible device is connected.\n", true); std::process::exit(2); }
    }
}

fn display_action(action: &KeyAction) -> String {
    match action {
        KeyAction::None => "Unassigned".into(),
        KeyAction::KeyChord { keys } => format!("Key chord · {}", keys.join(" + ")),
        KeyAction::MacroReference { macro_id } => format!("Macro · {macro_id}"),
        KeyAction::Website { url, label } => format!("Website · {}", label.as_deref().unwrap_or(url)),
        KeyAction::Application { name, .. } => format!("Application · {name}"),
        KeyAction::MouseAction { action } => format!("Mouse · {}", mouse_action_label(*action)),
    }
}

fn display_action_with_library(action: &KeyAction, library: &MacroLibrary) -> String {
    match action {
        KeyAction::MacroReference { macro_id } => library.find(macro_id).map(|item| format!("Macro · {}", item.name)).unwrap_or_else(|| format!("Missing macro · {macro_id}")),
        _ => display_action(action),
    }
}

fn application_location(value: &str) -> (Option<String>, Option<String>) {
    let trimmed = value.trim();
    if trimmed.is_empty() { return (None, None); }
    if trimmed.contains('\\') || trimmed.starts_with('/') || trimmed.ends_with(".exe") || trimmed.ends_with(".app") {
        (Some(trimmed.to_owned()), None)
    } else { (None, Some(trimmed.to_owned())) }
}

fn mouse_action_label(action: MouseActionKind) -> &'static str {
    match action { MouseActionKind::LeftClick => "Left click", MouseActionKind::RightClick => "Right click", MouseActionKind::DoubleClick => "Double click", MouseActionKind::ScrollUp => "Scroll up", MouseActionKind::ScrollDown => "Scroll down" }
}

fn set_assignment(ui: &MainWindow, profile: &Profile, state: &SelectionState, macros: &MacroLibrary) {
    let key = state.selected_key();
    let assigned = profile.action(key.physical_id, state.active_layer);
    let action = display_action_with_library(assigned, macros);
    let (editor_kind, editor_value, editor_extra, editor_arguments) = match assigned {
        KeyAction::None => ("keyChord", String::new(), String::new(), "[]".to_owned()),
        KeyAction::KeyChord { keys } => ("keyChord", keys.join("+"), String::new(), "[]".to_owned()),
        KeyAction::MacroReference { macro_id } => ("macro", macro_id.clone(), String::new(), "[]".to_owned()),
        KeyAction::Website { url, label } => ("website", url.clone(), label.clone().unwrap_or_default(), "[]".to_owned()),
        KeyAction::Application { name, executable_path, application_id, arguments } => ("application", name.clone(), executable_path.clone().or(application_id.clone()).unwrap_or_default(), serde_json::to_string(arguments).unwrap_or_else(|_| "[]".to_owned())),
        KeyAction::MouseAction { action } => ("mouseAction", mouse_action_label(*action).to_owned(), String::new(), "[]".to_owned()),
    };
    ui.set_assignment_text(action.clone().into());
    ui.set_editor_kind(editor_kind.into());
    ui.set_action_value(editor_value.into());
    ui.set_action_extra(editor_extra.into());
    ui.set_action_arguments(editor_arguments.into());
    ui.set_editor_dirty(false);
    ui.set_editor_error("".into());
    ui.set_capturing_shortcut(false);
    let options = macros.macros.iter().map(|item| format!("{} · {} step(s) · {} · {}", item.name, item.steps.len(), display_action(&item.steps[0].action), item.macro_id).into()).collect::<Vec<slint::SharedString>>();
    let names = macros.macros.iter().map(|item| item.name.clone().into()).collect::<Vec<slint::SharedString>>();
    let macro_index = match assigned { KeyAction::MacroReference { macro_id } => macros.macros.iter().position(|item| &item.macro_id == macro_id).map(|index| index as i32).unwrap_or(-1), _ => -1 };
    let macro_detail = match assigned {
        KeyAction::MacroReference { macro_id } => macros.find(macro_id).map(|item| format!("{} · {} step(s) · {}", item.name, item.steps.len(), display_action(&item.steps[0].action))).unwrap_or_else(|| format!("Missing macro · {macro_id}")),
        _ => "Select a saved macro".to_owned(),
    };
    ui.set_macro_options(model_rc(options));
    ui.set_macro_names(model_rc(names));
    ui.set_inspector_macro_index(macro_index);
    ui.set_macro_detail(macro_detail.into());
    if matches!(assigned, KeyAction::MacroReference { macro_id } if macros.find(macro_id).is_none()) {
        ui.set_editor_error("This assignment refers to a macro that is not in the saved macro library.".into());
    }
    ui.set_selected_key(state.selected_logical_id.clone().into());
    ui.set_selected_key_label(key.physical_reference.into());
    ui.set_physical_reference(key.physical_reference.into());
    ui.set_physical_key_id(key.physical_id.into());
    ui.set_active_layer(state.active_layer as i32);
    ui.set_inspector_title(format!("{}  /  LAYER {}", key.label, state.active_layer).into());
}

fn update_scene(
    ui: &MainWindow,
    camera: Camera,
    cache: &Rc<RefCell<Option<scene::SceneFrame>>>,
    width: usize,
    height: usize,
) {
    let frame = scene::render(width, height, camera);
    let pixels = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        &frame.rgba,
        width as u32,
        height as u32,
    );
    ui.set_scene_image(Image::from_rgba8(pixels));
    *cache.borrow_mut() = Some(frame);
}

fn update_hover_overlay(
    ui: &MainWindow,
    profile: &Profile,
    state: &SelectionState,
    frame: Option<&scene::SceneFrame>,
) {
    let Some(key) = state.hovered_logical_id.as_deref().and_then(board::find_logical) else {
        ui.set_hover_visible(false);
        return;
    };
    ui.set_hover_visible(true);
    ui.set_hover_label(key.physical_reference.into());
    ui.set_hover_hardware(format!("{} · {}", key.physical_reference, key.physical_id).into());
    ui.set_hover_matrix("Unknown · schematic verification required".into());
    ui.set_hover_gpio("Unknown · schematic verification required".into());
    ui.set_hover_layer(state.active_layer as i32);
    ui.set_hover_action(display_action(profile.action(key.physical_id, state.active_layer)).into());
    let Some(frame) = frame else { return; };
    let Some(anchor) = frame.anchor_for(key.id) else { return; };
    ui.set_hover_anchor_x(anchor.0 * 840.0 / frame.width as f32);
    ui.set_hover_anchor_y(anchor.1 * 540.0 / frame.height as f32);
}

fn update_selection_marker(
    ui: &MainWindow,
    state: &SelectionState,
    frame: Option<&scene::SceneFrame>,
) {
    let Some(frame) = frame else {
        ui.set_selection_visible(false);
        return;
    };
    let Some(anchor) = frame.anchor_for(&state.selected_logical_id) else {
        ui.set_selection_visible(false);
        return;
    };
    ui.set_selection_anchor_x(anchor.0 * 840.0 / frame.width as f32);
    ui.set_selection_anchor_y(anchor.1 * 540.0 / frame.height as f32);
    ui.set_selection_visible(true);
}

fn main() -> Result<(), Box<dyn Error>> {
    attach_parent_console();
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            write_cli_output(&error.to_string(), error.use_stderr());
            std::process::exit(error.exit_code());
        }
    };
    if let Some(command) = cli.command { run_cli(command); return Ok(()); }
    board::validate_definition(board::BOARD_JSON).expect("embedded board definition must match the physical layout");

    let profile = load_or_create_default().unwrap_or_else(|error| {
        eprintln!("Could not load profile: {error}"); Profile::default_profile()
    });
    let profile = Rc::new(RefCell::new(profile));
    let macro_load = load_macro_library();
    let macros = Rc::new(RefCell::new(macro_load.clone().unwrap_or_default()));
    let macro_draft: Rc<RefCell<Vec<MacroStep>>> = Rc::new(RefCell::new(Vec::new()));
    let macro_selected_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let last_macro_capture: Rc<RefCell<Option<Instant>>> = Rc::new(RefCell::new(None));
    let assign_macro_on_save = Rc::new(Cell::new(false));
    let state = Rc::new(RefCell::new(SelectionState::default()));
    let scene_cache = Rc::new(RefCell::new(None));
    let ui = MainWindow::new()?;
    ui.set_profile_name(profile.borrow().name.clone().into());
    ui.set_raw_config(serde_json::to_string_pretty(&*profile.borrow())?.into());
    ui.set_status_message("Local profile · hardware not connected".into());
    ui.set_macro_preview(macro_library_summary(&macros.borrow()).into());
    refresh_macro_library(&ui, &macros.borrow());
    refresh_macro_events(&ui, &macro_draft.borrow());
    ui.set_board_options(model_rc(vec!["Kohlrausch RP2040 · Rev A (provisional)".into()]));
    ui.set_board_index(0);
    ui.set_board_name("Kohlrausch RP2040 · Rev A (provisional)".into());
    let ports = enumerate_ports();
    ui.set_ports_available(!ports.is_empty());
    ui.set_port_options(model_rc(port_choices(&ports)));
    ui.set_port_index(0);
    ui.set_port_status("Select a port to inspect · no device handshake has occurred".into());
    set_assignment(&ui, &profile.borrow(), &state.borrow(), &macros.borrow());
    if let Err(error) = macro_load { ui.set_status_message(format!("Macro library could not be loaded: {error}").into()); }
    update_scene(&ui, Camera::default(), &scene_cache, SCENE_WIDTH, SCENE_HEIGHT);
    update_selection_marker(&ui, &state.borrow(), scene_cache.borrow().as_ref());

    let weak = ui.as_weak();
    ui.on_navigate(move |section| if let Some(ui) = weak.upgrade() {
        if section.as_str() != "Macros" && ui.get_macro_recording() { ui.set_macro_recording(false); }
        ui.set_active_section(section);
    });

    let state_for_key = state.clone();
    let profile_for_key = profile.clone();
    let macros_for_key = macros.clone();
    let scene_for_key = scene_cache.clone();
    let weak = ui.as_weak();
    ui.on_select_key(move |logical_id| {
        let mut current = state_for_key.borrow_mut();
        if !current.select(logical_id.as_str()) { return; }
        if let Some(ui) = weak.upgrade() {
            set_assignment(&ui, &profile_for_key.borrow(), &current, &macros_for_key.borrow());
            update_selection_marker(&ui, &current, scene_for_key.borrow().as_ref());
        }
    });

    let state_for_layer = state.clone();
    let profile_for_layer = profile.clone();
    let macros_for_layer = macros.clone();
    let scene_for_layer = scene_cache.clone();
    let weak = ui.as_weak();
    ui.on_select_layer(move |number| {
        let mut current = state_for_layer.borrow_mut();
        if !(1..=4).contains(&number) || !current.set_layer(number as u8) { return; }
        if let Some(ui) = weak.upgrade() {
            set_assignment(&ui, &profile_for_layer.borrow(), &current, &macros_for_layer.borrow());
            update_selection_marker(&ui, &current, scene_for_layer.borrow().as_ref());
            update_hover_overlay(&ui, &profile_for_layer.borrow(), &current, scene_for_layer.borrow().as_ref());
        }
    });

    let state_for_lyr = state.clone();
    let profile_for_lyr = profile.clone();
    let macros_for_lyr = macros.clone();
    let scene_for_lyr = scene_cache.clone();
    let weak = ui.as_weak();
    ui.on_cycle_lyr(move || {
        let mut current = state_for_lyr.borrow_mut();
        let layer = current.advance_layer();
        if let Some(ui) = weak.upgrade() {
            set_assignment(&ui, &profile_for_lyr.borrow(), &current, &macros_for_lyr.borrow());
            update_selection_marker(&ui, &current, scene_for_lyr.borrow().as_ref());
            update_hover_overlay(&ui, &profile_for_lyr.borrow(), &current, scene_for_lyr.borrow().as_ref());
            ui.set_status_message(format!("LYR press preview · now editing layer {layer}").into());
        }
    });

    let state_for_save = state.clone();
    let profile_for_save = profile.clone();
    let scene_for_save = scene_cache.clone();
    let draft_for_add = macro_draft.clone();
    let weak = ui.as_weak();
    ui.on_add_macro_step(move |mode, value, delay| {
        let result = macro_step_from_input(&mode, &value, &delay);
        if let Some(ui) = weak.upgrade() {
            match result {
                Ok(step) => {
                    let mut draft = draft_for_add.borrow_mut();
                    if draft.len() >= 64 { ui.set_macro_error("A macro can contain at most 64 events".into()); return; }
                    draft.push(step.clone());
                    refresh_macro_events(&ui, &draft);
                    ui.set_macro_error("".into());
                    ui.set_macro_event_index((draft.len() as i32) - 1);
                    ui.set_status_message(format!("Added step {} · preview only until macro is saved", draft.len()).into());
                }
                Err(error) => { ui.set_macro_error(error.clone().into()); ui.set_status_message(error.into()); }
            }
        }
    });

    let draft_for_new = macro_draft.clone();
    let selected_for_new = macro_selected_id.clone();
    let last_capture_for_new = last_macro_capture.clone();
    let weak = ui.as_weak();
    ui.on_new_macro(move || {
        *draft_for_new.borrow_mut() = Vec::new();
        *selected_for_new.borrow_mut() = None;
        *last_capture_for_new.borrow_mut() = None;
        if let Some(ui) = weak.upgrade() {
            ui.set_macro_index(-1);
            ui.set_macro_name("".into());
            ui.set_macro_event_index(-1);
            ui.set_macro_error("".into());
            ui.set_macro_recording(false);
            ui.set_macro_timing("Fixed delay".into());
            ui.set_macro_delay("1".into());
            ui.set_macro_loop("Once".into());
            ui.set_macro_loop_count("1".into());
            refresh_macro_events(&ui, &draft_for_new.borrow());
            ui.set_status_message("New macro draft · add events, then save".into());
        }
    });

    let draft_for_select = macro_draft.clone();
    let selected_for_select = macro_selected_id.clone();
    let macros_for_select = macros.clone();
    let weak = ui.as_weak();
    ui.on_select_macro(move |index| {
        let library = macros_for_select.borrow();
        let Some(definition) = usize::try_from(index).ok().and_then(|index| library.macros.get(index)) else { return; };
        *selected_for_select.borrow_mut() = Some(definition.macro_id.clone());
        *draft_for_select.borrow_mut() = definition.steps.clone();
        if let Some(ui) = weak.upgrade() {
            ui.set_macro_index(index);
            ui.set_macro_name(definition.name.clone().into());
            ui.set_macro_timing(match definition.timing { MacroTiming::Recorded => "Recorded delay", MacroTiming::Fixed => "Fixed delay" }.into());
            ui.set_macro_delay(definition.fixed_delay_ms.to_string().into());
            match definition.loop_mode {
                MacroLoopMode::Once => { ui.set_macro_loop("Once".into()); ui.set_macro_loop_count("1".into()); }
                MacroLoopMode::UntilStop => ui.set_macro_loop("Until stop".into()),
                MacroLoopMode::UntilAnyClick => ui.set_macro_loop("Until any click".into()),
                MacroLoopMode::Count(count) => { ui.set_macro_loop("Loop count".into()); ui.set_macro_loop_count(count.to_string().into()); }
            }
            ui.set_macro_event_index(-1);
            ui.set_macro_error("".into());
            refresh_macro_events(&ui, &definition.steps);
            ui.set_status_message(format!("Editing macro · {}", definition.name).into());
        }
    });

    let draft_for_remove = macro_draft.clone();
    let weak = ui.as_weak();
    ui.on_remove_macro_event(move |index| {
        if let Ok(index) = usize::try_from(index) {
            let mut draft = draft_for_remove.borrow_mut();
            if index < draft.len() { draft.remove(index); }
            if let Some(ui) = weak.upgrade() {
                refresh_macro_events(&ui, &draft);
                ui.set_macro_event_index(-1);
                ui.set_macro_error("".into());
                ui.set_status_message("Event removed from the unsaved macro draft".into());
            }
        }
    });

    let draft_for_clear = macro_draft.clone();
    let weak = ui.as_weak();
    ui.on_clear_macro_events(move || {
        draft_for_clear.borrow_mut().clear();
        if let Some(ui) = weak.upgrade() {
            refresh_macro_events(&ui, &draft_for_clear.borrow());
            ui.set_macro_event_index(-1);
            ui.set_macro_error("".into());
            ui.set_status_message("Events cleared from the unsaved macro draft".into());
        }
    });

    let macros_for_delete = macros.clone();
    let draft_for_delete = macro_draft.clone();
    let selected_for_delete = macro_selected_id.clone();
    let weak = ui.as_weak();
    ui.on_delete_macro(move |index| {
        let result = usize::try_from(index).map_err(|_| "Select a saved macro first".to_owned()).and_then(|index| {
            let mut library = macros_for_delete.borrow().clone();
            if index >= library.macros.len() { return Err("The selected macro no longer exists".into()); }
            let macro_id = library.macros[index].macro_id.clone();
            let profiles = config::load_profiles()?;
            if profiles.iter().any(|profile| profile.assignments.iter().any(|assignment| matches!(&assignment.action, KeyAction::MacroReference { macro_id: assigned } if assigned == &macro_id))) {
                return Err("This macro is assigned in a profile. Remove those assignments before deleting it.".into());
            }
            library.macros.remove(index);
            save_macro_library(&library)?;
            *macros_for_delete.borrow_mut() = library;
            *draft_for_delete.borrow_mut() = Vec::new();
            *selected_for_delete.borrow_mut() = None;
            Ok(())
        });
        if let Some(ui) = weak.upgrade() {
            match result {
                Ok(()) => {
                    let library = macros_for_delete.borrow();
                    refresh_macro_library(&ui, &library);
                    refresh_macro_events(&ui, &draft_for_delete.borrow());
                    ui.set_macro_name("".into()); ui.set_macro_index(-1); ui.set_macro_event_index(-1);
                    ui.set_status_message("Macro deleted from the local library".into());
                }
                Err(error) => { ui.set_macro_error(error.clone().into()); ui.set_status_message(error.into()); }
            }
        }
    });

    let last_for_capture = last_macro_capture.clone();
    let weak = ui.as_weak();
    ui.on_macro_recording_changed(move |recording| {
        *last_for_capture.borrow_mut() = None;
        if let Some(ui) = weak.upgrade() {
            if recording { ui.set_status_message("Recording focused-window key presses only · press Stop when finished".into()); }
        }
    });
    let draft_for_capture = macro_draft.clone();
    let last_for_capture = last_macro_capture.clone();
    let weak = ui.as_weak();
    ui.on_capture_macro_key(move |chord| {
        let result = config::normalize_key_chord(chord.as_str());
        if let Some(ui) = weak.upgrade() {
            if !ui.get_macro_recording() || ui.get_active_section() != "Macros" { return; }
            match result {
                Ok(keys) => {
                    let mut draft = draft_for_capture.borrow_mut();
                    if draft.len() >= 64 { ui.set_macro_error("A macro can contain at most 64 events".into()); return; }
                    if let Some(previous) = draft.last_mut() {
                        previous.delay_ms = if ui.get_macro_timing() == "Fixed delay" {
                            ui.get_macro_delay().parse::<u32>().unwrap_or(0).min(600_000)
                        } else {
                            last_for_capture.borrow().map(|last| last.elapsed().as_millis().min(600_000) as u32).unwrap_or(0)
                        };
                    }
                    draft.push(MacroStep { delay_ms: 0, action: KeyAction::KeyChord { keys } });
                    *last_for_capture.borrow_mut() = Some(Instant::now());
                    refresh_macro_events(&ui, &draft);
                    ui.set_macro_event_index((draft.len() as i32) - 1);
                    ui.set_macro_error("".into());
                    ui.set_status_message(format!("Captured event {} · not sent to the operating system", draft.len()).into());
                }
                Err(error) => ui.set_macro_error(error.into()),
            }
        }
    });

    let macros_for_save = macros.clone();
    let draft_for_save = macro_draft.clone();
    let weak = ui.as_weak();
    ui.on_save_action(move |kind, value, extra, arguments| {
        let current = state_for_save.borrow();
        let key = current.selected_key();
        let action = match kind.as_str() {
            "none" => Ok(KeyAction::None),
            "keyChord" => config::normalize_key_chord(value.as_str()).map(|keys| KeyAction::KeyChord { keys }),
            "macro" => {
                let macro_id = value.rsplit_once(" · ").map(|(_, id)| id).unwrap_or(value.as_str()).trim();
                if macros_for_save.borrow().find(macro_id).is_some() { Ok(KeyAction::MacroReference { macro_id: macro_id.to_owned() }) }
                else { Err("Select a saved macro. The referenced macro is missing.".to_owned()) }
            }
            "website" => Ok(KeyAction::Website { url: value.to_string(), label: if extra.trim().is_empty() { None } else { Some(extra.to_string()) } }),
            "application" => {
                let args = serde_json::from_str::<Vec<String>>(&arguments).map_err(|_| "Arguments must be a JSON array of strings, such as [\"--new-window\"]".to_owned());
                args.map(|arguments| { let (executable_path, application_id) = application_location(extra.as_str()); KeyAction::Application { name: value.to_string(), executable_path, application_id, arguments } })
            }
            "mouseAction" => {
                let mouse = match value.as_str() { "Left click" => Some(MouseActionKind::LeftClick), "Right click" => Some(MouseActionKind::RightClick), "Double click" => Some(MouseActionKind::DoubleClick), "Scroll up" => Some(MouseActionKind::ScrollUp), "Scroll down" => Some(MouseActionKind::ScrollDown), _ => None };
                mouse.map(|action| KeyAction::MouseAction { action }).ok_or_else(|| "Choose a supported click or scroll action".to_owned())
            }
            _ => Err("Choose an action type.".to_owned()),
        };
        let result = action.and_then(|action| {
            let mut candidate = profile_for_save.borrow().clone();
            candidate.set_action(key.physical_id, current.active_layer, action);
            candidate.validate()?;
            save_profile(&candidate)?;
            *profile_for_save.borrow_mut() = candidate;
            Ok(())
        });
        if let Some(ui) = weak.upgrade() {
            match result {
                Ok(()) => {
                    let profile = profile_for_save.borrow();
                    set_assignment(&ui, &profile, &current, &macros_for_save.borrow());
                    update_hover_overlay(&ui, &profile, &current, scene_for_save.borrow().as_ref());
                    ui.set_status_message("Saved to the active profile".into());
                }
                Err(error) => { ui.set_editor_error(error.clone().into()); ui.set_status_message(error.into()); }
            }
        }
    });

    let profile_for_cancel = profile.clone();
    let state_for_cancel = state.clone();
    let macros_for_cancel = macros.clone();
    let weak = ui.as_weak();
    ui.on_cancel_action(move || if let Some(ui) = weak.upgrade() {
        set_assignment(&ui, &profile_for_cancel.borrow(), &state_for_cancel.borrow(), &macros_for_cancel.borrow());
    });

    let weak = ui.as_weak();
    ui.on_capture_key(move |chord| if let Some(ui) = weak.upgrade() {
        match config::normalize_key_chord(chord.as_str()) {
            Ok(keys) => { ui.set_action_value(keys.join("+").into()); ui.set_editor_dirty(true); ui.set_editor_error("".into()); }
            Err(error) => ui.set_editor_error(error.into()),
        }
    });

    let weak = ui.as_weak();
    ui.on_test_action(move || if let Some(ui) = weak.upgrade() {
        ui.set_status_message("Shortcut preview only · no keyboard input was sent".into());
    });

    let assign_macro_for_open = assign_macro_on_save.clone();
    let selected_macro_for_open = macro_selected_id.clone();
    let macro_draft_for_open = macro_draft.clone();
    let weak = ui.as_weak();
    ui.on_open_macro_editor(move || if let Some(ui) = weak.upgrade() {
        assign_macro_for_open.set(true);
        *selected_macro_for_open.borrow_mut() = None;
        macro_draft_for_open.borrow_mut().clear();
        ui.set_macro_index(-1);
        ui.set_macro_name("".into());
        ui.set_macro_error("".into());
        refresh_macro_events(&ui, &macro_draft_for_open.borrow());
        ui.set_active_section("Macros".into());
        ui.set_status_message("Create and save a macro, then assign it from the Macro action tab".into());
    });

    let scene_for_pick = scene_cache.clone();
    ui.on_pick_key(move |x, y| {
        scene_for_pick.borrow().as_ref().and_then(|frame| {
            frame.key_at(x * frame.width as f32 / 840.0, y * frame.height as f32 / 540.0)
        })
            .unwrap_or_default().into()
    });

    let state_for_hover = state.clone();
    let scene_for_hover = scene_cache.clone();
    let profile_for_hover = profile.clone();
    let weak = ui.as_weak();
    ui.on_hover_at(move |x, y| {
        let id = scene_for_hover.borrow().as_ref().and_then(|frame| {
            frame.key_at(x * frame.width as f32 / 840.0, y * frame.height as f32 / 540.0)
        }).map(str::to_owned);
        let mut current = state_for_hover.borrow_mut();
        if current.hovered_logical_id != id {
            current.hover(id.as_deref());
            if let Some(ui) = weak.upgrade() {
                update_hover_overlay(&ui, &profile_for_hover.borrow(), &current, scene_for_hover.borrow().as_ref());
            }
        }
    });

    let weak = ui.as_weak();
    ui.on_refresh_ports(move || if let Some(ui) = weak.upgrade() {
        let ports = enumerate_ports();
        ui.set_ports_available(!ports.is_empty());
        ui.set_port_options(model_rc(port_choices(&ports)));
        ui.set_port_index(0);
        ui.set_port_status(if ports.is_empty() { "No serial ports found · no device handshake has occurred".into() } else { "Port list refreshed · selection does not imply a device connection".into() });
    });
    let weak = ui.as_weak();
    ui.on_flash_request(move || if let Some(ui) = weak.upgrade() {
        ui.set_status_message("Firmware flashing unavailable · no firmware operation initiated".into());
    });
    let weak = ui.as_weak();
    ui.on_port_selected(move |name| if let Some(ui) = weak.upgrade() {
        if name.as_str() == "No serial ports found" { return; }
        ui.set_selected_port(name.clone());
        ui.set_port_status(format!("Port selected: {name} · not opened or verified").into());
    });
    let weak = ui.as_weak();
    ui.on_board_selected(move |name| if let Some(ui) = weak.upgrade() {
        ui.set_board_name(name.clone());
        ui.set_status_message("Board selection changed · hardware metadata remains fixed".into());
    });

    let macros_for_save = macros.clone();
    let assign_macro_for_save = assign_macro_on_save.clone();
    let profile_for_macro = profile.clone();
    let state_for_macro = state.clone();
    let selected_for_save = macro_selected_id.clone();
    let weak = ui.as_weak();
    ui.on_save_macro(move |name, timing, delay, loop_mode, loop_count| {
        let result = update_macro_draft(
            &name,
            selected_for_save.borrow().as_deref(),
            &draft_for_save.borrow(),
            timing.as_str(), delay.as_str(), loop_mode.as_str(), loop_count.as_str(),
            &macros_for_save.borrow(),
        );
        if let Some(ui) = weak.upgrade() {
            match result.and_then(|(candidate, macro_item)| {
                save_macro_library(&candidate)?;
                *macros_for_save.borrow_mut() = candidate;
                let should_assign = assign_macro_for_save.replace(false);
                if should_assign {
                    let state = state_for_macro.borrow();
                    let mut profile = profile_for_macro.borrow().clone();
                    profile.set_action(state.selected_key().physical_id, state.active_layer, KeyAction::MacroReference { macro_id: macro_item.macro_id.clone() });
                    profile.validate()?;
                    save_profile(&profile)?;
                    *profile_for_macro.borrow_mut() = profile;
                }
                Ok((macro_item, should_assign))
            }) {
                Ok((macro_item, should_assign)) => {
                    *selected_for_save.borrow_mut() = Some(macro_item.macro_id.clone());
                    ui.set_macro_index(macros_for_save.borrow().macros.iter().position(|item| item.macro_id == macro_item.macro_id).unwrap_or(0) as i32);
                    ui.set_macro_name(macro_item.name.clone().into());
                    ui.set_macro_error("".into());
                    refresh_macro_library(&ui, &macros_for_save.borrow());
                    refresh_macro_events(&ui, &draft_for_save.borrow());
                    if should_assign {
                        ui.set_active_section("Keymaps".into());
                        set_assignment(&ui, &profile_for_macro.borrow(), &state_for_macro.borrow(), &macros_for_save.borrow());
                        ui.set_status_message("Macro saved locally and assigned · host action not executed".into());
                    } else { ui.set_status_message("Macro saved locally · no host action executed".into()); }
                }
                Err(error) => { ui.set_macro_error(error.clone().into()); ui.set_status_message(error.into()); }
            }
        }
    });

    let profile_for_name = profile.clone();
    let weak = ui.as_weak();
    ui.on_rename_profile(move |name| {
        let mut candidate = profile_for_name.borrow().clone(); candidate.name = name.trim().to_owned();
        let result = candidate.validate().and_then(|()| save_profile(&candidate)).map(|()| candidate);
        if let Some(ui) = weak.upgrade() { match result {
            Ok(candidate) => { *profile_for_name.borrow_mut() = candidate.clone(); ui.set_profile_name(candidate.name.into()); ui.set_status_message("Profile name saved".into()); }
            Err(error) => ui.set_status_message(error.into()),
        }}
    });
    let profile_for_create = profile.clone();
    let weak = ui.as_weak();
    ui.on_create_profile(move |name| {
        let name = name.trim().to_owned();
        let id = name.to_ascii_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().trim_matches('-').to_owned();
        let result = if name.is_empty() || id.is_empty() { Err("Enter a profile name using letters and numbers.".into()) }
        else { config::load_profiles().and_then(|items| {
            if items.iter().any(|item| item.profile_id == id) { return Err("A profile with that name already exists.".into()); }
            let mut candidate = Profile::default_profile(); candidate.profile_id = id; candidate.name = name; save_profile(&candidate)?; Ok(candidate)
        }) };
        if let Some(ui) = weak.upgrade() { match result {
            Ok(candidate) => { *profile_for_create.borrow_mut() = candidate.clone(); ui.set_profile_name(candidate.name.clone().into()); ui.set_raw_config(serde_json::to_string_pretty(&candidate).unwrap_or_default().into()); ui.set_status_message("New profile created".into()); }
            Err(error) => ui.set_status_message(error.into()),
        }}
    });
    let profile_for_raw = profile.clone();
    let state_for_raw = state.clone();
    let macros_for_raw = macros.clone();
    let weak = ui.as_weak();
    ui.on_save_raw_config(move |source| {
        let result = serde_json::from_str::<Profile>(&source).map_err(|error| format!("Invalid JSON: {error}"))
            .and_then(|candidate| { candidate.validate()?; save_profile(&candidate)?; Ok(candidate) });
        if let Some(ui) = weak.upgrade() { match result {
            Ok(candidate) => { *profile_for_raw.borrow_mut() = candidate.clone(); ui.set_profile_name(candidate.name.clone().into()); ui.set_raw_config(serde_json::to_string_pretty(&candidate).unwrap_or_default().into()); set_assignment(&ui, &candidate, &state_for_raw.borrow(), &macros_for_raw.borrow()); ui.set_status_message("Configuration validated and saved".into()); }
            Err(error) => ui.set_status_message(error.into()),
        }}
    });

    if cli.config { ui.set_active_section("Config Studio".into()); }
    let startup_timer = Rc::new(RefCell::new(Timer::default()));
    let timer_to_stop = Rc::downgrade(&startup_timer);
    let startup_step = Rc::new(Cell::new(0));
    let step_for_timer = startup_step.clone();
    let startup_window = ui.as_weak();
    startup_timer.borrow().start(TimerMode::Repeated, Duration::from_millis(90), move || {
        let next = step_for_timer.get() + 1;
        step_for_timer.set(next);
        if let Some(ui) = startup_window.upgrade() {
            ui.set_startup_sequence(next.min(9));
        }
        if next >= 9 {
            if let Some(timer) = timer_to_stop.upgrade() { timer.borrow().stop(); }
        }
    });
    ui.run()?;
    Ok(())
}

fn enumerate_ports() -> Vec<slint::SharedString> {
    let mut ports = serialport::available_ports().unwrap_or_default().into_iter().map(|port| port.port_name.into()).collect::<Vec<_>>();
    ports.sort_by(|a: &slint::SharedString, b| a.as_str().cmp(b.as_str()));
    ports
}

fn port_choices(ports: &[slint::SharedString]) -> Vec<slint::SharedString> {
    if ports.is_empty() { vec!["No serial ports found".into()] } else { ports.to_vec() }
}

#[cfg(test)]
fn create_macro(mode: &str, name: &str, value: &str, library: &MacroLibrary) -> Result<MacroDefinition, String> {
    let step = macro_step_from_input(mode, value, "0")?;
    let mut definition = create_macro_identity(name, library)?;
    definition.steps.push(step);
    definition.validate()?;
    Ok(definition)
}

fn create_macro_identity(name: &str, library: &MacroLibrary) -> Result<MacroDefinition, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 { return Err("Macro name must contain 1–64 characters.".into()); }
    if library.macros.iter().any(|item| item.name.eq_ignore_ascii_case(name)) { return Err("A macro with that name already exists.".into()); }
    let base_id = name.to_ascii_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect::<String>().trim_matches('-').to_owned();
    let base_id = if base_id.is_empty() { "macro".to_owned() } else { base_id };
    let mut macro_id = base_id.clone();
    let mut suffix = 2;
    while library.macros.iter().any(|item| item.macro_id == macro_id) { macro_id = format!("{base_id}-{suffix}"); suffix += 1; }
    Ok(MacroDefinition { schema_version: 1, macro_id, name: name.to_owned(), steps: Vec::new(), timing: MacroTiming::Fixed, fixed_delay_ms: 0, loop_mode: MacroLoopMode::Once })
}

fn macro_step_from_input(mode: &str, value: &str, delay: &str) -> Result<MacroStep, String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > 512 { return Err("Action details must contain 1–512 characters.".into()); }
    let delay_ms = delay.trim().parse::<u32>().map_err(|_| "Delay must be a whole number of milliseconds.".to_owned())?;
    let action = match mode {
        "Key chord" => KeyAction::KeyChord { keys: config::normalize_key_chord(value)? },
        "Website" => KeyAction::Website { url: value.to_owned(), label: None },
        "Software" => KeyAction::Application { name: value.to_owned(), executable_path: None, application_id: None, arguments: Vec::new() },
        "Clicks" => KeyAction::MouseAction { action: match value.to_ascii_lowercase().as_str() {
            "left click" => MouseActionKind::LeftClick, "right click" => MouseActionKind::RightClick,
            "double click" => MouseActionKind::DoubleClick, "scroll up" => MouseActionKind::ScrollUp,
            "scroll down" => MouseActionKind::ScrollDown, _ => return Err("Use Left click, Right click, Double click, Scroll up, or Scroll down.".into()),
        } },
        _ => return Err("Choose one of the supported macro modes.".into()),
    };
    let step = MacroStep { delay_ms, action };
    MacroDefinition { schema_version: 1, macro_id: "draft".into(), name: "Draft".into(), steps: vec![step.clone()], timing: MacroTiming::Fixed, fixed_delay_ms: 0, loop_mode: MacroLoopMode::Once }.validate()?;
    Ok(step)
}

fn update_macro_draft(
    name: &str,
    selected_id: Option<&str>,
    steps: &[MacroStep],
    timing: &str,
    delay: &str,
    loop_mode: &str,
    loop_count: &str,
    library: &MacroLibrary,
) -> Result<(MacroLibrary, MacroDefinition), String> {
    if steps.is_empty() { return Err("Add at least one event before saving the macro".into()); }
    let timing = match timing { "Recorded delay" => MacroTiming::Recorded, "Fixed delay" => MacroTiming::Fixed, _ => return Err("Choose recorded or fixed delay".into()) };
    let fixed_delay_ms = delay.trim().parse::<u32>().map_err(|_| "Delay must be a whole number of milliseconds".to_owned())?;
    let loop_mode = match loop_mode {
        "Once" => MacroLoopMode::Once,
        "Until stop" => MacroLoopMode::UntilStop,
        "Until any click" => MacroLoopMode::UntilAnyClick,
        "Loop count" => MacroLoopMode::Count(loop_count.trim().parse::<u16>().map_err(|_| "Loop count must be a whole number from 1 to 10,000".to_owned())?),
        _ => return Err("Choose a supported loop setting".into()),
    };
    let mut candidate = library.clone();
    let mut definition = if let Some(id) = selected_id {
        let position = candidate.macros.iter().position(|item| item.macro_id == id).ok_or_else(|| "The selected macro no longer exists".to_owned())?;
        let existing = candidate.macros.remove(position);
        if candidate.macros.iter().any(|item| item.name.eq_ignore_ascii_case(name.trim())) { return Err("A macro with that name already exists".into()); }
        existing
    } else {
        create_macro_identity(name, &candidate)?
    };
    definition.name = name.trim().to_owned();
    definition.steps = steps.to_vec();
    definition.timing = timing;
    definition.fixed_delay_ms = fixed_delay_ms;
    definition.loop_mode = loop_mode;
    definition.validate()?;
    candidate.macros.push(definition.clone());
    candidate.validate()?;
    Ok((candidate, definition))
}

fn macro_library_summary(library: &MacroLibrary) -> String {
    if library.macros.is_empty() { return "No saved macros".into(); }
    library.macros.iter().map(|item| format!("{}  ·  {} step(s)  ·  {}", item.name, item.steps.len(), item.macro_id)).collect::<Vec<_>>().join("\n")
}

fn refresh_macro_library(ui: &MainWindow, library: &MacroLibrary) {
    ui.set_macro_options(model_rc(library.macros.iter().map(|item| format!("{} · {} step(s) · {} · {}", item.name, item.steps.len(), display_action(&item.steps[0].action), item.macro_id).into()).collect()));
    ui.set_macro_names(model_rc(library.macros.iter().map(|item| item.name.clone().into()).collect()));
    ui.set_macro_preview(macro_library_summary(library).into());
}

fn refresh_macro_events(ui: &MainWindow, steps: &[MacroStep]) {
    let rows = steps.iter().map(|step| format!("{} · {} ms", display_action(&step.action), step.delay_ms)).collect::<Vec<_>>();
    ui.set_macro_events(model_rc(rows.into_iter().map(Into::into).collect()));
    ui.set_macro_preview(if steps.is_empty() { "No events recorded".into() } else { format!("{} event(s) · locally saved draft", steps.len()) }.into());
}

fn model_rc(values: Vec<slint::SharedString>) -> slint::ModelRc<slint::SharedString> {
    Rc::new(slint::VecModel::from(values)).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    #[test] fn cli_arguments_are_well_formed() { Cli::command().debug_assert(); }
    #[test] fn config_flag_is_supported() { let c=Cli::try_parse_from(["kohlrausch","--config"]).unwrap(); assert!(c.config); }
    #[test] fn short_configuration_alias_is_supported() { let c=Cli::try_parse_from(["kohlrausch","--c"]).unwrap(); assert!(c.config); }
    #[test] fn assignment_display_resolves_logical_selection_to_physical_storage() {
        let mut profile=Profile::default_profile();
        let mut state=SelectionState::default();
        state.select("logical-1-1");
        profile.set_action(state.selected_key().physical_id,1,KeyAction::KeyChord{keys:vec!["CTRL".into(),"C".into()]});
        assert_eq!(display_action(profile.action("key-3-1",1)),"Key chord · CTRL + C");
    }
    #[test] fn macro_inputs_are_validated_before_preview() {
        let library = MacroLibrary::default();
        assert!(create_macro("Website","Docs","https://example.com", &library).is_ok());
        assert!(create_macro("Website","Bad","file:///private", &library).is_err());
        assert!(create_macro("Software","App"," ", &library).is_err());
        assert!(create_macro("Unknown","Name","value", &library).is_err());
        assert!(create_macro("Key chord", "Shortcut", "Ctrl+Shift+P", &library).is_ok());
        assert!(create_macro("Clicks", "Bad click", "drag", &library).is_err());
    }
    #[test] fn macro_creation_and_assignment_resolve_to_the_saved_macro_name() {
        let library = MacroLibrary::default();
        let created = create_macro("Key chord", "Open search", "Ctrl+P", &library).unwrap();
        let mut populated = library;
        populated.macros.push(created.clone());
        populated.validate().unwrap();
        let mut profile = Profile::default_profile();
        profile.set_action("key-3-1", 1, KeyAction::MacroReference { macro_id: created.macro_id.clone() });
        profile.validate().unwrap();
        assert_eq!(display_action_with_library(profile.action("key-3-1", 1), &populated), "Macro · Open search");
        assert_eq!(create_macro_identity("Open search", &populated).unwrap_err(), "A macro with that name already exists.");
    }
    #[test] fn macro_editor_persists_recorded_timing_loops_and_edits_by_stable_id() {
        let library = MacroLibrary::default();
        let steps = vec![
            MacroStep { delay_ms: 84, action: KeyAction::KeyChord { keys: vec!["CTRL".into(), "P".into()] } },
            MacroStep { delay_ms: 0, action: KeyAction::MouseAction { action: MouseActionKind::LeftClick } },
        ];
        let (created_library, created) = update_macro_draft("Search", None, &steps, "Recorded delay", "1", "Loop count", "3", &library).unwrap();
        assert_eq!(created.timing, MacroTiming::Recorded);
        assert_eq!(created.loop_mode, MacroLoopMode::Count(3));
        assert_eq!(created.steps[0].delay_ms, 84);
        let edited_steps = vec![MacroStep { delay_ms: 0, action: KeyAction::KeyChord { keys: vec!["ESC".into()] } }];
        let (edited_library, edited) = update_macro_draft("Search updated", Some(&created.macro_id), &edited_steps, "Fixed delay", "50", "Until stop", "1", &created_library).unwrap();
        assert_eq!(edited.macro_id, created.macro_id);
        assert_eq!(edited.loop_mode, MacroLoopMode::UntilStop);
        assert_eq!(edited.fixed_delay_ms, 50);
        assert_eq!(edited_library.macros.len(), 1);
        assert_eq!(update_macro_draft("Bad", None, &steps, "Fixed delay", "1", "Loop count", "0", &library).unwrap_err(), "Loop count must be between 1 and 10,000");
    }
    #[test] fn application_location_preserves_path_or_identifier_as_typed_data() {
        assert_eq!(application_location(r"C:\Apps\editor.exe"), (Some(r"C:\Apps\editor.exe".into()), None));
        assert_eq!(application_location("com.example.Editor"), (None, Some("com.example.Editor".into())));
    }
    #[test] fn empty_port_list_has_an_honest_disabled_picker_label() {
        assert_eq!(port_choices(&[]), vec![slint::SharedString::from("No serial ports found")]);
    }
    #[test] fn checked_in_profile_uses_the_canonical_keymap_schema() {
        let sample: Profile = serde_json::from_str(include_str!("../profiles/default.json")).unwrap();
        assert_eq!(sample.validate(), Ok(()));
        assert_eq!(sample.assignments.len(), 36);
    }
}
