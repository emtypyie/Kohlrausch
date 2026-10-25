import type { KeyAction, KeymapProfile, LayerNumber, MacroDefinition } from "../../packages/config-schema/src/index.ts";
import { KOHLRAUSCH_BOARD } from "../../packages/board-definitions/index.ts";
import { createDefaultProfile, getAssignment, nextLayer, withAssignment } from "./profile-store.ts";

export const WORKSPACE_SECTIONS = ["Home", "Keymaps", "Layers", "Macros", "Profiles", "OLED", "Device"] as const;
export type WorkspaceSection = typeof WORKSPACE_SECTIONS[number];
export type MacroMode = "record" | "website" | "software" | "clicks";
export type WorkspaceState = {
  section: WorkspaceSection;
  activeLayer: LayerNumber;
  selectedKeyId: string;
  keyEditorOpen: boolean;
  profiles: KeymapProfile[];
  activeProfileId: string;
  macros: MacroDefinition[];
  selectedMacroId: string | null;
  macroMode: MacroMode;
  macroFormOpen: boolean;
  editingMacroId: string | null;
  flashOpen: boolean;
  notice: string;
};

export function createWorkspaceState(profiles: KeymapProfile[] = [createDefaultProfile()]): WorkspaceState {
  return {
    section: "Home", activeLayer: 1, selectedKeyId: KOHLRAUSCH_BOARD.keys[0]!.id,
    keyEditorOpen: false, profiles, activeProfileId: profiles[0]?.profileId ?? "default",
    macros: [], selectedMacroId: null, macroMode: "record", macroFormOpen: false,
    editingMacroId: null, flashOpen: false, notice: "Simulator ready · no physical device",
  };
}

export function changeLayer(state: WorkspaceState, layer: LayerNumber): WorkspaceState {
  return { ...state, activeLayer: layer, notice: `Simulator layer ${layer} active` };
}
export function cycleLyr(state: WorkspaceState): WorkspaceState { return changeLayer(state, nextLayer(state.activeLayer)); }
export function activeProfile(state: WorkspaceState): KeymapProfile {
  return state.profiles.find(profile => profile.profileId === state.activeProfileId) ?? state.profiles[0] ?? createDefaultProfile();
}
export function currentAction(state: WorkspaceState): KeyAction {
  return getAssignment(activeProfile(state), state.selectedKeyId, state.activeLayer);
}
export function updateKeyAction(state: WorkspaceState, action: KeyAction): WorkspaceState {
  const profile = withAssignment(activeProfile(state), state.selectedKeyId, state.activeLayer, action);
  return { ...state, profiles: state.profiles.map(item => item.profileId === profile.profileId ? profile : item), notice: `${state.selectedKeyId} · Layer ${state.activeLayer} saved` };
}
export function selectKey(state: WorkspaceState, keyId: string, openEditor = true): WorkspaceState {
  if (!KOHLRAUSCH_BOARD.keys.some(key => key.id === keyId)) return state;
  return { ...state, selectedKeyId: keyId, keyEditorOpen: openEditor, notice: `Selected ${KOHLRAUSCH_BOARD.keys.find(key => key.id === keyId)!.label}` };
}
export function selectProfile(state: WorkspaceState, profileId: string): WorkspaceState {
  if (!state.profiles.some(profile => profile.profileId === profileId)) return state;
  return { ...state, activeProfileId: profileId, notice: `Profile ${state.profiles.find(profile => profile.profileId === profileId)!.name} active` };
}
export function insertProfile(state: WorkspaceState, profile: KeymapProfile): WorkspaceState {
  return { ...state, profiles: [...state.profiles.filter(item => item.profileId !== profile.profileId), profile], activeProfileId: profile.profileId, notice: `Profile ${profile.name} saved` };
}
export function addMacro(state: WorkspaceState, macro: MacroDefinition): WorkspaceState {
  const macros = state.macros.some(item => item.id === macro.id) ? state.macros.map(item => item.id === macro.id ? macro : item) : [...state.macros, macro];
  return { ...state, macros, selectedMacroId: macro.id, macroFormOpen: false, editingMacroId: null, notice: `Macro ${macro.name} saved to simulator` };
}
