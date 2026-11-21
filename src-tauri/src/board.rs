use serde::Deserialize;
#[cfg(test)]
use std::sync::OnceLock;

pub const BOARD_ID: &str = "kohlrausch-rp2040-macropad";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogicalKey {
    pub id: &'static str,
    pub label: &'static str,
    pub screen_row: usize,
    pub screen_column: usize,
    pub physical_id: &'static str,
    pub physical_reference: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoardDefinition {
    board_id: String,
    keys: Vec<PhysicalKey>,
    layout: Vec<Vec<String>>,
    lyr_switch: SwitchMetadata,
    usb_port: UsbMetadata,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhysicalKey {
    id: String,
    label: String,
    screen_row: usize,
    screen_column: usize,
    matrix_row: Option<usize>,
    matrix_column: Option<usize>,
    model_position_mm: ModelPositionMm,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModelPositionMm { x: f32, y: f32 }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SwitchMetadata { model_position_mm: ModelPositionMm }
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsbMetadata { model_position_mm: ModelPositionMm }

// Logical screen positions map onto the fixed, component-side PCB positions.
// Electrical matrix coordinates intentionally stay null pending schematic verification.
pub const LOGICAL_KEYS: [LogicalKey; 9] = [
    LogicalKey { id: "logical-1-1", label: "3.1", screen_row: 0, screen_column: 0, physical_id: "key-3-1", physical_reference: "3.1" },
    LogicalKey { id: "logical-1-2", label: "2.1", screen_row: 0, screen_column: 1, physical_id: "key-2-1", physical_reference: "2.1" },
    LogicalKey { id: "logical-1-3", label: "1.1", screen_row: 0, screen_column: 2, physical_id: "key-1-1", physical_reference: "1.1" },
    LogicalKey { id: "logical-2-1", label: "3.2", screen_row: 1, screen_column: 0, physical_id: "key-3-2", physical_reference: "3.2" },
    LogicalKey { id: "logical-2-2", label: "2.2", screen_row: 1, screen_column: 1, physical_id: "key-2-2", physical_reference: "2.2" },
    LogicalKey { id: "logical-2-3", label: "1.2", screen_row: 1, screen_column: 2, physical_id: "key-1-2", physical_reference: "1.2" },
    LogicalKey { id: "logical-3-1", label: "3.3", screen_row: 2, screen_column: 0, physical_id: "key-3-3", physical_reference: "3.3" },
    LogicalKey { id: "logical-3-2", label: "2.3", screen_row: 2, screen_column: 1, physical_id: "key-2-3", physical_reference: "2.3" },
    LogicalKey { id: "logical-3-3", label: "1.3", screen_row: 2, screen_column: 2, physical_id: "key-1-3", physical_reference: "1.3" },
];

pub fn validate_definition(json: &str) -> Result<(), String> {
    let board: BoardDefinition = serde_json::from_str(json).map_err(|error| error.to_string())?;
    if board.board_id != BOARD_ID { return Err("Unexpected board ID".into()); }
    if board.keys.len() != 9 || board.layout.len() != 3 || board.layout.iter().any(|row| row.len() != 3) { return Err("Board must define one fixed 3×3 layout".into()); }
    if board.keys.iter().any(|key| key.matrix_row.is_some() || key.matrix_column.is_some()) { return Err("Unverified electrical mapping must remain unknown".into()); }
    if board.keys.iter().any(|key| !key.model_position_mm.x.is_finite() || !key.model_position_mm.y.is_finite()) { return Err("CAD model positions must be finite millimeter values".into()); }
    let mut model_positions: Vec<_> = board.keys.iter().map(|key| (key.model_position_mm.x, key.model_position_mm.y)).collect();
    model_positions.sort_by(|left, right| left.0.total_cmp(&right.0).then(left.1.total_cmp(&right.1)));
    model_positions.dedup_by(|left, right| (left.0 - right.0).abs() < 0.01 && (left.1 - right.1).abs() < 0.01);
    if model_positions.len() != 9 { return Err("Each physical switch needs a unique CAD model position".into()); }
    if !board.lyr_switch.model_position_mm.x.is_finite() || !board.lyr_switch.model_position_mm.y.is_finite()
        || !board.usb_port.model_position_mm.x.is_finite() || !board.usb_port.model_position_mm.y.is_finite() {
        return Err("LYR and USB model positions must be finite millimeter values".into());
    }
    let mut physical = Vec::new();
    for logical in LOGICAL_KEYS {
        let physical_id = &board.layout[logical.screen_row][logical.screen_column];
        let key = board.keys.iter().find(|key| &key.id == physical_id).ok_or_else(|| "Layout refers to an unknown physical switch".to_string())?;
        if key.label != logical.physical_reference || key.screen_row != logical.screen_row + 1 || key.screen_column != logical.screen_column + 1 {
            return Err("Logical-to-physical positional mapping conflicts with the board definition".into());
        }
        physical.push(key.id.as_str());
    }
    physical.sort_unstable();
    physical.dedup();
    if physical.len() != 9 { return Err("Logical map must cover every physical switch exactly once".into()); }
    Ok(())
}

#[cfg(test)]
fn embedded_definition() -> &'static BoardDefinition {
    static BOARD: OnceLock<BoardDefinition> = OnceLock::new();
    BOARD.get_or_init(|| serde_json::from_str(BOARD_JSON).expect("embedded board definition must be valid JSON"))
}

#[cfg(test)]
pub fn model_position_mm(physical_id: &str) -> Option<(f32, f32)> {
    embedded_definition().keys.iter().find(|key| key.id == physical_id)
        .map(|key| (key.model_position_mm.x, key.model_position_mm.y))
}

#[cfg(test)]
pub fn lyr_position_mm() -> (f32, f32) {
    let position = embedded_definition().lyr_switch.model_position_mm;
    (position.x, position.y)
}

#[cfg(test)]
pub fn usb_position_mm() -> (f32, f32) {
    let position = embedded_definition().usb_port.model_position_mm;
    (position.x, position.y)
}

pub const BOARD_JSON: &str = include_str!("../../packages/board-definitions/boards/kohlrausch-rp2040.json");

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn mapping_matches_logical_and_physical_screen_positions() {
        validate_definition(BOARD_JSON).unwrap();
        let labels: Vec<_> = LOGICAL_KEYS.iter().map(|key| key.label).collect();
        assert_eq!(labels, ["3.1", "2.1", "1.1", "3.2", "2.2", "1.2", "3.3", "2.3", "1.3"]);
        let ids: std::collections::HashSet<_> = LOGICAL_KEYS.iter().map(|key| key.physical_id).collect();
        assert_eq!(ids.len(), 9);
        for key in LOGICAL_KEYS {
            assert_eq!(key.physical_id, format!("key-{}", key.physical_reference.replace('.', "-")));
            assert!(BOARD_JSON.contains(&format!("\"{}\"", key.physical_id)));
        }
        assert_eq!(model_position_mm("key-3-1"), Some((-19.77, 7.92)));
        assert_eq!(lyr_position_mm(), (-19.05, 27.13));
        assert_eq!(usb_position_mm(), (0.38, 40.94));
    }
}
