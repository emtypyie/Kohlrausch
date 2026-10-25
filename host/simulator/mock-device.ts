import type { DeviceTransport, DeviceStatus } from "../../packages/device-protocol/src/index.ts";
import { validateProfile } from "../../packages/config-schema/src/index.ts";
import { KOHLRAUSCH_BOARD } from "../../packages/board-definitions/index.ts";

export class MockDeviceTransport implements DeviceTransport {
  private layer: 1 | 2 | 3 | 4 = 1;
  private config: unknown = {};
  async discover(): Promise<boolean> { return false; }
  async getStatus(): Promise<DeviceStatus> {
    return { transport: "mock", connected: false, simulated: true, activeLayer: this.layer, message: "SIMULATOR ONLY · no physical USB device" };
  }
  async readKeymap(): Promise<unknown> { return structuredClone(this.config); }
  async writeKeymap(value: unknown): Promise<void> {
    if (!value || typeof value !== "object") throw new Error("Invalid mock keymap payload");
    const profile = value as import("../../packages/config-schema/src/index.ts").KeymapProfile;
    if (profile.boardId !== KOHLRAUSCH_BOARD.boardId) throw new Error("Keymap board ID mismatch");
    const errors = validateProfile(profile, KOHLRAUSCH_BOARD.keys.map(key => key.id));
    if (errors.length) throw new Error(errors.join("; "));
    this.config = structuredClone(value);
  }
  async readActiveLayer(): Promise<1 | 2 | 3 | 4> { return this.layer; }
  async requestFirmwareUpdate(_image: Uint8Array): Promise<{ accepted: boolean; reason: string }> {
    return { accepted: false, reason: "Firmware updates are disabled in simulator mode" };
  }
  setLayer(layer: 1 | 2 | 3 | 4): void { this.layer = layer; }
}

export function advanceLayer(layer: 1 | 2 | 3 | 4): 1 | 2 | 3 | 4 { return layer === 4 ? 1 : (layer + 1) as 1 | 2 | 3 | 4; }
export async function executeMockMacro(actions: { delayMs: number; kind: string }[]): Promise<string[]> {
  const events: string[] = [];
  for (const [index, action] of actions.entries()) {
    if (action.delayMs < 0) throw new Error(`Invalid delay at action ${index + 1}`);
    events.push(`step ${index + 1}: ${action.kind}`);
    if (action.delayMs) events.push(`wait ${action.delayMs}ms`);
  }
  return events;
}

