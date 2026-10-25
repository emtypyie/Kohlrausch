use crate::board::{find_logical, LogicalKey};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionState {
    pub selected_logical_id: String,
    pub hovered_logical_id: Option<String>,
    pub active_layer: u8,
}

impl Default for SelectionState {
    fn default() -> Self { Self { selected_logical_id: "logical-1-1".into(), hovered_logical_id: None, active_layer: 1 } }
}

impl SelectionState {
    pub fn select(&mut self, logical_id: &str) -> bool {
        if find_logical(logical_id).is_none() { return false; }
        self.selected_logical_id = logical_id.to_owned();
        true
    }
    pub fn hover(&mut self, logical_id: Option<&str>) {
        self.hovered_logical_id = logical_id.filter(|id| find_logical(id).is_some()).map(str::to_owned);
    }
    pub fn set_layer(&mut self, layer: u8) -> bool {
        if !(1..=4).contains(&layer) { return false; }
        self.active_layer = layer;
        true
    }
    pub fn advance_layer(&mut self) -> u8 {
        self.active_layer = self.active_layer % 4 + 1;
        self.active_layer
    }
    pub fn selected_key(&self) -> &'static LogicalKey { find_logical(&self.selected_logical_id).expect("selection is validated") }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn inspector_identity_stays_in_sync_with_selected_scene_object() {
        let mut state = SelectionState::default();
        assert!(state.select("logical-2-3"));
        assert_eq!(state.selected_key().label, "2.3");
        assert_eq!(state.selected_key().physical_reference, "1.2");
        state.set_layer(4);
        assert_eq!(state.selected_key().id, state.selected_logical_id);
        assert_eq!(state.active_layer, 4);
    }
    #[test] fn hovering_never_changes_selection() {
        let mut state = SelectionState::default();
        state.hover(Some("logical-3-2"));
        assert_eq!(state.selected_logical_id, "logical-1-1");
        assert_eq!(state.hovered_logical_id.as_deref(), Some("logical-3-2"));
    }
    #[test] fn dedicated_lyr_switch_cycles_once_per_press() {
        let mut state=SelectionState::default();
        assert_eq!((0..4).map(|_| state.advance_layer()).collect::<Vec<_>>(),[2,3,4,1]);
    }
}
