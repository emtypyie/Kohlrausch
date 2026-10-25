import { render } from "@opentui/solid";
import { App } from "./workspace.tsx";

const initialSection = process.argv.includes("--c") || process.argv.includes("--config") ? "Keymaps" : "Home";
await render(() => <App initialSection={initialSection} />, { useMouse: true, enableMouseMovement: true, targetFps: 30, exitOnCtrlC: false });
