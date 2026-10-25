import { useRenderer, type JSX } from "@opentui/solid";
import { createSignal } from "solid-js";
import { MouseButton, type Renderable } from "@opentui/core";

type ButtonProps = {
  label: string;
  onPress: () => void;
  active?: boolean;
  disabled?: boolean;
  compact?: boolean;
  tone?: "violet" | "quiet" | "warning" | "success";
};
const toneColor = { violet: "#a46cf0", quiet: "#766982", warning: "#e5a84a", success: "#6cc9b0" } as const;

/** Cell-based press/release button. OpenTUI has down/up events, not a synthetic click event. */
export function TuiButton(props: ButtonProps): JSX.Element {
  const renderer = useRenderer();
  const [hovered, setHovered] = createSignal(false);
  let pressed = false;
  const color = () => props.disabled ? "#4b4450" : toneColor[props.tone ?? "violet"];
  const handleDown = (event: { type: string; button: number; stopPropagation(): void; preventDefault(): void }) => {
    if (event.type !== "down" || event.button !== MouseButton.LEFT || props.disabled) return;
    pressed = true;
    event.stopPropagation();
    event.preventDefault();
  };
  const handleUp = (event: { type: string; button: number; x: number; y: number; currentTarget: Renderable | null; stopPropagation(): void }) => {
    if (event.type !== "up" || event.button !== MouseButton.LEFT || !pressed) return;
    pressed = false;
    event.stopPropagation();
    const target = event.currentTarget;
    const inside = target !== null && event.x >= target.x && event.x < target.x + target.width && event.y >= target.y && event.y < target.y + target.height;
    if (inside && !props.disabled) props.onPress();
  };
  return <box
    flexDirection="row" alignItems="center" justifyContent="center" paddingX={props.compact ? 1 : 2} height={props.compact ? 1 : 3}
    border={props.active === true || hovered()} borderColor={hovered() ? "#e7c8ff" : color()}
    backgroundColor={hovered() ? "#2a1d38" : props.active ? "#21172c" : "#100d15"}
    onMouseDown={handleDown} onMouseUp={handleUp} onMouseDragEnd={() => { pressed = false; }}
    onMouseOver={() => { setHovered(true); renderer.setMousePointer("pointer"); }}
    onMouseOut={() => { setHovered(false); renderer.setMousePointer("default"); }}
  >
    <text height={1} fg={hovered() ? "#f4e6ff" : props.disabled ? "#655c6a" : "#d9cbe5"}>{props.label}</text>
  </box>;
}

export function SectionTitle(props: { eyebrow: string; title: string; detail?: string }): JSX.Element {
  return <box flexDirection="column" marginBottom={1}>
    <text height={1} fg="#9c77bb">{props.eyebrow.toUpperCase()}</text>
    <text height={1} fg="#f1e5f8">{props.title}</text>
    {props.detail && <text height={1} fg="#91859a">{props.detail}</text>}
  </box>;
}

export function Divider(): JSX.Element { return <text height={1} fg="#3a3042">────────────────────────────────────────────────────────────────────────</text>; }
