import board from "./boards/kohlrausch-rp2040.json";
export type ModelPositionMm = { x:number; y:number };
export type PhysicalKey = { id:string; label:string; screenRow:number; screenColumn:number; matrixRow:number|null; matrixColumn:number|null; modelPositionMm:ModelPositionMm };
export const KOHLRAUSCH_BOARD = board;
const EXPECTED_LAYOUT = [["key-3-1","key-2-1","key-1-1"],["key-3-2","key-2-2","key-1-2"],["key-3-3","key-2-3","key-1-3"]];
export function validateBoard(value: unknown): string[] {
  const errors: string[] = [];
  if (!value || typeof value !== "object") return ["Board definition must be an object"];
  const b = value as typeof board;
  if (b.boardId !== "kohlrausch-rp2040-macropad") errors.push("Unexpected boardId");
  if (!Array.isArray(b.keys) || b.keys.length !== 9) return [...errors, "Board must define exactly nine keys"];
  if (b.keys.some(k => !k || typeof k.id !== "string" || typeof k.label !== "string" || !Number.isInteger(k.screenRow) || !Number.isInteger(k.screenColumn))) return [...errors, "Each key needs an ID, label and integer screen position"];
  if (b.keys.some(k => !k.modelPositionMm || !Number.isFinite(k.modelPositionMm.x) || !Number.isFinite(k.modelPositionMm.y))) errors.push("Each key needs a CAD-derived model position in millimeters");
  if (new Set(b.keys.map(k => k.id)).size !== 9) errors.push("Physical key IDs must be unique");
  if (new Set(b.keys.map(k => `${k.screenRow}:${k.screenColumn}`)).size !== 9) errors.push("Screen positions must be unique");
  if (JSON.stringify(b.layout) !== JSON.stringify(EXPECTED_LAYOUT)) errors.push("Layout must match the fixed PCB positions");
  for (const [row, ids] of EXPECTED_LAYOUT.entries()) for (const [column, id] of ids.entries()) {
    const key = b.keys.find(k => k.id === id);
    if (!key) { errors.push(`Layout references missing physical key ${id}`); continue; }
    if (key.screenRow !== row + 1 || key.screenColumn !== column + 1) errors.push(`${id} screen position does not match fixed layout`);
    if (key.label !== id.replace("key-", "").replace("-", ".")) errors.push(`${id} label does not match physical identity`);
  }
  if (b.mappingStatus !== "schematic-verified" && b.keys.some(k => k.matrixRow !== null || k.matrixColumn !== null)) errors.push("Unverified matrix identities must remain null");
  if (!b.lyrSwitch?.modelPositionMm || !Number.isFinite(b.lyrSwitch.modelPositionMm.x) || !Number.isFinite(b.lyrSwitch.modelPositionMm.y)) errors.push("LYR switch needs a CAD-derived model position");
  if (!b.usbPort?.modelPositionMm || !Number.isFinite(b.usbPort.modelPositionMm.x) || !Number.isFinite(b.usbPort.modelPositionMm.y)) errors.push("USB port needs a CAD-derived model position");
  return errors;
}
export function keyAt(row: number, column: number): PhysicalKey | undefined { return KOHLRAUSCH_BOARD.keys.find(key => key.screenRow === row && key.screenColumn === column); }
