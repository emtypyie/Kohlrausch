use crate::config::{
    KeyAction, MacroDefinition, MacroLibrary, MacroLoopMode, MacroStep, MacroTiming,
    MouseActionKind, Profile, KEY_IDS,
};

const CONFIG_MAGIC: u32 = 0x3146_434B;
const CONFIG_SCHEMA: u16 = 1;
const CONFIG_LAYER_COUNT: u8 = 4;
const CONFIG_MAX_MACROS: usize = 8;
const CONFIG_MAX_BYTES: usize = 8192;
const CONFIG_HEADER_SIZE: usize = 12;
#[cfg(test)]
const CONFIG_LAYER_SIZE: usize = 9 * 8;
const CONFIG_MACRO_HEADER_SIZE: usize = 12;
const CONFIG_MACRO_STEP_SIZE: usize = 12;

const MOD_LEFT_CTRL: u8 = 0x01;
const MOD_LEFT_SHIFT: u8 = 0x02;
const MOD_LEFT_ALT: u8 = 0x04;
const MOD_LEFT_GUI: u8 = 0x08;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HidChord {
    modifiers: u8,
    keycode: u8,
}

#[derive(Debug)]
pub struct DeviceConfigPayload {
    pub bytes: Vec<u8>,
    pub crc: u32,
    pub macro_count: u8,
}

pub fn build_device_config(
    profile: &Profile,
    library: &MacroLibrary,
) -> Result<DeviceConfigPayload, String> {
    profile.validate()?;
    library.validate()?;

    let mut referenced = Vec::<&MacroDefinition>::new();
    for layer in 1..=4 {
        for key_id in KEY_IDS {
            let assignment = profile
                .assignments
                .iter()
                .find(|item| item.layer == layer && item.key_id == key_id)
                .ok_or_else(|| "Profile is missing a key assignment".to_owned())?;
            if let KeyAction::MacroReference { macro_id } = &assignment.action {
                let definition = library
                    .find(macro_id)
                    .ok_or_else(|| format!("Macro '{macro_id}' no longer exists"))?;
                if !referenced
                    .iter()
                    .any(|item| item.macro_id == definition.macro_id)
                {
                    if referenced.len() >= CONFIG_MAX_MACROS {
                        return Err(format!(
                            "The device supports at most {CONFIG_MAX_MACROS} referenced macros per profile."
                        ));
                    }
                    referenced.push(definition);
                }
            }
        }
    }

    let mut bytes = Vec::with_capacity(CONFIG_MAX_BYTES);
    bytes.resize(CONFIG_HEADER_SIZE, 0);
    for layer in 1..=4 {
        for key_id in KEY_IDS {
            let assignment = profile
                .assignments
                .iter()
                .find(|item| item.layer == layer && item.key_id == key_id)
                .ok_or_else(|| "Profile is missing a key assignment".to_owned())?;
            bytes.extend_from_slice(&binding_bytes(&assignment.action, &referenced)?);
        }
    }

    for definition in &referenced {
        bytes.extend_from_slice(&macro_bytes(definition)?);
    }

    if bytes.len() > CONFIG_MAX_BYTES || bytes.len() > u16::MAX as usize {
        return Err("The device configuration exceeds its storage limit.".to_owned());
    }

    bytes[0..4].copy_from_slice(&CONFIG_MAGIC.to_le_bytes());
    bytes[4..6].copy_from_slice(&CONFIG_SCHEMA.to_le_bytes());
    bytes[6] = CONFIG_LAYER_COUNT;
    bytes[7] = referenced.len() as u8;
    let size = bytes.len() as u16;
    bytes[8..10].copy_from_slice(&size.to_le_bytes());
    bytes[10..12].copy_from_slice(&0u16.to_le_bytes());

    let crc = crc32(&bytes);
    Ok(DeviceConfigPayload {
        bytes,
        crc,
        macro_count: referenced.len() as u8,
    })
}

fn binding_bytes(action: &KeyAction, macros: &[&MacroDefinition]) -> Result<[u8; 8], String> {
    match action {
        KeyAction::None => Ok([0; 8]),
        KeyAction::KeyChord { keys } => {
            let chord = chord_to_hid(keys)?;
            Ok([1, chord.modifiers, chord.keycode, 0, 0, 0, 0, 0])
        }
        KeyAction::MouseAction { action } => Ok([2, 0, 0, 0, mouse_code(*action), 0, 0, 0]),
        KeyAction::MacroReference { macro_id } => {
            let index = macros
                .iter()
                .position(|item| item.macro_id == *macro_id)
                .ok_or_else(|| format!("Macro '{macro_id}' is not available for sync"))?;
            Ok([3, 0, 0, index as u8, 0, 0, 0, 0])
        }
        KeyAction::Website { .. } => {
            Err("Website actions run on the host and cannot be stored on the macropad.".to_owned())
        }
        KeyAction::Application { .. } => Err(
            "Application actions run on the host and cannot be stored on the macropad.".to_owned(),
        ),
    }
}

fn macro_bytes(definition: &MacroDefinition) -> Result<Vec<u8>, String> {
    if definition.steps.len() > 64 {
        return Err(format!(
            "Macro '{}' exceeds the device's 64-step limit.",
            definition.name
        ));
    }
    let (loop_mode, loop_count) = match definition.loop_mode {
        MacroLoopMode::Once => (0, 0),
        MacroLoopMode::UntilStop => (1, 0),
        MacroLoopMode::UntilAnyClick => (2, 0),
        MacroLoopMode::Count(count) => (3, count),
    };
    let timing = match definition.timing {
        MacroTiming::Fixed => 0,
        MacroTiming::Recorded => 1,
    };

    let mut bytes = Vec::with_capacity(
        CONFIG_MACRO_HEADER_SIZE + definition.steps.len() * CONFIG_MACRO_STEP_SIZE,
    );
    bytes.push(definition.steps.len() as u8);
    bytes.push(loop_mode);
    bytes.extend_from_slice(&loop_count.to_le_bytes());
    bytes.extend_from_slice(&definition.fixed_delay_ms.to_le_bytes());
    bytes.push(timing);
    bytes.extend_from_slice(&[0; 3]);
    for step in &definition.steps {
        bytes.extend_from_slice(&step_bytes(step, &definition.name)?);
    }
    Ok(bytes)
}

fn step_bytes(step: &MacroStep, macro_name: &str) -> Result<[u8; 12], String> {
    match &step.action {
        KeyAction::KeyChord { keys } => {
            let chord = chord_to_hid(keys)?;
            let mut bytes = [0; 12];
            bytes[0..4].copy_from_slice(&step.delay_ms.to_le_bytes());
            bytes[4] = 1;
            bytes[5] = chord.modifiers;
            bytes[6] = chord.keycode;
            Ok(bytes)
        }
        KeyAction::MouseAction { action } => {
            let mut bytes = [0; 12];
            bytes[0..4].copy_from_slice(&step.delay_ms.to_le_bytes());
            bytes[4] = 2;
            bytes[7] = mouse_code(*action);
            Ok(bytes)
        }
        KeyAction::Website { .. } | KeyAction::Application { .. } => Err(format!(
            "Macro '{macro_name}' contains a host-only website or application step and cannot be synced."
        )),
        KeyAction::None | KeyAction::MacroReference { .. } => Err(format!(
            "Macro '{macro_name}' contains an empty or nested step that cannot be synced."
        )),
    }
}

fn mouse_code(action: MouseActionKind) -> u8 {
    match action {
        MouseActionKind::LeftClick => 1,
        MouseActionKind::RightClick => 2,
        MouseActionKind::DoubleClick => 3,
        MouseActionKind::ScrollUp => 4,
        MouseActionKind::ScrollDown => 5,
    }
}

fn chord_to_hid(keys: &[String]) -> Result<HidChord, String> {
    let mut modifiers = 0;
    let mut keycode = None;
    for key in keys {
        let upper = key.to_ascii_uppercase();
        match upper.as_str() {
            "CTRL" => modifiers |= MOD_LEFT_CTRL,
            "SHIFT" => modifiers |= MOD_LEFT_SHIFT,
            "ALT" => modifiers |= MOD_LEFT_ALT,
            "META" => modifiers |= MOD_LEFT_GUI,
            _ => {
                let (usage, extra_modifier) = key_usage(&upper)
                    .ok_or_else(|| format!("Key '{key}' cannot be sent to the device."))?;
                if keycode.is_some() {
                    return Err(
                        "A device key chord must contain exactly one non-modifier key.".to_owned(),
                    );
                }
                keycode = Some(usage);
                modifiers |= extra_modifier;
            }
        }
    }
    let keycode = keycode.ok_or_else(|| {
        "A device key chord must contain exactly one non-modifier key.".to_owned()
    })?;
    Ok(HidChord { modifiers, keycode })
}

fn key_usage(name: &str) -> Option<(u8, u8)> {
    if name.len() == 1 {
        let byte = name.as_bytes()[0].to_ascii_uppercase();
        if byte.is_ascii_uppercase() {
            return Some((0x04 + (byte - b'A'), 0));
        }
        if byte.is_ascii_digit() {
            let usage = if byte == b'0' {
                0x27
            } else {
                0x1E + (byte - b'1')
            };
            return Some((usage, 0));
        }
    }

    let plain = match name {
        "SPACE" => 0x2C,
        "ENTER" => 0x28,
        "ESC" => 0x29,
        "TAB" => 0x2B,
        "BACKSPACE" => 0x2A,
        "DELETE" => 0x4C,
        "INSERT" => 0x49,
        "HOME" => 0x4A,
        "END" => 0x4D,
        "PAGEUP" => 0x4B,
        "PAGEDOWN" => 0x4E,
        "UP" => 0x52,
        "DOWN" => 0x51,
        "LEFT" => 0x50,
        "RIGHT" => 0x4F,
        "PRINTSCREEN" => 0x46,
        "PAUSE" => 0x48,
        "MINUS" | "-" => 0x2D,
        "EQUAL" | "=" => 0x2E,
        "LBRACKET" | "[" => 0x2F,
        "RBRACKET" | "]" => 0x30,
        "BACKSLASH" | "\\" => 0x31,
        "SEMICOLON" | ";" => 0x33,
        "APOSTROPHE" | "'" => 0x34,
        "GRAVE" | "`" => 0x35,
        "COMMA" | "," => 0x36,
        "PERIOD" | "." => 0x37,
        "SLASH" | "/" => 0x38,
        _ => {
            if let Some(number) = name
                .strip_prefix('F')
                .and_then(|value| value.parse::<u8>().ok())
            {
                if (1..=12).contains(&number) {
                    return Some((0x3A + number - 1, 0));
                }
                if (13..=24).contains(&number) {
                    return Some((0x68 + number - 13, 0));
                }
            }
            let shifted = match name {
                "TILDE" | "~" => 0x35,
                "BANG" | "!" => 0x1E,
                "AT" | "@" => 0x1F,
                "HASH" | "#" => 0x20,
                "DOLLAR" | "$" => 0x21,
                "PERCENT" | "%" => 0x22,
                "CARET" | "^" => 0x23,
                "AMPERSAND" | "&" => 0x24,
                "ASTERISK" | "*" => 0x25,
                "LPAREN" | "(" => 0x26,
                "RPAREN" | ")" => 0x27,
                "UNDERSCORE" | "_" => 0x2D,
                "PLUS" | "+" => 0x2E,
                "LBRACE" | "{" => 0x2F,
                "RBRACE" | "}" => 0x30,
                "PIPE" | "|" => 0x31,
                "COLON" | ":" => 0x33,
                "QUOTE" | "\"" => 0x34,
                "LESS" | "<" => 0x36,
                "GREATER" | ">" => 0x37,
                "QUESTION" | "?" => 0x38,
                _ => return None,
            };
            return Some((shifted, MOD_LEFT_SHIFT));
        }
    };
    Some((plain, 0))
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xEDB8_8320u32 & (0u32.wrapping_sub(crc & 1)));
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_profile() -> Profile {
        Profile::default_profile()
    }

    fn set(profile: &mut Profile, key_id: &str, layer: u8, action: KeyAction) {
        profile.set_action(key_id, layer, action);
    }

    #[test]
    fn hid_mapping_covers_modifiers_symbols_and_function_keys() {
        assert_eq!(
            chord_to_hid(&["CTRL".into(), "SHIFT".into(), "P".into()]).unwrap(),
            HidChord {
                modifiers: MOD_LEFT_CTRL | MOD_LEFT_SHIFT,
                keycode: 0x13
            }
        );
        assert_eq!(
            chord_to_hid(&["PLUS".into()]).unwrap(),
            HidChord {
                modifiers: MOD_LEFT_SHIFT,
                keycode: 0x2E
            }
        );
        assert_eq!(
            chord_to_hid(&["F13".into()]).unwrap(),
            HidChord {
                modifiers: 0,
                keycode: 0x68
            }
        );
        assert!(chord_to_hid(&["NOTAKEY".into()]).is_err());
    }

    #[test]
    fn crc_matches_the_standard_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn default_profile_builds_the_empty_device_layout() {
        let payload = build_device_config(&default_profile(), &MacroLibrary::default()).unwrap();
        assert_eq!(
            payload.bytes.len(),
            CONFIG_HEADER_SIZE + CONFIG_LAYER_COUNT as usize * CONFIG_LAYER_SIZE
        );
        assert_eq!(&payload.bytes[0..4], &CONFIG_MAGIC.to_le_bytes());
        assert_eq!(payload.bytes[6], CONFIG_LAYER_COUNT);
        assert_eq!(payload.bytes[7], 0);
        assert_eq!(payload.macro_count, 0);
        assert!(payload.bytes[CONFIG_HEADER_SIZE..]
            .iter()
            .all(|byte| *byte == 0));
    }

    #[test]
    fn builds_chords_mouse_and_macros_in_key_order() {
        let mut profile = default_profile();
        set(
            &mut profile,
            "key-3-1",
            1,
            KeyAction::KeyChord {
                keys: vec!["CTRL".into(), "C".into()],
            },
        );
        set(
            &mut profile,
            "key-2-1",
            1,
            KeyAction::MouseAction {
                action: MouseActionKind::DoubleClick,
            },
        );
        set(
            &mut profile,
            "key-1-1",
            1,
            KeyAction::MacroReference {
                macro_id: "combo".into(),
            },
        );

        let mut library = MacroLibrary::default();
        library.macros.push(MacroDefinition {
            schema_version: 1,
            macro_id: "combo".into(),
            name: "Combo".into(),
            steps: vec![
                MacroStep {
                    delay_ms: 10,
                    action: KeyAction::KeyChord {
                        keys: vec!["A".into()],
                    },
                },
                MacroStep {
                    delay_ms: 20,
                    action: KeyAction::MouseAction {
                        action: MouseActionKind::LeftClick,
                    },
                },
            ],
            timing: MacroTiming::Fixed,
            fixed_delay_ms: 50,
            loop_mode: MacroLoopMode::Count(2),
        });

        let payload = build_device_config(&profile, &library).unwrap();
        assert_eq!(payload.macro_count, 1);
        assert_eq!(payload.bytes.len(), 300 + 12 + 2 * 12);
        let first = CONFIG_HEADER_SIZE;
        assert_eq!(
            &payload.bytes[first..first + 8],
            &[1, MOD_LEFT_CTRL, 0x06, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            &payload.bytes[first + 8..first + 16],
            &[2, 0, 0, 0, 3, 0, 0, 0]
        );
        assert_eq!(
            &payload.bytes[first + 16..first + 24],
            &[3, 0, 0, 0, 0, 0, 0, 0]
        );
        let macro_start = 300;
        assert_eq!(payload.bytes[macro_start], 2);
        assert_eq!(payload.bytes[macro_start + 1], 3);
        assert_eq!(
            &payload.bytes[macro_start + 2..macro_start + 4],
            &2u16.to_le_bytes()
        );
        assert_eq!(
            &payload.bytes[macro_start + 4..macro_start + 8],
            &50u32.to_le_bytes()
        );
        assert_eq!(payload.bytes[macro_start + 8], 0);
        assert_eq!(payload.bytes[macro_start + 12], 10);
        assert_eq!(payload.bytes[macro_start + 16], 1);
        assert_eq!(payload.bytes[macro_start + 18], 0x04);
        assert_eq!(payload.bytes[macro_start + 24], 20);
        assert_eq!(payload.bytes[macro_start + 28], 2);
        assert_eq!(payload.bytes[macro_start + 31], 1);
    }

    #[test]
    fn rejects_host_only_actions_with_a_truthful_error() {
        let mut profile = default_profile();
        set(
            &mut profile,
            "key-3-1",
            1,
            KeyAction::Website {
                url: "https://example.com".into(),
                label: None,
            },
        );
        let error = build_device_config(&profile, &MacroLibrary::default()).unwrap_err();
        assert!(error.contains("Website actions run on the host"));

        let mut profile = default_profile();
        set(
            &mut profile,
            "key-3-1",
            1,
            KeyAction::MacroReference {
                macro_id: "host".into(),
            },
        );
        let mut library = MacroLibrary::default();
        library.macros.push(MacroDefinition {
            schema_version: 1,
            macro_id: "host".into(),
            name: "Host".into(),
            steps: vec![MacroStep {
                delay_ms: 0,
                action: KeyAction::Application {
                    name: "Editor".into(),
                    executable_path: None,
                    application_id: None,
                    arguments: Vec::new(),
                },
            }],
            timing: MacroTiming::Fixed,
            fixed_delay_ms: 0,
            loop_mode: MacroLoopMode::Once,
        });
        let error = build_device_config(&profile, &library).unwrap_err();
        assert!(error.contains("host-only"));
    }

    #[test]
    fn empty_binding_stays_empty() {
        let profile = default_profile();
        let payload = build_device_config(&profile, &MacroLibrary::default()).unwrap();
        assert_eq!(payload.bytes[CONFIG_HEADER_SIZE], 0);
    }
    #[test]
    fn recorded_timing_serializes_to_wire_value_one() {
        let mut profile = default_profile();
        set(
            &mut profile,
            "key-3-1",
            1,
            KeyAction::MacroReference {
                macro_id: "rec".into(),
            },
        );
        let mut library = MacroLibrary::default();
        library.macros.push(MacroDefinition {
            schema_version: 1,
            macro_id: "rec".into(),
            name: "Rec".into(),
            steps: vec![MacroStep {
                delay_ms: 42,
                action: KeyAction::KeyChord {
                    keys: vec!["A".into()],
                },
            }],
            timing: MacroTiming::Recorded,
            fixed_delay_ms: 0,
            loop_mode: MacroLoopMode::Once,
        });
        let payload = build_device_config(&profile, &library).unwrap();
        let macro_start = CONFIG_HEADER_SIZE + CONFIG_LAYER_COUNT as usize * CONFIG_LAYER_SIZE;
        assert_eq!(
            payload.bytes[macro_start + 8],
            1,
            "MacroTiming::Recorded must be wire value 1 to match firmware"
        );
        // Verify the step delay is preserved (firmware uses it when timing == Recorded)
        assert_eq!(
            &payload.bytes[macro_start + 12..macro_start + 16],
            &42u32.to_le_bytes(),
            "Step delay_ms must be preserved for Recorded timing"
        );
    }
}
