export type LayerNumber = 1 | 2 | 3 | 4;
export type MouseStep = { kind: "click" | "doubleClick" | "scroll"; button?: "left" | "right" | "middle"; amount?: number; delayMs?: number };
export type KeyAction = { kind: "none" } | { kind: "keystroke"; keys: string[] } | { kind: "macro"; macroId: string } | { kind: "website"; url: string } | { kind: "software"; application: string } | { kind: "mouse"; steps: MouseStep[] };
export type KeyAssignment = { keyId: string; layer: LayerNumber; action: KeyAction };
export type KeymapProfile = { schemaVersion: 1; profileId: string; name: string; boardId: string; assignments: KeyAssignment[] };
export type MacroAction = { kind: "keystroke"; keys: string[]; delayMs: number } | { kind: "delay"; delayMs: number } | { kind: "website"; url: string; delayMs: number } | { kind: "software"; application: string; delayMs: number } | { kind: "mouse"; steps: MouseStep[]; delayMs: number };
export type MacroDefinition = { id: string; name: string; mode: "record" | "website" | "software" | "clicks"; actions: MacroAction[] };

function validWebUrl(value: unknown): value is string {
  if (typeof value !== "string") return false;
  try { const url = new URL(value); return (url.protocol === "http:" || url.protocol === "https:") && url.hostname.length > 0; } catch { return false; }
}
function validAction(action: unknown): action is KeyAction {
  if (!action || typeof action !== "object" || !("kind" in action)) return false;
  const a = action as Record<string, unknown>;
  switch (a.kind) {
    case "none": return true;
    case "keystroke": return Array.isArray(a.keys) && a.keys.length > 0 && a.keys.every(k => typeof k === "string" && k.trim().length > 0);
    case "macro": return typeof a.macroId === "string" && a.macroId.trim().length > 0;
    case "website": return validWebUrl(a.url);
    case "software": return typeof a.application === "string" && a.application.trim().length > 0;
    case "mouse": return Array.isArray(a.steps) && a.steps.length > 0 && a.steps.every(s => s && typeof s === "object" && ["click", "doubleClick", "scroll"].includes((s as MouseStep).kind));
    default: return false;
  }
}

export function validateProfile(input: unknown, keyIds: string[]): string[] {
  const errors: string[] = [];
  if (!input || typeof input !== "object") return ["Profile must be an object"];
  const p = input as Partial<KeymapProfile>;
  if (p.schemaVersion !== 1) errors.push("Unsupported schemaVersion");
  if (typeof p.profileId !== "string" || !p.profileId.trim()) errors.push("profileId required");
  if (typeof p.name !== "string" || !p.name.trim()) errors.push("name required");
  if (typeof p.boardId !== "string" || !p.boardId.trim()) errors.push("boardId required");
  if (!Array.isArray(p.assignments)) return [...errors, "assignments must be an array"];
  const counts = new Map<LayerNumber, number>([[1, 0], [2, 0], [3, 0], [4, 0]]);
  const seen = new Set<string>();
  for (const [index, raw] of p.assignments.entries()) {
    if (!raw || typeof raw !== "object") { errors.push(`Assignment ${index + 1} must be an object`); continue; }
    const a = raw as KeyAssignment;
    if (![1, 2, 3, 4].includes(a.layer)) { errors.push(`Assignment ${index + 1} has invalid layer`); continue; }
    const key = `${a.layer}:${a.keyId}`;
    counts.set(a.layer, counts.get(a.layer)! + 1);
    if (typeof a.keyId !== "string" || !keyIds.includes(a.keyId)) errors.push(`Unknown physical key ${String(a.keyId)}`);
    if (seen.has(key)) errors.push(`Duplicate assignment ${key}`);
    seen.add(key);
    if (!validAction(a.action)) errors.push(`Assignment ${index + 1} has invalid action data`);
  }
  for (const layer of [1, 2, 3, 4] as const) if (counts.get(layer) !== 9) errors.push(`Layer ${layer} must contain exactly nine assignments`);
  return errors;
}

export function validateMacro(input: MacroDefinition): string[] {
  const errors: string[] = [];
  if (!input || typeof input !== "object") return ["Macro must be an object"];
  const m = input as MacroDefinition;
  if (typeof m.name !== "string" || !m.name.trim()) errors.push("Macro name required");
  if (!Array.isArray(m.actions) || !m.actions.length) return [...errors, "Add at least one action"];
  m.actions.forEach((a, i) => {
    if (!a || typeof a !== "object") { errors.push(`Action ${i + 1} must be an object`); return; }
    if (!Number.isFinite(a.delayMs) || a.delayMs < 0) errors.push(`Action ${i + 1} delay must be non-negative`);
    if (a.kind === "website" && !validWebUrl(a.url)) errors.push(`Action ${i + 1} URL must use http or https`);
    if (a.kind === "software" && (typeof a.application !== "string" || !a.application.trim())) errors.push(`Action ${i + 1} application required`);
    if (a.kind === "keystroke" && (!Array.isArray(a.keys) || !a.keys.length || a.keys.some(k => !k.trim()))) errors.push(`Action ${i + 1} needs a key`);
    if (a.kind === "mouse" && (!Array.isArray(a.steps) || !a.steps.length)) errors.push(`Action ${i + 1} needs a step`);
    if (a.kind === "mouse" && Array.isArray(a.steps) && a.steps.some(s => !s || !["click", "doubleClick", "scroll"].includes(s.kind))) errors.push(`Action ${i + 1} has an invalid mouse step`);
    if (!["keystroke", "delay", "website", "software", "mouse"].includes(a.kind)) errors.push(`Action ${i + 1} has an unknown action kind`);
  });
  return errors;
}
export function serializeProfile(profile: KeymapProfile): string { return JSON.stringify(profile, null, 2); }
export function parseProfile(json: string, keyIds: string[]): KeymapProfile {
  const profile: unknown = JSON.parse(json);
  const errors = validateProfile(profile, keyIds);
  if (errors.length) throw new Error(errors.join("; "));
  return profile as KeymapProfile;
}


