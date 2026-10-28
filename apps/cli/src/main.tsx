import { render } from "@opentui/solid";
import { App } from "./workspace.tsx";

await render(() => <App initialSection="Home" />, { useMouse: true, enableMouseMovement: true, targetFps: 30, exitOnCtrlC: false });
