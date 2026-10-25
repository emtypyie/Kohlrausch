import { useKeyboard, useRenderer } from "@opentui/solid";
import { createSignal, For, Show, onMount, onCleanup } from "solid-js";
import { KOHLRAUSCH_BOARD } from "../../../packages/board-definitions/index.ts";
import { validateMacro, validateProfile, type KeyAction, type KeymapProfile, type LayerNumber, type MacroDefinition } from "../../../packages/config-schema/src/index.ts";
import { LocalProfileStore, createDefaultProfile, getAssignment, withAssignment } from "../../../host/simulator/profile-store.ts";
import { OLED_SCREEN_DURATION_MS } from "../../../host/simulator/oled-preview.ts";

const sections = ["Home", "Keymaps", "Layers", "Macros", "Profiles", "OLED", "Device settings"] as const;
type Section = typeof sections[number];
type Props = { initialSection?: string; onSectionChange?: (section: string) => void; store?: LocalProfileStore | null };

export function App(props: Props) {
  const renderer = useRenderer();
  const initial = (sections.includes(props.initialSection as Section) ? props.initialSection : "Home") as Section;
  const [section, setSectionRaw] = createSignal<Section>(initial);
  const [layer, setLayer] = createSignal<LayerNumber>(1);
  const [selectedKey, setSelectedKey] = createSignal(KOHLRAUSCH_BOARD.keys[0]!.id);
  const [keyEditorOpen, setKeyEditorOpen] = createSignal(false);
  const [keyActionKind, setKeyActionKind] = createSignal<"keystroke" | "website" | "software" | "macro" | "mouse">("keystroke");
  const [profiles, setProfiles] = createSignal<KeymapProfile[]>([createDefaultProfile()]);
  const [profileId, setProfileId] = createSignal("default");
  const profile = () => profiles().find(p => p.profileId === profileId()) ?? profiles()[0]!;
  const [macros, setMacros] = createSignal<MacroDefinition[]>([]);
  const [mode, setMode] = createSignal<MacroDefinition["mode"]>("record");
  const [draft, setDraft] = createSignal("");
  const [macroName, setMacroName] = createSignal("");
  const [error, setError] = createSignal("");
  const [notice, setNotice] = createSignal("Simulator ready · no physical device");
  const [flash, setFlash] = createSignal(false);
  const [oledStage, setOledStage] = createSignal<"layer" | "brand">("layer");
  const [hover, setHover] = createSignal("");
  let timer: ReturnType<typeof setTimeout> | undefined;
  const mouseProps = (id: string, action: () => void) => ({
    onMouseOver: () => { setHover(id); renderer.setMousePointer("pointer"); },
    onMouseOut: () => { setHover(""); renderer.setMousePointer("default"); },
    onMouseDown: (event: any) => { event.stopPropagation(); if (event.button === 0) event.currentTarget.__khlPressed = true; },
    onMouseUp: (event: any) => { event.stopPropagation(); if (event.button === 0 && event.currentTarget.__khlPressed) { event.currentTarget.__khlPressed = false; action(); } },
    onMouseDragEnd: (event: any) => { event.currentTarget.__khlPressed = false; },
  });
  const setSection = (next: Section) => { setSectionRaw(next); props.onSectionChange?.(next); };
  const restartOled = () => { if (timer) clearTimeout(timer); setOledStage("layer"); timer = setTimeout(() => { setOledStage("brand"); timer = setTimeout(restartOled, OLED_SCREEN_DURATION_MS); }, OLED_SCREEN_DURATION_MS); };
  const setLayerTo = (n: LayerNumber) => { setLayer(n); setNotice(`Simulator layer ${n} active`); restartOled(); };
  const cycleLayer = () => setLayerTo(layer() === 4 ? 1 : (layer() + 1) as LayerNumber);
  const saveProfile = async (next: KeymapProfile) => {
    const problems = validateProfile(next, KOHLRAUSCH_BOARD.keys.map(k => k.id));
    if (problems.length) { setError(problems.join("; ")); return; }
    setProfiles(old => old.map(p => p.profileId === next.profileId ? next : p));
    try { if (props.store !== null) await (props.store ?? new LocalProfileStore()).save(next); setError(""); setNotice(`Saved ${next.name} locally`); }
    catch (e) { setError(String(e)); }
  };
  const actionForKey = (id: string) => getAssignment(profile(), id, layer());
  const actionLabel = (action: KeyAction) => action.kind === "none" ? "Unassigned" : action.kind === "keystroke" ? action.keys.join("+") : action.kind === "macro" ? `Macro ${action.macroId}` : action.kind === "website" ? action.url : action.kind === "software" ? action.application : `${action.steps.length} mouse step(s)`;
  const selectKey = (id: string) => { setSelectedKey(id); setKeyEditorOpen(true); setNotice(`Selected ${KOHLRAUSCH_BOARD.keys.find(k => k.id === id)?.label}`); };
  const saveKeyAction = () => {
    const value = draft().trim();
    let action: KeyAction;
    if (keyActionKind() === "keystroke") action = { kind: "keystroke", keys: value.split("+").map(k => k.trim()).filter(Boolean) };
    else if (keyActionKind() === "website") action = { kind: "website", url: value };
    else if (keyActionKind() === "software") action = { kind: "software", application: value };
    else if (keyActionKind() === "macro") action = { kind: "macro", macroId: value };
    else action = { kind: "mouse", steps: [{ kind: (value || "click") as "click" | "doubleClick" | "scroll", button: "left" }] };
    const next = withAssignment(profile(), selectedKey(), layer(), action);
    const problems = validateProfile(next, KOHLRAUSCH_BOARD.keys.map(k => k.id));
    if (problems.length) { setError(problems.join("; ")); return; }
    setError(""); void saveProfile(next); setNotice(`Saved ${action.kind} assignment for ${KOHLRAUSCH_BOARD.keys.find(k => k.id === selectedKey())?.label} on Layer ${layer()}`);
  };
  const saveMacro = () => {
    const id = `macro-${Date.now()}`;
    let action: MacroDefinition["actions"][number];
    if (mode() === "website") action = { kind: "website", url: draft(), delayMs: 0 };
    else if (mode() === "software") action = { kind: "software", application: draft(), delayMs: 0 };
    else if (mode() === "clicks") action = { kind: "mouse", steps: [{ kind: (draft() || "click") as "click" | "doubleClick" | "scroll", button: "left" }], delayMs: 0 };
    else action = { kind: "keystroke", keys: draft().split("+").map(k => k.trim()).filter(Boolean), delayMs: 0 };
    const item: MacroDefinition = { id, name: macroName() || `${mode()} macro`, mode: mode(), actions: [action] };
    const problems = validateMacro(item); if (problems.length) { setError(problems.join("; ")); return; }
    setMacros(old => [...old, item]); setError(""); setNotice(`${item.name} saved · simulator only`);
  };
  onMount(() => { restartOled(); if (props.store !== null) (props.store ?? new LocalProfileStore()).load().then(items => { if (items.length) { setProfiles(items); setProfileId(items[0]!.profileId); } }).catch(e => setError(`Profile load: ${String(e)}`)); });
  onCleanup(() => { if (timer) clearTimeout(timer); renderer.setMousePointer("default"); });
  useKeyboard(key => {
    if (key.name === "q" || key.name === "ctrl+c") { renderer.destroy(); return; }
    if (key.name === "escape") { if (flash()) setFlash(false); else setSection("Home"); return; }
    if (["1", "2", "3", "4"].includes(key.name)) { setLayerTo(Number(key.name) as LayerNumber); return; }
    if (key.name === "l") { cycleLayer(); return; }
    if (key.name === "f") { setFlash(true); return; }
    if (key.name === "tab" && section() === "Macros") { const modes: MacroDefinition["mode"][] = ["record", "website", "software", "clicks"]; setMode(modes[(modes.indexOf(mode()) + 1) % modes.length]!); return; }
    if (key.name === "left" || key.name === "right" || key.name === "up" || key.name === "down") {
      if (section() === "Keymaps") { const keys = KOHLRAUSCH_BOARD.keys; const idx = keys.findIndex(k => k.id === selectedKey()); const row = Math.floor(idx / 3), col = idx % 3; const next = key.name === "left" ? (idx + 2) % 3 + row * 3 : key.name === "right" ? (idx + 1) % 3 + row * 3 : key.name === "up" ? (row + 2) % 3 * 3 + col : (row + 1) % 3 * 3 + col; selectKey(keys[next]!.id); }
      else { const idx = sections.indexOf(section()); setSection(sections[(idx + (key.name === "up" || key.name === "left" ? sections.length - 1 : 1)) % sections.length]!); }
      return;
    }
    if (key.name === "space" && section() === "Keymaps") { setNotice(`Mock action preview: ${actionLabel(actionForKey(selectedKey()))}`); return; }
    if (key.name === "return" || key.name === "enter") {
      if (section() === "Home") { setSection("Keymaps"); return; }
      if (section() === "Keymaps") { const current = actionForKey(selectedKey()); const options: KeyAction[] = [{ kind: "none" }, { kind: "keystroke", keys: ["CTRL", "SHIFT", "P"] }, { kind: "website", url: "https://example.com" }]; const index = options.findIndex(a => JSON.stringify(a) === JSON.stringify(current)); void saveProfile(withAssignment(profile(), selectedKey(), layer(), options[(index + 1) % options.length]!)); return; }
    }
  });
  const buttonStyle = (id: string, active = false) => ({ border: true, borderColor: active ? "#c781ff" : hover() === id ? "#a56bd0" : "#51405e", backgroundColor: active ? "#392444" : hover() === id ? "#21182b" : "#100d16" });
  return <box flexDirection="column" width="100%" height="100%" backgroundColor="#09070e" padding={1}>
    <box height={3} border borderColor="#7550a8" flexDirection="row" justifyContent="space-between" padding={1}><text height={1} fg="#e2c7ff">◈ KOHLRAUSCH</text><text height={1} fg="#c5a5d8">SIMULATOR · USB DISCONNECTED · LAYER {layer()}</text></box>
    <box flexGrow={1} flexDirection="row" marginTop={1}>
      <box width={22} border borderColor="#42344f" flexDirection="column" padding={1}><text height={1} fg="#a78cb5">WORKSPACE</text><For each={sections}>{item => <box height={2} {...mouseProps(`nav-${item}`, () => setSection(item))} {...buttonStyle(`nav-${item}`, section() === item)}><text height={1} fg={section() === item ? "#f0c7ff" : "#b9a9c0"}>{section() === item ? "◆ " : "  "}{item}</text></box>}</For><box flexGrow={1}/><box height={2} {...mouseProps("lyr", cycleLayer)} {...buttonStyle("lyr")}><text height={1} fg="#cda9ef">[L] Simulate LYR</text></box></box>
      <box flexGrow={1} border borderColor="#42344f" flexDirection="column" padding={2}>
        <Show when={flash()}><text height={1} fg="#ffd28e">FLASH · SIMULATION ONLY</text><text height={1}>Physical USB: disconnected. No update request or flash operation was attempted.</text><box height={2} {...mouseProps("close-flash", () => setFlash(false))} {...buttonStyle("close-flash")}><text height={1}>Close · Escape</text></box></Show>
        <Show when={!flash()}>
          <text height={1} fg="#a48aab">KOHLRAUSCH / {section().toUpperCase()}</text><text height={1}> </text>
          <Show when={section() === "Home"}><text height={1} fg="#e3c4ff">█  █  ██  █  █ █    ███   ██  █  █  ███  ███ █  █</text><text height={1} fg="#cba6f7">█ █  █  █ █  █ █    █  █ █  █ █  █ █    █    █  █</text><text height={1} fg="#bd95ef">██   █  █ ████ █    ███  ████ █  █  ██  █    ████</text><text height={1} fg="#ab7dde">█ █  █  █ █  █ █    █ █  █  █ █  █    █ █    █  █</text><text height={1} fg="#9c6ed0">█  █  ██  █  █ ████ █  █ █  █  ██  ███   ███ █  █</text><text height={1} fg="#dbc0f3">H O L O G R A M · SIMULATOR READY</text><text height={1}>Hardware independent workspace · use mouse or keyboard to explore</text></Show>
          <Show when={section() === "Keymaps"}><text height={1} fg="#ddc2f5">FIXED PCB LAYOUT · Layer {layer()} · physical identity is developer maintained</text><text height={1}>Click a key to open its editor. Arrows move, Enter cycles, Space previews.</text><text height={1}> </text><For each={KOHLRAUSCH_BOARD.layout}>{row => <box flexDirection="row">{row.map(id => { const key = KOHLRAUSCH_BOARD.keys.find(k => k.id === id)!; const active = selectedKey() === id; return <box width={24} height={5} marginRight={1} flexDirection="column" justifyContent="center" alignItems="center" {...mouseProps(`key-${id}`, () => { selectKey(id); setKeyEditorOpen(true); })} {...buttonStyle(`key-${id}`, active)}><text height={1} fg={active ? "#f1c7ff" : "#cab8d5"}>{key.label}</text><text height={1} fg="#9989a2">{actionLabel(actionForKey(id))}</text></box>; })}</box>}</For><text height={1} fg="#c9a9e7">Selected {KOHLRAUSCH_BOARD.keys.find(k => k.id === selectedKey())?.label} · Layer {layer()} · {actionLabel(actionForKey(selectedKey()))}</text><Show when={keyEditorOpen()}><input value={draft()} placeholder={keyActionKind() === "keystroke" ? "CTRL+SHIFT+P" : keyActionKind() === "website" ? "https://example.com" : keyActionKind() === "software" ? "Application label" : keyActionKind() === "macro" ? "Macro ID" : "click / doubleClick / scroll"} onInput={(v: string) => setDraft(v)} /><box height={2} {...mouseProps("key-action-kind", () => { const kinds: typeof keyActionKind extends () => infer T ? T[] : never = ["keystroke", "website", "software", "macro", "mouse"]; setKeyActionKind(kinds[(kinds.indexOf(keyActionKind()) + 1) % kinds.length]!); })} {...buttonStyle("key-action-kind")}><text height={1}>Action type · {keyActionKind()} · click to change</text></box><box height={2} {...mouseProps("save-key-action", saveKeyAction)} {...buttonStyle("save-key-action")}><text height={1}>Save assignment</text></box></Show><Show when={error()}><text height={1} fg="#ff8080">{error()}</text></Show></Show>
          <Show when={section() === "Layers"}><text height={1}>Startup layer 1 · virtual LYR cycles 1 → 2 → 3 → 4 → 1</text><text height={1}>Select a layer to view its independent key assignments.</text><For each={[1,2,3,4] as LayerNumber[]}>{n => <box height={3} marginTop={1} {...mouseProps(`layer-${n}`, () => setLayerTo(n))} {...buttonStyle(`layer-${n}`, layer() === n)}><text height={1} fg={layer() === n ? "#efcaff" : "#aa99b5"}>{layer() === n ? "▶ " : "  "}LAYER {n} {layer() === n ? "· ACTIVE SIMULATOR LAYER" : ""}</text></box>}</For></Show>
          <Show when={section() === "Macros"}><text height={1}>Macro actions are saved and previewed locally. Host automation is not enabled.</text><text height={1}>Creation mode: {mode()} · Tab cycles modes</text><text height={1}>Name (keyboard): type after selecting field · Enter saves</text><input value={macroName()} placeholder="Macro name" onInput={(v: string) => setMacroName(v)} /><input value={draft()} placeholder={mode() === "record" ? "CTRL+SHIFT+P" : mode() === "website" ? "https://example.com" : mode() === "software" ? "Application label" : "click / doubleClick / scroll"} onInput={(v: string) => setDraft(v)} /><box height={2} {...mouseProps("macro-mode", () => { const modes: MacroDefinition["mode"][] = ["record", "website", "software", "clicks"]; setMode(modes[(modes.indexOf(mode()) + 1) % modes.length]!); })} {...buttonStyle("macro-mode")}><text height={1}>Change action mode</text></box><box height={2} {...mouseProps("macro-save", saveMacro)} {...buttonStyle("macro-save")}><text height={1}>Save preview</text></box><Show when={error()}><text height={1} fg="#ff8080">{error()}</text></Show><For each={macros()}>{m => <text height={1} fg="#d8c1e6">{m.name} · {m.mode} · {m.actions.map(a => a.kind).join(" → ")} · not executed</text>}</For></Show>
          <Show when={section() === "Profiles"}><text height={1}>Profiles are stored separately from immutable board metadata. Active: {profile().name}</text><For each={profiles()}>{p => <box height={2} {...mouseProps(`profile-${p.profileId}`, () => { setProfileId(p.profileId); setNotice(`Profile ${p.name} selected`); })} {...buttonStyle(`profile-${p.profileId}`, profileId() === p.profileId)}><text height={1}>{profileId() === p.profileId ? "◆ " : "  "}{p.name} · schema v{p.schemaVersion}</text></box>}</For><input value={draft()} placeholder="Profile name" onInput={(v: string) => setDraft(v)} /><box height={2} {...mouseProps("create-profile", () => { const name = draft().trim(); if (!name) { setError("Profile name required"); return; } const id = name.toLowerCase().replace(/[^a-z0-9_-]+/g, "-"); if (profiles().some(p => p.profileId === id)) { setError("A profile with that ID already exists"); return; } const next = { ...createDefaultProfile(id, name) }; setProfiles(old => [...old, next]); setProfileId(id); setDraft(""); void saveProfile(next); })} {...buttonStyle("create-profile")}><text height={1}>Create profile</text></box><box height={2} {...mouseProps("rename-profile", () => { const name = draft().trim(); if (!name) { setError("Enter a name to rename the active profile"); return; } void saveProfile({ ...profile(), name }); setDraft(""); })} {...buttonStyle("rename-profile")}><text height={1}>Rename active profile</text></box><Show when={profiles().length > 1}><box height={2} {...mouseProps("delete-profile", async () => { const removed = profileId(); try { if (props.store !== null) await (props.store ?? new LocalProfileStore()).delete(removed); const remaining = profiles().filter(p => p.profileId !== removed); setProfiles(remaining); setProfileId(remaining[0]!.profileId); setNotice("Profile deleted"); setError(""); } catch (e) { setError(String(e)); } })} {...buttonStyle("delete-profile")}><text height={1}>Delete active profile</text></box></Show><Show when={error()}><text height={1} fg="#ff8080">{error()}</text></Show></Show>
          <Show when={section() === "OLED"}><text height={1}>OLED PREVIEW · SIMULATION · alternating every five seconds</text><box border borderColor="#9870b6" padding={1} marginTop={1}><Show when={oledStage() === "layer"}><text height={1}>[ 1 ]  [ 2 ]  [ 3 ]  [ 4 ]</text><text height={1}>        Active layer: {layer()}         </text><text height={1}>Layer screen · next branding screen in 5s</text></Show><Show when={oledStage() === "brand"}><text height={1} fg="#e3c4ff">       KOHLRAUSCH       </text><text height={1}>Brand screen · next layer screen in 5s</text></Show></box><text height={1}>Controller, resolution and I²C address: pending schematic/part verification.</text></Show>
          <Show when={section() === "Device settings"}><text height={1}>Transport: MOCK · physical USB: DISCONNECTED</text><text height={1}>Firmware: unavailable · no update operation enabled</text><text height={1}>Board matrix, diode behavior and OLED part remain unverified.</text></Show>
        </Show>
      </box>
    </box>
    <box height={3} border borderColor="#51405e" flexDirection="row" justifyContent="space-between" padding={1}><text height={1} fg="#bba9c7">{notice()}</text><box {...mouseProps("flash", () => setFlash(true))} {...buttonStyle("flash")}><text height={1} fg="#f1bd84">FLASH [F] · MOCK ONLY</text></box></box>
    <text height={1} fg="#83758a">Mouse click controls · ↑↓←→ navigate · Enter select · Esc home · 1–4 layer · L virtual LYR · F mock FLASH · Q quit</text>
  </box>;
}
