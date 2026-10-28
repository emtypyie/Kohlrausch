use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, io::Write, path::PathBuf};

pub const BOARD_ID: &str = "kohlrausch-rp2040-macropad";
pub const KEY_IDS: [&str; 9] = [
    "key-3-1", "key-2-1", "key-1-1", "key-3-2", "key-2-2", "key-1-2", "key-3-3", "key-2-3", "key-1-3",
];
static UNASSIGNED: KeyAction = KeyAction::None;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase", deny_unknown_fields)]
pub enum KeyAction {
    None,
    KeyChord { keys: Vec<String> },
    MacroReference { #[serde(rename = "macroId")] macro_id: String },
    Website { url: String, #[serde(default, skip_serializing_if = "Option::is_none")] label: Option<String> },
    Application {
        name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")] executable_path: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")] application_id: Option<String>,
        #[serde(default)] arguments: Vec<String>,
    },
    MouseAction { action: MouseActionKind },
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MouseActionKind { LeftClick, RightClick, DoubleClick, ScrollUp, ScrollDown }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Assignment { pub key_id: String, pub layer: u8, pub action: KeyAction }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(rename = "schemaVersion")] pub schema_version: u8,
    #[serde(rename = "profileId")] pub profile_id: String,
    pub name: String,
    #[serde(rename = "boardId")] pub board_id: String,
    pub assignments: Vec<Assignment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerConfig {
    pub schema_version: u8,
    pub profile_id: String,
    pub layer: u8,
    pub assignments: Vec<LayerAssignment>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LayerAssignment { pub key_id: String, pub action: KeyAction }

impl LayerConfig {
    pub fn from_profile(profile: &Profile, layer: u8) -> Result<Self, String> {
        if !(1..=4).contains(&layer) { return Err("Layer must be between 1 and 4".into()); }
        Ok(Self { schema_version: 1, profile_id: profile.profile_id.clone(), layer, assignments: profile.assignments.iter().filter(|item| item.layer == layer).map(|item| LayerAssignment { key_id: item.key_id.clone(), action: item.action.clone() }).collect() })
    }

    pub fn apply_to(&self, profile: &Profile) -> Result<Profile, String> {
        if self.schema_version != 1 || self.profile_id != profile.profile_id || !(1..=4).contains(&self.layer) || self.assignments.len() != 9 { return Err("Layer file does not match this profile or has an invalid schema".into()); }
        let mut candidate = profile.clone();
        candidate.assignments.retain(|item| item.layer != self.layer);
        for item in &self.assignments {
            candidate.assignments.push(Assignment { key_id: item.key_id.clone(), layer: self.layer, action: item.action.clone() });
        }
        candidate.validate()?;
        Ok(candidate)
    }
}

impl Profile {
    pub fn default_profile() -> Self {
        let assignments = (1..=4).flat_map(|layer| KEY_IDS.iter().map(move |key_id| Assignment {
            key_id: (*key_id).to_owned(), layer, action: KeyAction::None,
        })).collect();
        Self { schema_version: 2, profile_id: "default".into(), name: "Default".into(), board_id: BOARD_ID.into(), assignments }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 2 { return Err("Unsupported profile schema version".into()); }
        if self.board_id != BOARD_ID { return Err("Profile is for a different board".into()); }
        if self.profile_id.trim().is_empty() || self.name.trim().is_empty() { return Err("Profile ID and name are required".into()); }
        if !valid_stable_id(&self.profile_id) { return Err("Profile ID may contain only letters, numbers, hyphens and underscores".into()); }
        if self.name.chars().count() > 64 { return Err("Profile name must be 64 characters or fewer".into()); }
        if self.assignments.len() != 36 { return Err("A profile must define exactly 9 keys on each of 4 layers".into()); }
        let mut seen = HashSet::new();
        for assignment in &self.assignments {
            if !(1..=4).contains(&assignment.layer) || !KEY_IDS.contains(&assignment.key_id.as_str()) { return Err("Assignment references an invalid layer or physical key".into()); }
            if !seen.insert((assignment.layer, assignment.key_id.as_str())) { return Err("Duplicate key assignment".into()); }
            match &assignment.action {
                KeyAction::KeyChord { keys } => validate_key_chord(keys)?,
                KeyAction::MacroReference { macro_id } if !valid_stable_id(macro_id) => return Err("Macro reference has an invalid ID".into()),
                KeyAction::MacroReference { .. } => {}
                KeyAction::Website { url, label } => {
                    if !valid_http_url(url) { return Err("Website action requires a valid HTTP or HTTPS URL".into()); }
                    if label.as_ref().is_some_and(|value| value.chars().count() > 80) { return Err("Website label must be 80 characters or fewer".into()); }
                }
                KeyAction::Application { name, executable_path, application_id, arguments } => {
                    if name.trim().is_empty() { return Err("Application action requires an application name".into()); }
                    if name.chars().count() > 100 { return Err("Application name must be 100 characters or fewer".into()); }
                    if executable_path.as_ref().is_some_and(|value| value.contains('\0') || value.chars().count() > 512) { return Err("Executable path is invalid or too long".into()); }
                    if application_id.as_ref().is_some_and(|value| value.trim().is_empty() || value.chars().count() > 200) { return Err("Application ID is invalid or too long".into()); }
                    if arguments.len() > 32 || arguments.iter().any(|value| value.contains('\0') || value.chars().count() > 256) { return Err("Application arguments exceed the supported limit".into()); }
                }
                KeyAction::None | KeyAction::MouseAction { .. } => {}
            }
        }
        Ok(())
    }

    pub fn action(&self, key_id: &str, layer: u8) -> &KeyAction {
        self.assignments.iter().find(|a| a.key_id == key_id && a.layer == layer).map(|a| &a.action).unwrap_or(&UNASSIGNED)
    }

    pub fn set_action(&mut self, key_id: &str, layer: u8, action: KeyAction) {
        if let Some(item) = self.assignments.iter_mut().find(|a| a.key_id == key_id && a.layer == layer) { item.action = action; }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MacroDefinition {
    pub schema_version: u8,
    pub macro_id: String,
    pub name: String,
    pub steps: Vec<MacroStep>,
    #[serde(default)] pub timing: MacroTiming,
    #[serde(default)] pub fixed_delay_ms: u32,
    #[serde(default)] pub loop_mode: MacroLoopMode,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MacroTiming { Recorded, #[default] Fixed }

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "mode", content = "count", rename_all = "camelCase")]
pub enum MacroLoopMode { #[default] Once, UntilStop, UntilAnyClick, Count(u16) }

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MacroStep {
    pub delay_ms: u32,
    pub action: KeyAction,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MacroLibrary {
    pub schema_version: u8,
    pub macros: Vec<MacroDefinition>,
}

impl Default for MacroLibrary {
    fn default() -> Self { Self { schema_version: 1, macros: Vec::new() } }
}

impl MacroDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 { return Err("Unsupported macro schema version".into()); }
        if !valid_stable_id(&self.macro_id) { return Err("Macro ID must contain only letters, numbers, hyphens and underscores".into()); }
        if self.name.trim().is_empty() || self.name.chars().count() > 64 { return Err("Macro name must contain 1–64 characters".into()); }
        if self.steps.is_empty() { return Err("Add at least one step to the macro".into()); }
        if self.steps.len() > 64 { return Err("A macro can contain at most 64 steps".into()); }
        if self.fixed_delay_ms > 600_000 { return Err("Fixed delay cannot exceed 10 minutes".into()); }
        if matches!(self.loop_mode, MacroLoopMode::Count(0) | MacroLoopMode::Count(10_001..=u16::MAX)) { return Err("Loop count must be between 1 and 10,000".into()); }
        for step in &self.steps {
            if step.delay_ms > 600_000 { return Err("Step delay cannot exceed 10 minutes".into()); }
            validate_macro_step_action(&step.action)?;
        }
        Ok(())
    }
}

impl MacroLibrary {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1 { return Err("Unsupported macro library schema version".into()); }
        let mut ids = HashSet::new();
        let mut names = HashSet::new();
        for item in &self.macros {
            item.validate()?;
            if !ids.insert(item.macro_id.as_str()) { return Err("Duplicate macro ID".into()); }
            if !names.insert(item.name.to_lowercase()) { return Err("Macro names must be unique".into()); }
        }
        Ok(())
    }

    pub fn find(&self, macro_id: &str) -> Option<&MacroDefinition> {
        self.macros.iter().find(|item| item.macro_id == macro_id)
    }
}

fn validate_macro_step_action(action: &KeyAction) -> Result<(), String> {
    match action {
        KeyAction::None | KeyAction::MacroReference { .. } => Err("A macro step must be a concrete action, not an empty or nested macro reference".into()),
        KeyAction::KeyChord { keys } => validate_key_chord(keys),
        KeyAction::Website { url, label } => {
            if !valid_http_url(url) { return Err("Website step requires a valid HTTP or HTTPS URL".into()); }
            if label.as_ref().is_some_and(|value| value.chars().count() > 80) { return Err("Website label must be 80 characters or fewer".into()); }
            Ok(())
        }
        KeyAction::Application { name, executable_path, application_id, arguments } => {
            if name.trim().is_empty() { return Err("Application step requires an application name".into()); }
            if executable_path.as_ref().is_some_and(|value| value.contains('\0') || value.chars().count() > 512) { return Err("Executable path is invalid or too long".into()); }
            if application_id.as_ref().is_some_and(|value| value.trim().is_empty() || value.chars().count() > 200) { return Err("Application ID is invalid or too long".into()); }
            if arguments.len() > 32 || arguments.iter().any(|value| value.contains('\0') || value.chars().count() > 256) { return Err("Application arguments exceed the supported limit".into()); }
            Ok(())
        }
        KeyAction::MouseAction { .. } => Ok(()),
    }
}

pub fn normalize_key_chord(input: &str) -> Result<Vec<String>, String> {
    if input.trim() == "+" { return Ok(vec!["PLUS".to_owned()]); }
    let raw_tokens = input.split('+').collect::<Vec<_>>();
    let mut tokens = Vec::with_capacity(raw_tokens.len());
    for (index, token) in raw_tokens.iter().enumerate() {
        if token.trim().is_empty() {
            if raw_tokens.len() == 1 && input.trim().is_empty() { return Err("Capture or enter a key or shortcut before saving".into()); }
            if index + 1 == raw_tokens.len() { tokens.push("SPACE"); }
            else { return Err("Shortcut contains an empty key".into()); }
        } else { tokens.push(token.trim()); }
    }
    let mut normalized = Vec::with_capacity(tokens.len());
    for token in tokens {
        let upper = token.to_ascii_uppercase();
        let value = match upper.as_str() {
            "CTRL" | "CONTROL" | "CMD" | "COMMAND" => "CTRL".to_owned(),
            "SHIFT" => "SHIFT".to_owned(),
            "ALT" | "OPTION" => "ALT".to_owned(),
            "META" | "WIN" | "WINDOWS" => "META".to_owned(),
            "ESCAPE" => "ESC".to_owned(),
            "RETURN" => "ENTER".to_owned(),
            "ARROWUP" => "UP".to_owned(),
            "ARROWDOWN" => "DOWN".to_owned(),
            "ARROWLEFT" => "LEFT".to_owned(),
            "ARROWRIGHT" => "RIGHT".to_owned(),
            "+" => "PLUS".to_owned(),
            " " => "SPACE".to_owned(),
            _ if is_supported_key(&upper) => upper,
            _ => return Err(format!("Unsupported key: {token}")),
        };
        if normalized.contains(&value) { return Err(format!("Duplicate key: {token}")); }
        normalized.push(value);
    }
    validate_key_chord(&normalized)?;
    Ok(normalized)
}

fn validate_key_chord(keys: &[String]) -> Result<(), String> {
    if keys.is_empty() { return Err("Capture or enter a key or shortcut before saving".into()); }
    let mut base_keys = 0;
    let mut seen = HashSet::new();
    for key in keys {
        let upper = key.to_ascii_uppercase();
        if !seen.insert(upper.clone()) { return Err(format!("Duplicate key: {key}")); }
        if matches!(upper.as_str(), "CTRL" | "SHIFT" | "ALT" | "META") { continue; }
        if !is_supported_key(&upper) { return Err(format!("Unsupported key: {key}")); }
        base_keys += 1;
    }
    if base_keys != 1 { return Err("A shortcut must contain exactly one non-modifier key".into()); }
    Ok(())
}

fn is_supported_key(key: &str) -> bool {
    matches!(key, "SPACE" | "ENTER" | "ESC" | "TAB" | "BACKSPACE" | "DELETE" | "INSERT" | "HOME" | "END" | "PAGEUP" | "PAGEDOWN" | "UP" | "DOWN" | "LEFT" | "RIGHT" | "PRINTSCREEN" | "PAUSE")
        || matches!(key, "PLUS" | "MINUS" | "EQUAL" | "COMMA" | "PERIOD" | "SLASH" | "SEMICOLON" | "APOSTROPHE" | "LBRACKET" | "RBRACKET" | "BACKSLASH" | "GRAVE" | "TILDE" | "BANG" | "AT" | "HASH" | "DOLLAR" | "PERCENT" | "CARET" | "AMPERSAND" | "ASTERISK" | "LPAREN" | "RPAREN" | "UNDERSCORE" | "LBRACE" | "RBRACE" | "PIPE" | "COLON" | "QUOTE" | "LESS" | "GREATER" | "QUESTION")
        || (key.len() == 1 && key.as_bytes()[0].is_ascii_alphanumeric())
        || (key.len() == 1 && "-=[]\\;',./`~!@#$%^&*()_{}|:\"<>?".contains(key))
        || key.strip_prefix('F').is_some_and(|number| number.parse::<u8>().is_ok_and(|number| (1..=24).contains(&number)))
}

fn valid_stable_id(value: &str) -> bool {
    !value.is_empty() && value.chars().all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

pub fn profiles_dir() -> PathBuf {
    // Prefer user-writable storage under APPDATA/LOCALAPPDATA; never write beside executable.
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return PathBuf::from(appdata).join("Kohlrausch").join("profiles");
    }
    if let Some(localappdata) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(localappdata).join("Kohlrausch").join("profiles");
    }
    if let Some(userprofile) = std::env::var_os("USERPROFILE") {
        return PathBuf::from(userprofile).join("AppData").join("Roaming").join("Kohlrausch").join("profiles");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".config").join("kohlrausch").join("profiles");
    }
    // Last resort: use OS temp dir to avoid writing next to executable
    std::env::temp_dir().join("Kohlrausch").join("profiles")
}

pub fn load_or_create_default() -> Result<Profile, String> {
    let dir = profiles_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create profile directory: {e}"))?;
    let path = dir.join("default.json");
    if path.exists() {
        let layered = dir.join("default");
        if layered.join("profile.json").is_file() { return load_profile_folder(&layered); }
        return load_profile_file(&path);
    }
    let profile = Profile::default_profile();
    save_profile(&profile)?;
    Ok(profile)
}

pub fn load_active_profile() -> Result<Profile, String> {
    let dir = profiles_dir();
    let base = dir.parent().unwrap_or(&std::env::temp_dir().join("Kohlrausch")).to_path_buf();
    let marker = base.join("active-profile");
    if marker.is_file() {
        if let Ok(profile_id) = fs::read_to_string(&marker) {
            let profile_id = profile_id.trim();
            if valid_stable_id(profile_id) {
                if let Some(profile) = load_profiles()?.into_iter().find(|profile| profile.profile_id == profile_id) {
                    return Ok(profile);
                }
            }
        }
    }
    load_or_create_default()
}

pub fn save_active_profile(profile_id: &str) -> Result<(), String> {
    if !valid_stable_id(profile_id) { return Err("Invalid profile ID".into()); }
    if !load_profiles()?.iter().any(|profile| profile.profile_id == profile_id) {
        return Err("Profile no longer exists".into());
    }
    let base = profiles_dir().parent().unwrap_or(&std::env::temp_dir().join("Kohlrausch")).to_path_buf();
    std::fs::create_dir_all(&base).map_err(|error| format!("Cannot create configuration directory: {error}"))?;
    save_json_atomic(&base.join("active-profile"), &base, profile_id)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProfileV1 {
    schema_version: u8,
    profile_id: String,
    name: String,
    board_id: String,
    assignments: Vec<AssignmentV1>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AssignmentV1 { key_id: String, layer: u8, action: KeyActionV1 }

#[derive(Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum KeyActionV1 {
    #[serde(rename = "none")] None,
    #[serde(rename = "keystroke")] Keystroke { keys: Vec<String> },
    #[serde(rename = "website")] Website { url: String },
    #[serde(rename = "software")] Software { application: String },
}

fn load_profile_file(path: &std::path::Path) -> Result<Profile, String> {
    let data = fs::read_to_string(path).map_err(|error| format!("Cannot read profile {}: {error}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&data).map_err(|error| format!("Profile JSON is invalid: {error}"))?;
    let version = value.get("schemaVersion").and_then(serde_json::Value::as_u64).ok_or_else(|| "Profile has no valid schemaVersion".to_owned())?;
    let (profile, migrated) = match version {
        1 => {
            let legacy: ProfileV1 = serde_json::from_value(value).map_err(|error| format!("Profile v1 JSON is invalid: {error}"))?;
            if legacy.schema_version != 1 { return Err("Unsupported profile schema version".into()); }
            let assignments = legacy.assignments.into_iter().map(|assignment| Assignment {
                key_id: assignment.key_id,
                layer: assignment.layer,
                action: match assignment.action {
                    KeyActionV1::None => KeyAction::None,
                    KeyActionV1::Keystroke { keys } => KeyAction::KeyChord { keys },
                    KeyActionV1::Website { url } => KeyAction::Website { url, label: None },
                    KeyActionV1::Software { application } => KeyAction::Application { name: application, executable_path: None, application_id: None, arguments: Vec::new() },
                },
            }).collect();
            (Profile { schema_version: 2, profile_id: legacy.profile_id, name: legacy.name, board_id: legacy.board_id, assignments }, true)
        }
        2 => (serde_json::from_value(value).map_err(|error| format!("Profile JSON is invalid: {error}"))?, false),
        _ => return Err(format!("Unsupported profile schema version {version}")),
    };
    profile.validate()?;
    if migrated {
        let parent = path.parent().ok_or_else(|| "Profile path has no parent directory".to_owned())?;
        save_profile_in_dir(&profile, parent)?;
    }
    Ok(profile)
}

pub fn load_external_profile_file(path: &std::path::Path) -> Result<Profile, String> { load_profile_file(path) }

pub fn load_profile_folder(path: &std::path::Path) -> Result<Profile, String> {
    let manifest = path.join("profile.json");
    let profile_path = if manifest.is_file() { manifest } else {
        let mut files = fs::read_dir(path).map_err(|error| format!("Cannot read config folder: {error}"))?
            .filter_map(Result::ok).map(|entry| entry.path())
            .filter(|entry| entry.extension().is_some_and(|extension| extension == "json"))
            .collect::<Vec<_>>();
        files.sort();
        files.into_iter().find(|entry| entry.file_stem().is_some_and(|name| name == "profile")).ok_or_else(|| "Folder must contain profile.json".to_owned())?
    };
    let mut profile = load_profile_file(&profile_path)?;
    for layer in 1..=4 {
        let layer_path = path.join(format!("layer-{layer}.json"));
        if layer_path.is_file() {
            let data = fs::read_to_string(&layer_path).map_err(|error| format!("Cannot read {}: {error}", layer_path.display()))?;
            let layer: LayerConfig = serde_json::from_str(&data).map_err(|error| format!("Invalid layer file {}: {error}", layer_path.display()))?;
            profile = layer.apply_to(&profile)?;
        }
    }
    profile.validate()?;
    Ok(profile)
}

pub fn load_profiles() -> Result<Vec<Profile>, String> {
    let dir = profiles_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create profile directory: {e}"))?;
    let mut profiles = Vec::new();
    let mut seen = HashSet::new();
    let entries = fs::read_dir(&dir).map_err(|e| format!("Cannot read profile directory: {e}"))?
        .map(|entry| entry.map(|entry| entry.path())).collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("Cannot read profile entry: {error}"))?;
    for path in &entries {
        if path.is_dir() {
            let manifest = path.join("profile.json");
            if manifest.is_file() {
                let mut profile = load_profile_file(&manifest)?;
                for layer in 1..=4 {
                    let layer_path = path.join(format!("layer-{layer}.json"));
                    if layer_path.is_file() {
                        let data = fs::read_to_string(&layer_path).map_err(|error| format!("Cannot read {}: {error}", layer_path.display()))?;
                        let layer_config: LayerConfig = serde_json::from_str(&data).map_err(|error| format!("Invalid layer file {}: {error}", layer_path.display()))?;
                        profile = layer_config.apply_to(&profile)?;
                    }
                }
                profile.validate()?;
                seen.insert(profile.profile_id.clone());
                profiles.push(profile);
            }
        }
    }
    for path in &entries {
        if path.extension().is_some_and(|extension| extension == "json") {
            let profile = load_profile_file(path).map_err(|error| format!("Invalid profile {}: {error}", path.display()))?;
            if seen.insert(profile.profile_id.clone()) { profiles.push(profile); }
        }
    }
    if profiles.is_empty() { profiles.push(load_or_create_default()?); }
    for profile in profiles.clone() {
        if !dir.join(&profile.profile_id).join("profile.json").is_file() { save_profile(&profile)?; }
    }
    profiles.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(profiles)
}

pub fn archive_profile(profile_id: &str) -> Result<(), String> {
    if !valid_stable_id(profile_id) { return Err("Invalid profile ID".into()); }
    let profiles = load_profiles()?;
    if profiles.len() <= 1 { return Err("Keep at least one profile".into()); }
    if !profiles.iter().any(|profile| profile.profile_id == profile_id) {
        return Err("Profile no longer exists".into());
    }

    let dir = profiles_dir();
    let trash = dir.join(".trash");
    fs::create_dir_all(&trash).map_err(|error| format!("Cannot create profile archive: {error}"))?;
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("Cannot timestamp profile archive: {error}"))?.as_secs();
    let archive = trash.join(format!("{profile_id}-{stamp}-{}", std::process::id()));
    fs::create_dir(&archive).map_err(|error| format!("Cannot prepare profile archive: {error}"))?;

    let sources = [
        (dir.join(profile_id), archive.join(profile_id)),
        (dir.join(format!("{profile_id}.json")), archive.join(format!("{profile_id}.json"))),
        (dir.join(format!("{profile_id}.json.bak")), archive.join(format!("{profile_id}.json.bak"))),
    ];
    let mut moved = Vec::new();
    for (source, destination) in sources {
        if !source.exists() { continue; }
        if let Err(error) = fs::rename(&source, &destination) {
            for (original, archived) in moved.into_iter().rev() { let _ = fs::rename(archived, original); }
            let _ = fs::remove_dir(&archive);
            return Err(format!("Cannot archive profile files: {error}"));
        }
        moved.push((source, destination));
    }
    Ok(())
}

pub fn macro_library_path() -> PathBuf {
    profiles_dir().parent().unwrap_or_else(|| std::path::Path::new(".")).join("macros.json")
}

pub fn load_macro_library() -> Result<MacroLibrary, String> {
    let path = macro_library_path();
    if !path.exists() { return Ok(MacroLibrary::default()); }
    let text = fs::read_to_string(&path).map_err(|error| format!("Cannot read macro library: {error}"))?;
    let library: MacroLibrary = serde_json::from_str(&text).map_err(|error| format!("Macro library JSON is invalid: {error}"))?;
    library.validate()?;
    Ok(library)
}

pub fn save_macro_library(library: &MacroLibrary) -> Result<(), String> {
    library.validate()?;
    let path = macro_library_path();
    let parent = path.parent().ok_or_else(|| "Macro library path has no parent directory".to_owned())?;
    save_macro_library_at(library, &path, parent)
}

fn save_macro_library_at(library: &MacroLibrary, path: &std::path::Path, parent: &std::path::Path) -> Result<(), String> {
    library.validate()?;
    save_json_atomic(path, parent, &serde_json::to_string_pretty(library).map_err(|error| error.to_string())?)
}

fn save_json_atomic(path: &std::path::Path, dir: &std::path::Path, text: &str) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|error| format!("Cannot create configuration directory: {error}"))?;
    let file_name = path.file_name().and_then(|name| name.to_str()).ok_or_else(|| "Invalid configuration file name".to_owned())?;
    let temp = dir.join(format!(".{file_name}.{}.tmp", std::process::id()));
    let mut file = fs::File::create(&temp).map_err(|error| format!("Cannot create configuration candidate: {error}"))?;
    file.write_all(text.as_bytes()).and_then(|()| file.sync_all()).map_err(|error| format!("Cannot flush configuration candidate: {error}"))?;
    let backup = dir.join(format!("{file_name}.bak"));
    if path.exists() { fs::copy(path, &backup).map_err(|error| format!("Cannot preserve configuration backup: {error}"))?; }
    replace_file(&temp, path).map_err(|error| { let _ = fs::remove_file(&temp); format!("Cannot atomically replace configuration: {error}") })
}

pub fn save_profile(profile: &Profile) -> Result<(), String> {
    profile.validate()?;
    let dir = profiles_dir();
    save_profile_in_dir(profile, &dir)?;
    let profile_dir = dir.join(&profile.profile_id);
    fs::create_dir_all(&profile_dir).map_err(|error| format!("Cannot create profile folder: {error}"))?;
    save_json_atomic(&profile_dir.join("profile.json"), &profile_dir, &serde_json::to_string_pretty(profile).map_err(|error| error.to_string())?)?;
    for layer in 1..=4 {
        let config = LayerConfig::from_profile(profile, layer)?;
        let path = profile_dir.join(format!("layer-{layer}.json"));
        let text = serde_json::to_string_pretty(&config).map_err(|error| error.to_string())?;
        save_json_atomic(&path, &profile_dir, &text)?;
    }
    Ok(())
}

fn save_profile_in_dir(profile: &Profile, dir: &std::path::Path) -> Result<(), String> {
    fs::create_dir_all(&dir).map_err(|e| format!("Cannot create profile directory: {e}"))?;
    let path = dir.join(format!("{}.json", profile.profile_id));
    let temp = dir.join(format!(".{}.{}.tmp", profile.profile_id, std::process::id()));
    let text = serde_json::to_string_pretty(profile).map_err(|e| e.to_string())?;
    let mut file = fs::File::create(&temp).map_err(|e| format!("Cannot create profile candidate: {e}"))?;
    file.write_all(text.as_bytes()).and_then(|()| file.sync_all()).map_err(|e| format!("Cannot flush profile candidate: {e}"))?;
    let backup = dir.join(format!("{}.json.bak", profile.profile_id));
    if path.exists() { fs::copy(&path, &backup).map_err(|e| format!("Cannot preserve profile backup: {e}"))?; }
    replace_file(&temp, &path).map_err(|e| { let _ = fs::remove_file(&temp); format!("Cannot atomically replace profile: {e}") })
}

fn valid_http_url(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once("://") else { return false; };
    if !matches!(scheme, "http" | "https") || rest.trim().is_empty() || rest.chars().any(char::is_whitespace) { return false; }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    !authority.is_empty() && !authority.starts_with('.') && !authority.ends_with('.') && authority.contains('.')
}

#[cfg(windows)]
fn replace_file(source: &std::path::Path, destination: &std::path::Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH};
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination.as_os_str().encode_wide().chain(Some(0)).collect();
    let success = unsafe { MoveFileExW(source.as_ptr(), destination.as_ptr(), MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH) };
    if success == 0 { Err(std::io::Error::last_os_error()) } else { Ok(()) }
}

#[cfg(not(windows))]
fn replace_file(source: &std::path::Path, destination: &std::path::Path) -> std::io::Result<()> { fs::rename(source, destination) }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn default_profile_has_36_unique_slots() {
        let profile = Profile::default_profile();
        assert_eq!(profile.validate(), Ok(()));
        assert_eq!(profile.assignments.len(), 36);
        for layer in 1..=4 { assert_eq!(profile.assignments.iter().filter(|a| a.layer == layer).count(), 9); }
    }
    #[test] fn layer_assignments_are_independent() {
        let mut profile = Profile::default_profile();
        profile.set_action("key-3-1", 2, KeyAction::KeyChord { keys: vec!["CTRL".into(), "C".into()] });
        assert_eq!(profile.action("key-3-1", 1), &KeyAction::None);
        assert_eq!(profile.action("key-3-1", 2), &KeyAction::KeyChord { keys: vec!["CTRL".into(), "C".into()] });
    }
    #[test] fn validation_rejects_unknown_physical_keys_and_bad_urls() {
        let mut profile = Profile::default_profile();
        profile.assignments[0].key_id = "key-0-0".into();
        assert!(profile.validate().is_err());
        let mut profile = Profile::default_profile();
        profile.set_action("key-3-1", 1, KeyAction::Website { url: "file:///secret".into(), label: None });
        assert!(profile.validate().is_err());
    }
    #[test] fn profile_saves_are_recoverable_and_round_trip() {
        let dir = std::env::temp_dir().join(format!("kohlrausch-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let original = Profile::default_profile();
        save_profile_in_dir(&original, &dir).unwrap();
        let mut changed = original.clone();
        changed.name = "Edited".into();
        save_profile_in_dir(&changed, &dir).unwrap();
        let loaded: Profile = serde_json::from_str(&fs::read_to_string(dir.join("default.json")).unwrap()).unwrap();
        assert_eq!(loaded.name, "Edited");
        assert!(dir.join("default.json.bak").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn chord_website_application_and_mouse_actions_round_trip() {
        assert_eq!(normalize_key_chord("Ctrl+Shift+P").unwrap(), vec!["CTRL", "SHIFT", "P"]);
        assert_eq!(normalize_key_chord("CTRL+SPACE").unwrap(), vec!["CTRL", "SPACE"]);
        assert_eq!(normalize_key_chord("CTRL+PLUS").unwrap(), vec!["CTRL", "PLUS"]);
        assert_eq!(normalize_key_chord(".").unwrap(), vec!["."]);
        assert!(normalize_key_chord("Ctrl+NoSuchKey").is_err());
        assert!(normalize_key_chord("").is_err());
        let mut profile = Profile::default_profile();
        profile.set_action("key-3-1", 1, KeyAction::Website { url: "https://example.com/docs".into(), label: Some("Docs".into()) });
        profile.set_action("key-2-1", 1, KeyAction::Application { name: "Editor".into(), executable_path: None, application_id: Some("editor.app".into()), arguments: vec!["--new-window".into()] });
        profile.set_action("key-1-1", 1, KeyAction::MouseAction { action: MouseActionKind::DoubleClick });
        profile.validate().unwrap();
        let encoded = serde_json::to_string(&profile).unwrap();
        let decoded: Profile = serde_json::from_str(&encoded).unwrap();
        assert_eq!(profile, decoded);
    }

    #[test]
    fn macros_validate_stably_and_round_trip() {
        let mut library = MacroLibrary::default();
        library.macros.push(MacroDefinition {
            schema_version: 1,
            macro_id: "open-docs".into(),
            name: "Open docs".into(),
            steps: vec![
                MacroStep { delay_ms: 120, action: KeyAction::KeyChord { keys: vec!["CTRL".into(), "K".into()] } },
                MacroStep { delay_ms: 0, action: KeyAction::MouseAction { action: MouseActionKind::DoubleClick } },
            ],
            timing: MacroTiming::Recorded,
            fixed_delay_ms: 1,
            loop_mode: MacroLoopMode::Count(3),
        });
        library.validate().unwrap();
        let reference = KeyAction::MacroReference { macro_id: "open-docs".into() };
        assert_eq!(library.find("open-docs").unwrap().name, "Open docs");
        let mut profile = Profile::default_profile();
        profile.set_action("key-3-1", 1, reference);
        profile.validate().unwrap();
        let decoded: MacroLibrary = serde_json::from_str(&serde_json::to_string(&library).unwrap()).unwrap();
        assert_eq!(decoded, library);
        assert!(MacroLibrary { macros: vec![], schema_version: 1 }.find("missing").is_none());
    }

    #[test]
    fn older_macro_definitions_default_to_fixed_once_settings() {
        let older = serde_json::json!({
            "schemaVersion": 1,
            "macroId": "legacy",
            "name": "Legacy",
            "steps": [{"delayMs": 0, "action": {"kind": "keyChord", "keys": ["A"]}}]
        });
        let parsed: MacroDefinition = serde_json::from_value(older).unwrap();
        assert_eq!(parsed.timing, MacroTiming::Fixed);
        assert_eq!(parsed.loop_mode, MacroLoopMode::Once);
        parsed.validate().unwrap();
    }

    #[test]
    fn macro_library_persists_with_recovery_backup() {
        let dir = std::env::temp_dir().join(format!("kohlrausch-macros-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("macros.json");
        let first = MacroLibrary::default();
        save_macro_library_at(&first, &path, &dir).unwrap();
        let mut updated = first.clone();
        updated.macros.push(MacroDefinition { schema_version: 1, macro_id: "docs".into(), name: "Docs".into(), steps: vec![MacroStep { delay_ms: 0, action: KeyAction::Website { url: "https://example.com".into(), label: None } }], timing: MacroTiming::Fixed, fixed_delay_ms: 0, loop_mode: MacroLoopMode::Once });
        save_macro_library_at(&updated, &path, &dir).unwrap();
        let saved: MacroLibrary = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(saved, updated);
        assert!(dir.join("macros.json.bak").exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn an_uncommitted_edit_and_cancel_leave_saved_assignment_untouched() {
        let original = Profile::default_profile();
        let mut draft = original.clone();
        draft.set_action("key-3-1", 1, KeyAction::KeyChord { keys: vec!["CTRL".into(), "P".into()] });
        assert_eq!(original.action("key-3-1", 1), &KeyAction::None);
        drop(draft); // Cancel discards the local draft before persistence.
        assert_eq!(original.action("key-3-1", 1), &KeyAction::None);
        let mut cleared = original.clone();
        cleared.set_action("key-3-1", 1, KeyAction::None);
        assert_eq!(cleared.validate(), Ok(()));
        assert_eq!(cleared.action("key-3-1", 1), &KeyAction::None);
    }

    #[test]
    fn per_layer_config_round_trip_preserves_other_layers() {
        let mut profile = Profile::default_profile();
        profile.set_action("key-3-1", 3, KeyAction::KeyChord { keys: vec!["CTRL".into(), "C".into()] });
        let layer: LayerConfig = serde_json::from_str(&serde_json::to_string_pretty(&LayerConfig::from_profile(&profile, 3).unwrap()).unwrap()).unwrap();
        let mut edited = layer.clone();
        edited.assignments[0].action = KeyAction::Website { url: "https://example.com".into(), label: None };
        let restored = edited.apply_to(&profile).unwrap();
        assert!(matches!(restored.action("key-3-1", 3), KeyAction::Website { .. }));
        assert_eq!(restored.action("key-3-1", 1), &KeyAction::None);
        assert_eq!(restored.assignments.len(), 36);
    }

    #[test]
    fn profile_v1_migration_preserves_assignments() {
        let dir = std::env::temp_dir().join(format!("kohlrausch-migrate-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let mut old = serde_json::to_value(Profile::default_profile()).unwrap();
        old["schemaVersion"] = 1.into();
        old["assignments"][0]["action"] = serde_json::json!({"kind":"keystroke","keys":["CTRL","C"]});
        let path = dir.join("default.json");
        fs::write(&path, serde_json::to_vec_pretty(&old).unwrap()).unwrap();
        let migrated = load_profile_file(&path).unwrap();
        assert_eq!(migrated.schema_version, 2);
        assert_eq!(migrated.action("key-3-1", 1), &KeyAction::KeyChord { keys: vec!["CTRL".into(), "C".into()] });
        assert!(path.with_extension("json.bak").exists());
        let _ = fs::remove_dir_all(dir);
    }
}
