import { mkdir, readFile, readdir, rename, unlink, writeFile } from "node:fs/promises";
import { homedir } from "node:os";
import { join } from "node:path";
import { KOHLRAUSCH_BOARD } from "../../packages/board-definitions/index.ts";
import { parseProfile, serializeProfile, validateProfile, type KeyAction, type KeymapProfile, type LayerNumber } from "../../packages/config-schema/src/index.ts";

export function createDefaultProfile(profileId = "default", name = "Default"): KeymapProfile {
  return {
    schemaVersion: 1,
    profileId,
    name,
    boardId: KOHLRAUSCH_BOARD.boardId,
    assignments: [1, 2, 3, 4].flatMap(layer => KOHLRAUSCH_BOARD.keys.map(key => ({
      keyId: key.id,
      layer: layer as LayerNumber,
      action: { kind: "none" } as KeyAction,
    }))),
  };
}

export function getAssignment(profile: KeymapProfile, keyId: string, layer: LayerNumber): KeyAction {
  return profile.assignments.find(item => item.keyId === keyId && item.layer === layer)?.action ?? { kind: "none" };
}

export function withAssignment(profile: KeymapProfile, keyId: string, layer: LayerNumber, action: KeyAction): KeymapProfile {
  const assignments = profile.assignments.map(item => item.keyId === keyId && item.layer === layer ? { ...item, action } : item);
  return { ...profile, assignments };
}

export function nextLayer(layer: LayerNumber): LayerNumber { return layer === 4 ? 1 : (layer + 1) as LayerNumber; }

export class LocalProfileStore {
  readonly root: string;
  constructor(root = process.env.APPDATA ? join(process.env.APPDATA, "KohlrauschPrototype", "profiles") : join(homedir(), ".config", "kohlrausch-prototype", "profiles")) { this.root = root; }

  async load(): Promise<KeymapProfile[]> {
    await mkdir(this.root, { recursive: true });
    const files = (await readdir(this.root)).filter(file => file.endsWith(".json")).sort();
    if (files.length === 0) return [createDefaultProfile()];
    const profiles: KeymapProfile[] = [];
    for (const file of files) {
      const value = await readFile(join(this.root, file), "utf8");
      profiles.push(parseProfile(value, KOHLRAUSCH_BOARD.keys.map(key => key.id)));
    }
    return profiles;
  }

  async save(profile: KeymapProfile): Promise<void> {
    const errors = validateProfile(profile, KOHLRAUSCH_BOARD.keys.map(key => key.id));
    if (errors.length) throw new Error(errors.join("; "));
    if (!/^[a-z0-9][a-z0-9_-]*$/i.test(profile.profileId)) throw new Error("Profile ID may contain only letters, numbers, hyphens and underscores");
    await mkdir(this.root, { recursive: true });
    const path = join(this.root, `${profile.profileId}.json`);
    const temporaryPath = join(this.root, `.${profile.profileId}.${Date.now()}.tmp`);
    await writeFile(temporaryPath, serializeProfile(profile), "utf8");
    await rename(temporaryPath, path);
  }

  async delete(profileId: string): Promise<void> {
    if (!/^[a-z0-9][a-z0-9_-]*$/i.test(profileId)) throw new Error("Invalid profile ID");
    try { await unlink(join(this.root, `${profileId}.json`)); }
    catch (error) { if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error; }
  }
}
