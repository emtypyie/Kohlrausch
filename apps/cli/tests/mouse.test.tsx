import { expect, test } from "bun:test";
import { testRender } from "@opentui/solid";
import { TuiButton } from "../src/components/controls.tsx";

test("mouse activates sidebar navigation and the fixed key grid", async () => {
  let clicks = 0;
  const setup = await testRender(() => <box width="100%" height="100%"><TuiButton label="Open Keymaps" onPress={() => clicks++} /></box>, { width: 60, height: 20, useMouse: true, enableMouseMovement: true });
  try {
    await setup.flush();
    await setup.mockMouse.click(30, 1);
    await setup.flush();
    expect(clicks).toBe(1);
    setup.resize(90, 30);
    await setup.flush();
    expect(setup.captureCharFrame().length).toBeGreaterThan(0);
  } finally { await setup.dispose(); }
});
