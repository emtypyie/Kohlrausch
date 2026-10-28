use crate::config::{
    self, KeyAction, LayerConfig, MacroDefinition, MacroLibrary, MacroLoopMode, MacroStep,
    MacroTiming, MouseActionKind, Profile,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;
use tauri::State;

#[derive(Default)]
pub struct AppState {
    pub write_lock: Mutex<()>,
}

pub struct StartupRoute {
    pub section: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileSummary {
    profile_id: String,
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    active_profile: Profile,
    profiles: Vec<ProfileSummary>,
    macros: MacroLibrary,
    warning: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    workspace: Workspace,
    startup_section: Option<String>,
    ports: Vec<String>,
    port_status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFile {
    name: String,
    kind: String,
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSnapshot {
    workspace: Workspace,
    files: Vec<ConfigFile>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    workspace: Workspace,
    imported_profile_id: String,
    imported_kind: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PortScan {
    ports: Vec<String>,
    status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroSaveResult {
    workspace: Workspace,
    macro_definition: MacroDefinition,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentInput {
    profile_id: String,
    key_id: String,
    layer: u8,
    kind: String,
    #[serde(default)]
    value: String,
    #[serde(default)]
    extra: String,
    #[serde(default = "default_arguments")]
    arguments: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSaveInput {
    profile_id: String,
    file_name: String,
    source: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroDraft {
    name: String,
    #[serde(default)]
    macro_id: Option<String>,
    steps: Vec<MacroStepInput>,
    timing: String,
    #[serde(default)]
    fixed_delay_ms: u32,
    loop_mode: String,
    #[serde(default)]
    loop_count: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroStepInput {
    mode: String,
    value: String,
    #[serde(default)]
    delay_ms: u32,
}

fn default_arguments() -> String {
    "[]".to_owned()
}

fn combine_warning(first: Option<String>, second: Option<String>) -> Option<String> {
    match (first, second) {
        (Some(a), Some(b)) => Some(format!("{a} {b}")),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn make_workspace(
    active: Profile,
    mut profiles: Vec<Profile>,
    macros: MacroLibrary,
    warning: Option<String>,
) -> Workspace {
    if !profiles
        .iter()
        .any(|profile| profile.profile_id == active.profile_id)
    {
        profiles.push(active.clone());
        profiles.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }
    Workspace {
        active_profile: active,
        profiles: profiles
            .into_iter()
            .map(|profile| ProfileSummary {
                profile_id: profile.profile_id,
                name: profile.name,
            })
            .collect(),
        macros,
        warning,
    }
}

fn macros_with_warning() -> (MacroLibrary, Option<String>) {
    match config::load_macro_library() {
        Ok(library) => (library, None),
        Err(error) => (
            MacroLibrary::default(),
            Some(format!("Macro library warning: {error}")),
        ),
    }
}

fn load_workspace() -> Result<Workspace, String> {
    let profiles = config::load_profiles()?;
    let mut warning = None;
    let active = match config::load_active_profile() {
        Ok(profile) => profile,
        Err(error) => {
            warning = Some(format!("Active profile warning: {error}"));
            profiles
                .first()
                .cloned()
                .ok_or_else(|| "No profile is available".to_owned())?
        }
    };
    let (macros, macro_warning) = macros_with_warning();
    Ok(make_workspace(
        active,
        profiles,
        macros,
        combine_warning(warning, macro_warning),
    ))
}

fn active_workspace(profiles: Vec<Profile>) -> Result<Workspace, String> {
    let active = config::load_active_profile()?;
    let (macros, macro_warning) = macros_with_warning();
    Ok(make_workspace(active, profiles, macros, macro_warning))
}

fn config_files_for(profile: &Profile) -> Result<Vec<ConfigFile>, String> {
    let directory = config::profiles_dir().join(&profile.profile_id);
    let mut files = Vec::new();

    let profile_path = directory.join("profile.json");
    let profile_source = if profile_path.is_file() {
        fs::read_to_string(&profile_path)
            .map_err(|error| format!("Cannot read {}: {error}", profile_path.display()))?
    } else {
        serde_json::to_string_pretty(profile).map_err(|error| error.to_string())?
    };
    files.push(ConfigFile {
        name: "profile.json".to_owned(),
        kind: "profile".to_owned(),
        content: profile_source,
    });

    for layer in 1..=4 {
        let path = directory.join(format!("layer-{layer}.json"));
        let source = if path.is_file() {
            fs::read_to_string(&path)
                .map_err(|error| format!("Cannot read {}: {error}", path.display()))?
        } else {
            serde_json::to_string_pretty(&LayerConfig::from_profile(profile, layer)?)
                .map_err(|error| error.to_string())?
        };
        files.push(ConfigFile {
            name: format!("layer-{layer}.json"),
            kind: "layer".to_owned(),
            content: source,
        });
    }

    Ok(files)
}

fn scan_ports_inner() -> (Vec<String>, String) {
    let mut ports = serialport::available_ports()
        .unwrap_or_default()
        .into_iter()
        .map(|port| port.port_name)
        .collect::<Vec<_>>();
    ports.sort();
    let status = if ports.is_empty() {
        "No serial ports found · no verified device handshake".to_owned()
    } else {
        format!(
            "{} serial port(s) detected · no verified device handshake",
            ports.len()
        )
    };
    (ports, status)
}

fn application_location(value: &str) -> (Option<String>, Option<String>) {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return (None, None);
    }
    if trimmed.contains('\\')
        || trimmed.starts_with('/')
        || trimmed.ends_with(".exe")
        || trimmed.ends_with(".app")
    {
        (Some(trimmed.to_owned()), None)
    } else {
        (None, Some(trimmed.to_owned()))
    }
}

fn mouse_action(value: &str) -> Option<MouseActionKind> {
    match value {
        "Left click" => Some(MouseActionKind::LeftClick),
        "Right click" => Some(MouseActionKind::RightClick),
        "Double click" => Some(MouseActionKind::DoubleClick),
        "Scroll up" => Some(MouseActionKind::ScrollUp),
        "Scroll down" => Some(MouseActionKind::ScrollDown),
        _ => None,
    }
}

fn slug(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn profile_references_macro(profile: &Profile, macro_id: &str) -> bool {
    profile.assignments.iter().any(|assignment| {
        matches!(
            &assignment.action,
            KeyAction::MacroReference { macro_id: reference } if reference == macro_id
        )
    })
}

fn macro_step(input: &MacroStepInput) -> Result<MacroStep, String> {
    let value = input.value.trim();
    if value.is_empty() || value.chars().count() > 512 {
        return Err("Action details must contain 1-512 characters.".to_owned());
    }
    let action = match input.mode.as_str() {
        "Key chord" => KeyAction::KeyChord {
            keys: config::normalize_key_chord(value)?,
        },
        "Website" => KeyAction::Website {
            url: value.to_owned(),
            label: None,
        },
        "Software" => KeyAction::Application {
            name: value.to_owned(),
            executable_path: None,
            application_id: None,
            arguments: Vec::new(),
        },
        "Clicks" => KeyAction::MouseAction {
            action: mouse_action(value).ok_or_else(|| {
                "Use Left click, Right click, Double click, Scroll up, or Scroll down.".to_owned()
            })?,
        },
        _ => return Err("Choose one of the supported macro modes.".to_owned()),
    };
    let step = MacroStep {
        delay_ms: input.delay_ms,
        action,
    };
    MacroDefinition {
        schema_version: 1,
        macro_id: "draft".to_owned(),
        name: "Draft".to_owned(),
        steps: vec![step.clone()],
        timing: MacroTiming::Fixed,
        fixed_delay_ms: 0,
        loop_mode: MacroLoopMode::Once,
    }
    .validate()?;
    Ok(step)
}

fn macro_identity(name: &str, library: &MacroLibrary) -> Result<MacroDefinition, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err("Macro name must contain 1-64 characters.".to_owned());
    }
    if library
        .macros
        .iter()
        .any(|item| item.name.eq_ignore_ascii_case(name))
    {
        return Err("A macro with that name already exists.".to_owned());
    }
    let base_id = slug(name);
    let base_id = if base_id.is_empty() {
        "macro".to_owned()
    } else {
        base_id
    };
    let mut macro_id = base_id.clone();
    let mut suffix = 2;
    while library.macros.iter().any(|item| item.macro_id == macro_id) {
        macro_id = format!("{base_id}-{suffix}");
        suffix += 1;
    }
    Ok(MacroDefinition {
        schema_version: 1,
        macro_id,
        name: name.to_owned(),
        steps: Vec::new(),
        timing: MacroTiming::Fixed,
        fixed_delay_ms: 0,
        loop_mode: MacroLoopMode::Once,
    })
}

fn update_macro_draft(
    draft: &MacroDraft,
    library: &MacroLibrary,
) -> Result<(MacroLibrary, MacroDefinition), String> {
    if draft.steps.is_empty() {
        return Err("Add at least one event before saving the macro".to_owned());
    }
    let timing = match draft.timing.as_str() {
        "Recorded delay" => MacroTiming::Recorded,
        "Fixed delay" => MacroTiming::Fixed,
        _ => return Err("Choose recorded or fixed delay".to_owned()),
    };
    let loop_mode = match draft.loop_mode.as_str() {
        "Once" => MacroLoopMode::Once,
        "Until stop" => MacroLoopMode::UntilStop,
        "Until any click" => MacroLoopMode::UntilAnyClick,
        "Loop count" => MacroLoopMode::Count(draft.loop_count),
        _ => return Err("Choose a supported loop setting".to_owned()),
    };
    let steps = draft
        .steps
        .iter()
        .map(macro_step)
        .collect::<Result<Vec<_>, _>>()?;

    let mut candidate = library.clone();
    let mut definition = if let Some(id) = draft.macro_id.as_deref() {
        let position = candidate
            .macros
            .iter()
            .position(|item| item.macro_id == id)
            .ok_or_else(|| "The selected macro no longer exists".to_owned())?;
        let existing = candidate.macros.remove(position);
        if candidate
            .macros
            .iter()
            .any(|item| item.name.eq_ignore_ascii_case(draft.name.trim()))
        {
            return Err("A macro with that name already exists".to_owned());
        }
        existing
    } else {
        macro_identity(&draft.name, &candidate)?
    };
    definition.name = draft.name.trim().to_owned();
    definition.steps = steps;
    definition.timing = timing;
    definition.fixed_delay_ms = draft.fixed_delay_ms;
    definition.loop_mode = loop_mode;
    definition.validate()?;
    candidate.macros.push(definition.clone());
    candidate.validate()?;
    Ok((candidate, definition))
}

#[tauri::command]
pub async fn bootstrap(
    state: State<'_, AppState>,
    startup: State<'_, StartupRoute>,
) -> Result<Bootstrap, String> {
    let workspace = {
        let _guard = state
            .write_lock
            .lock()
            .map_err(|_| "Configuration lock is unavailable".to_owned())?;
        load_workspace()?
    };
    let (ports, port_status) = scan_ports_inner();
    Ok(Bootstrap {
        workspace,
        startup_section: startup.section.clone(),
        ports,
        port_status,
    })
}

#[tauri::command]
pub async fn get_workspace(state: State<'_, AppState>) -> Result<Workspace, String> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "Configuration lock is unavailable".to_owned())?;
    load_workspace()
}

#[tauri::command]
pub async fn scan_ports() -> Result<PortScan, String> {
    let (ports, status) = scan_ports_inner();
    Ok(PortScan { ports, status })
}

#[tauri::command]
pub async fn create_profile(
    state: State<'_, AppState>,
    name: String,
) -> Result<Workspace, String> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "Configuration lock is unavailable".to_owned())?;
    let name = name.trim().to_owned();
    if name.is_empty() {
        return Err("Enter a profile name using letters and numbers.".to_owned());
    }
    let profile_id = slug(&name);
    if profile_id.is_empty() {
        return Err("Enter a profile name using letters and numbers.".to_owned());
    }
    let profiles = config::load_profiles()?;
    if profiles
        .iter()
        .any(|profile| profile.profile_id == profile_id)
    {
        return Err("A profile with that name already exists.".to_owned());
    }
    let mut profile = Profile::default_profile();
    profile.profile_id = profile_id;
    profile.name = name;
    profile.validate()?;
    config::save_profile(&profile)?;
    config::save_active_profile(&profile.profile_id)?;
    let (macros, macro_warning) = macros_with_warning();
    Ok(make_workspace(profile, profiles, macros, macro_warning))
}

#[tauri::command]
pub async fn switch_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Workspace, String> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "Configuration lock is unavailable".to_owned())?;
    let profiles = config::load_profiles()?;
    let profile = profiles
        .iter()
        .find(|profile| profile.profile_id == profile_id)
        .cloned()
        .ok_or_else(|| "Profile no longer exists".to_owned())?;
    config::save_active_profile(&profile.profile_id)?;
    let (macros, macro_warning) = macros_with_warning();
    Ok(make_workspace(profile, profiles, macros, macro_warning))
}

#[tauri::command]
pub async fn archive_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Workspace, String> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "Configuration lock is unavailable".to_owned())?;
    let active_before = config::load_active_profile()?;
    config::archive_profile(&profile_id)?;
    let profiles = config::load_profiles()?;
    let active = if active_before.profile_id == profile_id
        || !profiles
            .iter()
            .any(|profile| profile.profile_id == active_before.profile_id)
    {
        let fallback = profiles
            .first()
            .cloned()
            .ok_or_else(|| "No profile is available".to_owned())?;
        config::save_active_profile(&fallback.profile_id)?;
        fallback
    } else {
        active_before
    };
    let (macros, macro_warning) = macros_with_warning();
    Ok(make_workspace(active, profiles, macros, macro_warning))
}

#[tauri::command]
pub async fn save_assignment(
    state: State<'_, AppState>,
    input: AssignmentInput,
) -> Result<Workspace, String> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "Configuration lock is unavailable".to_owned())?;
    if !config::KEY_IDS.contains(&input.key_id.as_str()) {
        return Err("Unknown physical key".to_owned());
    }
    if !(1..=4).contains(&input.layer) {
        return Err("Layer must be between 1 and 4".to_owned());
    }
    let active = config::load_active_profile()?;
    if active.profile_id != input.profile_id {
        return Err("Assignments can only be saved to the active profile".to_owned());
    }
    let (macros, _) = macros_with_warning();
    let action = match input.kind.as_str() {
        "none" => KeyAction::None,
        "keyChord" => {
            if input.value.trim().is_empty() {
                KeyAction::None
            } else {
                KeyAction::KeyChord {
                    keys: config::normalize_key_chord(&input.value)?,
                }
            }
        }
        "macro" => {
            let macro_id = input.value.trim();
            if macros.find(macro_id).is_none() {
                return Err("Select a saved macro. The referenced macro is missing.".to_owned());
            }
            KeyAction::MacroReference {
                macro_id: macro_id.to_owned(),
            }
        }
        "website" => KeyAction::Website {
            url: input.value.clone(),
            label: if input.extra.trim().is_empty() {
                None
            } else {
                Some(input.extra.trim().to_owned())
            },
        },
        "application" => {
            let arguments = serde_json::from_str::<Vec<String>>(&input.arguments).map_err(|_| {
                "Arguments must be a JSON array of strings, such as [\"--new-window\"]".to_owned()
            })?;
            let (executable_path, application_id) = application_location(&input.extra);
            KeyAction::Application {
                name: input.value.trim().to_owned(),
                executable_path,
                application_id,
                arguments,
            }
        }
        "mouseAction" => KeyAction::MouseAction {
            action: mouse_action(&input.value)
                .ok_or_else(|| "Choose a supported click or scroll action".to_owned())?,
        },
        _ => return Err("Choose an action type.".to_owned()),
    };
    let mut candidate = active;
    candidate.set_action(&input.key_id, input.layer, action);
    candidate.validate()?;
    config::save_profile(&candidate)?;
    let profiles = config::load_profiles()?;
    let (macros, macro_warning) = macros_with_warning();
    Ok(make_workspace(candidate, profiles, macros, macro_warning))
}

#[tauri::command]
pub async fn config_files(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<Vec<ConfigFile>, String> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "Configuration lock is unavailable".to_owned())?;
    let profiles = config::load_profiles()?;
    let profile = profiles
        .iter()
        .find(|profile| profile.profile_id == profile_id)
        .ok_or_else(|| "Profile no longer exists".to_owned())?;
    config_files_for(profile)
}

#[tauri::command]
pub async fn save_config(
    state: State<'_, AppState>,
    input: ConfigSaveInput,
) -> Result<ConfigSnapshot, String> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "Configuration lock is unavailable".to_owned())?;
    let active = config::load_active_profile()?;
    if active.profile_id != input.profile_id {
        return Err("Configuration can only be saved to the active profile".to_owned());
    }
    let candidate = if input.file_name == "profile.json" {
        let profile: Profile = serde_json::from_str(&input.source)
            .map_err(|error| format!("Invalid profile JSON: {error}"))?;
        if profile.profile_id != active.profile_id {
            return Err("The profile ID cannot be changed from Config Studio".to_owned());
        }
        profile.validate()?;
        profile
    } else if input.file_name.starts_with("layer-") && input.file_name.ends_with(".json") {
        let layer_number = input
            .file_name
            .trim_start_matches("layer-")
            .trim_end_matches(".json")
            .parse::<u8>()
            .map_err(|_| "Unknown configuration file".to_owned())?;
        if !(1..=4).contains(&layer_number) {
            return Err("Layer must be between 1 and 4".to_owned());
        }
        let layer: LayerConfig = serde_json::from_str(&input.source)
            .map_err(|error| format!("Invalid layer JSON: {error}"))?;
        if layer.layer != layer_number {
            return Err("Layer number does not match the file being edited".to_owned());
        }
        layer.apply_to(&active)?
    } else {
        return Err("Unknown configuration file".to_owned());
    };
    config::save_profile(&candidate)?;
    let profiles = config::load_profiles()?;
    let (macros, macro_warning) = macros_with_warning();
    let workspace = make_workspace(candidate.clone(), profiles, macros, macro_warning);
    let files = config_files_for(&candidate)?;
    Ok(ConfigSnapshot { workspace, files })
}

#[tauri::command]
pub async fn import_config(
    state: State<'_, AppState>,
    folder: bool,
) -> Result<Option<ImportOutcome>, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        if folder {
            rfd::FileDialog::new().pick_folder()
        } else {
            rfd::FileDialog::new()
                .add_filter("Kohlrausch configuration", &["json"])
                .pick_file()
        }
    })
    .await
    .map_err(|error| format!("File dialog failed: {error}"))?;
    let Some(path) = picked else {
        return Ok(None);
    };

    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "Configuration lock is unavailable".to_owned())?;
    let active = config::load_active_profile()?;
    let (candidate, kind) = if folder {
        (config::load_profile_folder(&path)?, "profile")
    } else {
        let text = fs::read_to_string(&path)
            .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
        match serde_json::from_str::<LayerConfig>(&text) {
            Ok(layer) => (layer.apply_to(&active)?, "layer"),
            Err(_) => (config::load_external_profile_file(&path)?, "profile"),
        }
    };
    candidate.validate()?;
    config::save_profile(&candidate)?;
    let profiles = config::load_profiles()?;
    let workspace = active_workspace(profiles)?;
    Ok(Some(ImportOutcome {
        workspace,
        imported_profile_id: candidate.profile_id,
        imported_kind: kind.to_owned(),
    }))
}

#[tauri::command]
pub async fn save_macro(
    state: State<'_, AppState>,
    draft: MacroDraft,
) -> Result<MacroSaveResult, String> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "Configuration lock is unavailable".to_owned())?;
    let library = config::load_macro_library()?;
    let (candidate, definition) = update_macro_draft(&draft, &library)?;
    config::save_macro_library(&candidate)?;
    let workspace = load_workspace()?;
    Ok(MacroSaveResult {
        workspace,
        macro_definition: definition,
    })
}

#[tauri::command]
pub async fn delete_macro(
    state: State<'_, AppState>,
    macro_id: String,
) -> Result<Workspace, String> {
    let _guard = state
        .write_lock
        .lock()
        .map_err(|_| "Configuration lock is unavailable".to_owned())?;
    let mut library = config::load_macro_library()?;
    let position = library
        .macros
        .iter()
        .position(|item| item.macro_id == macro_id)
        .ok_or_else(|| "Macro no longer exists".to_owned())?;
    let profiles = config::load_profiles()?;
    if profiles
        .iter()
        .any(|profile| profile_references_macro(profile, &macro_id))
    {
        return Err(
            "This macro is assigned in a profile. Remove those assignments before deleting it."
                .to_owned(),
        );
    }
    library.macros.remove(position);
    config::save_macro_library(&library)?;
    active_workspace(profiles)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_location_preserves_path_or_identifier_as_typed_data() {
        assert_eq!(
            application_location(r"C:\Apps\editor.exe"),
            (Some(r"C:\Apps\editor.exe".into()), None)
        );
        assert_eq!(
            application_location("com.example.Editor"),
            (None, Some("com.example.Editor".into()))
        );
    }

    #[test]
    fn slug_produces_a_stable_id() {
        assert_eq!(slug("My Work Profile"), "my-work-profile");
    }
}
