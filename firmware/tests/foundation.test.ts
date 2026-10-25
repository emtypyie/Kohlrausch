import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { KOHLRAUSCH_BOARD, validateBoard } from "../../packages/board-definitions/index.ts";
import { validateProfile, serializeProfile, parseProfile, validateMacro, type KeymapProfile, type MacroDefinition } from "../../packages/config-schema/src/index.ts";
import { advanceLayer, MockDeviceTransport, executeMockMacro } from "../../host/simulator/mock-device.ts";
import { nextLayer } from "../../firmware/src/layer_logic.ts";
import { isDeviceStatus } from "../../packages/device-protocol/src/index.ts";
import { OLED_SCREEN_DURATION_MS, nextOledPreviewScreen, oledLayerBoxes } from "../../host/simulator/oled-preview.ts";

const ids = KOHLRAUSCH_BOARD.keys.map(key => key.id);
function profile(): KeymapProfile {
  return { schemaVersion: 1, profileId: "test", name: "Test", boardId: KOHLRAUSCH_BOARD.boardId,
    assignments: [1, 2, 3, 4].flatMap(layer => ids.map(keyId => ({ layer: layer as 1|2|3|4, keyId, action: { kind: "none" as const } }))) };
}

describe("board definition", () => {
  test("board schema and sample profile schema are valid JSON with required fields", () => {
    const boardSchema = JSON.parse(readFileSync(new URL("../../packages/board-definitions/board.schema.json", import.meta.url), "utf8"));
    const profileSchema = JSON.parse(readFileSync(new URL("../../packages/config-schema/keymap-profile-v1.schema.json", import.meta.url), "utf8"));
    const sample = JSON.parse(readFileSync(new URL("../../profiles/default.json", import.meta.url), "utf8"));
    expect(boardSchema.required).toContain("keys"); expect(profileSchema.properties.schemaVersion.const).toBe(1);
    expect(validateProfile(sample, ids)).toEqual([]);
  });
  test("has nine unique keys in exact fixed positions", () => {
    expect(validateBoard(KOHLRAUSCH_BOARD)).toEqual([]);
    expect(KOHLRAUSCH_BOARD.layout).toEqual([["key-3-1","key-2-1","key-1-1"],["key-3-2","key-2-2","key-1-2"],["key-3-3","key-2-3","key-1-3"]]);
    expect(new Set(KOHLRAUSCH_BOARD.keys.map(k => `${k.screenRow}:${k.screenColumn}`)).size).toBe(9);
  });
  test("rejects duplicate IDs and malformed input without crashing", () => {
    const b = structuredClone(KOHLRAUSCH_BOARD); b.keys[1]!.id = b.keys[0]!.id;
    expect(validateBoard(b).join(" ")).toContain("unique");
    const shifted = structuredClone(KOHLRAUSCH_BOARD); shifted.keys[0]!.screenColumn = 2;
    expect(validateBoard(shifted).join(" ")).toContain("screen position");
    expect(validateBoard({boardId:"bad",keys:null})).toContain("Board must define exactly nine keys");
  });
  test("keeps unverified electrical identities and OLED metadata pending", () => {
    expect(KOHLRAUSCH_BOARD.keys.every(k => k.matrixRow === null && k.matrixColumn === null)).toBe(true);
    expect(KOHLRAUSCH_BOARD.matrix.verified).toBe(false);
    expect(KOHLRAUSCH_BOARD.oled.status).toBe("pending-verification");
  });
});

describe("keymaps", () => {
  test("validates four complete independent layers", () => {
    const p = profile(); expect(validateProfile(p, ids)).toEqual([]);
    p.assignments[0]!.action = {kind:"keystroke",keys:["A"]};
    expect(p.assignments.find(a => a.layer === 2 && a.keyId === ids[0])!.action.kind).toBe("none");
  });
  test("round trips serialized profiles and rejects incomplete or malformed data", () => {
    const p = profile(); expect(parseProfile(serializeProfile(p), ids)).toEqual(p);
    p.assignments = p.assignments.slice(1);
    expect(validateProfile(p, ids)).toContain("Layer 1 must contain exactly nine assignments");
    expect(() => parseProfile("null", ids)).toThrow("Profile must be an object");
  });
  test("rejects unknown keys, duplicate assignments and invalid actions", () => {
    const p = profile(); p.assignments[0]!.keyId = "wrong";
    p.assignments[2]!.keyId = "wrong";
    (p.assignments[2] as any).action = {kind:"website",url:"javascript:alert(1)"};
    const errors = validateProfile(p, ids).join(" ");
    expect(errors).toContain("Unknown physical key"); expect(errors).toContain("Duplicate assignment"); expect(errors).toContain("invalid action data");
  });
});

describe("simulator and layers", () => {
  test("OLED preview alternates screens every five seconds and highlights the active layer", () => {
    expect(OLED_SCREEN_DURATION_MS).toBe(5000); expect(nextOledPreviewScreen("layer")).toBe("brand");
    expect(nextOledPreviewScreen("brand")).toBe("layer");
    expect(oledLayerBoxes(3).map(b => b.active)).toEqual([false,false,true,false]);
  });
  test("cycles LYR 1→2→3→4→1", () => {
    let layer:1|2|3|4 = 1; const seen:(1|2|3|4)[] = [layer];
    for (let i=0; i<4; i++) { layer=advanceLayer(layer); seen.push(layer); }
    expect(seen).toEqual([1,2,3,4,1]); expect(nextLayer(4)).toBe(1);
  });
  test("never reports mock as physically connected and refuses flash", async () => {
    const d = new MockDeviceTransport(); expect(await d.discover()).toBe(false);
    const status = await d.getStatus(); expect(isDeviceStatus(status)).toBe(true);
    expect(status.connected).toBe(false); expect(status.simulated).toBe(true);
    expect(isDeviceStatus({...status,connected:true})).toBe(false);
    expect((await d.requestFirmwareUpdate(new Uint8Array([1]))).accepted).toBe(false);
  });
  test("validates mock writes and preserves last good configuration after error", async () => {
    const d = new MockDeviceTransport(); const good = profile(); await d.writeKeymap(good);
    await expect(d.writeKeymap({bad:true})).rejects.toThrow();
    expect(await d.readKeymap()).toEqual(good);
  });
  test("macro sequence preserves steps and delays, rejects negative delay", async () => {
    expect(await executeMockMacro([{kind:"keystroke",delayMs:0},{kind:"delay",delayMs:50}])).toEqual(["step 1: keystroke","step 2: delay","wait 50ms"]);
    await expect(executeMockMacro([{kind:"bad",delayMs:-1}])).rejects.toThrow();
  });
});

describe("macro validation", () => {
  test("accepts typed website action and rejects malformed URL", () => {
    const m:MacroDefinition = {id:"m",name:"site",mode:"website",actions:[{kind:"website",url:"https://example.com",delayMs:0}]};
    expect(validateMacro(m)).toEqual([]); m.actions=[{kind:"website",url:"https://",delayMs:0}];
    expect(validateMacro(m)).toContain("Action 1 URL must use http or https");
  });
  test("supports sequences and rejects empty keystrokes or invalid mouse steps", () => {
    const m:MacroDefinition = {id:"m",name:"sequence",mode:"record",actions:[{kind:"keystroke",keys:["CTRL","P"],delayMs:0},{kind:"delay",delayMs:100}]};
    expect(validateMacro(m)).toEqual([]); m.actions=[{kind:"mouse",steps:[{kind:"bad"} as any],delayMs:0}];
    expect(validateMacro(m)).toContain("Action 1 has an invalid mouse step");
  });
});





