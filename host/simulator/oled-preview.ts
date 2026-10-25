import type { LayerNumber } from "../../packages/config-schema/src/index.ts";
export type OledPreviewScreen = "layer" | "brand";
export const OLED_SCREEN_DURATION_MS = 5000;
export function nextOledPreviewScreen(screen: OledPreviewScreen): OledPreviewScreen { return screen === "layer" ? "brand" : "layer"; }
export function oledLayerBoxes(active: LayerNumber): { layer: LayerNumber; active: boolean }[] {
  return ([1, 2, 3, 4] as const).map(layer => ({ layer, active: layer === active }));
}
