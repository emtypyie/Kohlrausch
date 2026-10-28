/* Kohlrausch Konfig - Tauri frontend controller. */
(function () {
  "use strict";

  const KEYS = (window.KohlrauschScene && window.KohlrauschScene.KEYS) || [];
  const keyById = Object.fromEntries(KEYS.map((k) => [k.id, k]));
  const tauri = window.__TAURI__;

  const MOUSE_LABELS = {
    leftClick: "Left click",
    rightClick: "Right click",
    doubleClick: "Double click",
    scrollUp: "Scroll up",
    scrollDown: "Scroll down",
  };

  const state = {
    section: "Keymaps",
    layer: 1,
    selectedKey: "logical-1-1",
    editorKind: "keyChord",
    capturingShortcut: false,
    editorDirty: false,
    editorError: "",
    actionValue: "",
    actionExtra: "",
    actionArguments: "[]",
    activeProfile: null,
    profileName: "Default",
    profiles: [],
    selectedProfileId: null,
    profileArchiveConfirm: false,
    macros: [],
    macroDraftId: null,
    macroSteps: [],
    macroEventIndex: -1,
    macroRecording: false,
    macroMode: "Key chord",
    macroTiming: "Recorded delay",
    macroLoop: "Once",
    macroLoopCount: 3,
    configFiles: [],
    selectedConfigFile: -1,
  };

  const $ = (sel) => document.querySelector(sel);
  const $$ = (sel) => Array.from(document.querySelectorAll(sel));
  const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));

  function el(tag, className, text) {
    const node = document.createElement(tag);
    if (className) node.className = className;
    if (text !== undefined) node.textContent = text;
    return node;
  }

  function message(error) {
    if (error == null) return "Unknown error";
    if (typeof error === "string") return error;
    if (error.message) return error.message;
    return String(error);
  }

  async function invoke(command, args) {
    if (!tauri || !tauri.core || typeof tauri.core.invoke !== "function") {
      throw new Error("Tauri backend unavailable");
    }
    return tauri.core.invoke(command, args || {});
  }

  /* ---------------- navigation ---------------- */
  function showSection(name) {
    state.section = name;
    $$(".section").forEach((s) => s.classList.toggle("hidden", s.dataset.section !== name));
    $$(".nav-item").forEach((n) => {
      const nav = n.dataset.nav;
      const active = nav === name || (nav === "Profiles" && name === "Profiles");
      n.classList.toggle("active", active);
    });
    if (name !== "Profiles") closeProfiles();
    if (name === "Keymaps") startEntrance();
  }

  $$(".nav-item").forEach((item) => {
    item.addEventListener("click", () => {
      const nav = item.dataset.nav;
      if (nav === "Profiles") {
        toggleProfiles();
        return;
      }
      showSection(nav);
    });
  });

  /* ---------------- window controls ---------------- */
  function currentWindow() {
    try {
      return tauri && tauri.window ? tauri.window.getCurrentWindow() : null;
    } catch (_) {
      return null;
    }
  }
  $$(".win-btn").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const win = currentWindow();
      if (!win) return;
      try {
        if (btn.dataset.win === "min") await win.minimize();
        else if (btn.dataset.win === "max") await win.toggleMaximize();
        else await win.close();
      } catch (error) {
        console.error(error);
      }
    });
  });

  /* ---------------- profile and action mapping ---------------- */
  function physicalIdFor(logicalId) {
    const key = keyById[logicalId];
    return key ? key.physicalId : logicalId;
  }

  function macroName(macroId) {
    const macro = state.macros.find((item) => item.macroId === macroId);
    return macro ? macro.name : "";
  }

  function rawAssignmentFor(layer, keyId) {
    if (!state.activeProfile) return null;
    const physical = physicalIdFor(keyId);
    const assignment = (state.activeProfile.assignments || []).find(
      (item) => item.keyId === physical && item.layer === layer
    );
    return assignment ? assignment.action : null;
  }

  function editorForAction(action) {
    if (!action || action.kind === "none") {
      return { kind: "none", value: "", extra: "", args: "[]" };
    }
    switch (action.kind) {
      case "keyChord":
        return { kind: "keyChord", value: (action.keys || []).join("+"), extra: "", args: "[]" };
      case "macroReference":
        return { kind: "macro", value: action.macroId || "", extra: "", args: "[]" };
      case "website":
        return { kind: "website", value: action.url || "", extra: action.label || "", args: "[]" };
      case "application":
        return {
          kind: "application",
          value: action.name || "",
          extra: action.executablePath || action.applicationId || "",
          args: JSON.stringify(action.arguments || []),
        };
      case "mouseAction":
        return {
          kind: "mouseAction",
          value: MOUSE_LABELS[action.action] || "Left click",
          extra: "",
          args: "[]",
        };
      default:
        return { kind: "none", value: "", extra: "", args: "[]" };
    }
  }

  function assignmentLabel(action) {
    if (!action || action.kind === "none") return "Unassigned";
    switch (action.kind) {
      case "keyChord":
        return (action.keys || []).join("+") || "Unassigned";
      case "macroReference": {
        const name = macroName(action.macroId);
        return name ? "Macro · " + name : "Missing macro · " + (action.macroId || "");
      }
      case "website":
        return action.url || "Website";
      case "application":
        return "Application · " + (action.name || "unknown");
      case "mouseAction":
        return MOUSE_LABELS[action.action] || "Mouse action";
      default:
        return "Unassigned";
    }
  }

  function updateMacroDetail(action) {
    const detail = $("#macro-detail");
    if (action && action.kind === "macroReference") {
      const macro = state.macros.find((item) => item.macroId === action.macroId);
      detail.textContent = macro
        ? macro.name + " · " + macro.steps.length + " step(s)"
        : "Missing macro · " + action.macroId;
    } else {
      detail.textContent = "Select a saved macro";
    }
  }

  /* ---------------- layer control ---------------- */
  function setLayer(layer) {
    state.layer = layer;
    $$(".layer-btn").forEach((b) => b.classList.toggle("active", Number(b.dataset.layer) === layer));
    $("#oled-layer").textContent = String(layer);
    $$(".oled-layer").forEach((b, index) => b.classList.toggle("active", index + 1 === layer));
    loadAssignment();
    updateInspector();
    refreshSelectionVisual();
  }
  $$(".layer-btn").forEach((b) => b.addEventListener("click", () => setLayer(Number(b.dataset.layer))));

  /* ---------------- inspector ---------------- */
  function loadAssignment() {
    const action = rawAssignmentFor(state.layer, state.selectedKey);
    const editor = editorForAction(action);
    state.capturingShortcut = false;
    state.editorError = "";
    state.editorDirty = false;
    state.actionValue = editor.value;
    state.actionExtra = editor.extra;
    state.actionArguments = editor.args;
    setEditorKind(editor.kind === "none" ? "keyChord" : editor.kind, false);
    updateMacroDetail(action);
    syncEditorInputs();
  }

  function setEditorKind(kind, resetValues) {
    state.editorKind = kind;
    if (resetValues !== false) {
      state.actionValue = kind === "mouseAction" ? "Left click" : "";
      state.actionExtra = "";
      state.actionArguments = "[]";
      state.editorDirty = true;
      state.editorError = "";
    }
    $$(".insp-tab").forEach((t) => t.classList.toggle("active", t.dataset.kind === kind));
    $$("[data-pane]").forEach((p) => p.classList.toggle("hidden", p.dataset.pane !== kind));
    syncEditorInputs();
  }

  function syncEditorInputs() {
    $("#shortcut-display").textContent = state.capturingShortcut
      ? "Press shortcut…"
      : state.actionValue || "No shortcut captured";
    $("#website-url").value = state.editorKind === "website" ? state.actionValue : $("#website-url").value;
    $("#website-label").value = state.editorKind === "website" ? state.actionExtra : $("#website-label").value;
    $("#app-name").value = state.editorKind === "application" ? state.actionValue : $("#app-name").value;
    $("#app-path").value = state.editorKind === "application" ? state.actionExtra : $("#app-path").value;
    $("#app-args").value = state.editorKind === "application" ? state.actionArguments : $("#app-args").value;
    $("#mouse-kind").value = state.actionValue || "Left click";
    const saveState = $("#insp-save-state");
    saveState.textContent = state.editorDirty
      ? "UNSAVED CHANGES"
      : "Saved assignment · " + state.profileName;
    saveState.classList.toggle("dirty", state.editorDirty);
    const error = $("#insp-error");
    error.textContent = state.editorError || "";
    error.classList.toggle("hidden", !state.editorError);
    $("#capture-shortcut").textContent = state.capturingShortcut ? "Press shortcut…" : "Capture shortcut";
  }

  function updateInspector() {
    const key = keyById[state.selectedKey] || { label: "1.1", physicalReference: "3.1", physicalId: "key-3-1" };
    $("#insp-title").textContent = key.label + "  /  LAYER " + state.layer;
    $("#insp-key-id").textContent = key.physicalId;
    $("#insp-physical").textContent = key.physicalReference + "  ·  " + key.physicalId;
    $("#insp-assignment").textContent = assignmentLabel(rawAssignmentFor(state.layer, state.selectedKey));
  }

  $$(".insp-tab").forEach((tab) => {
    tab.addEventListener("click", () => {
      setEditorKind(tab.dataset.kind, true);
      updateMacroDetail(null);
      syncEditorInputs();
    });
  });

  $("#capture-shortcut").addEventListener("click", () => {
    state.capturingShortcut = true;
    state.editorDirty = true;
    syncEditorInputs();
  });

  $("#clear-shortcut").addEventListener("click", () => {
    state.actionValue = "";
    state.editorDirty = true;
    syncEditorInputs();
  });
  $("#preview-shortcut").addEventListener("click", () => setStatus("Preview only · no input sent"));

  $("#website-url").addEventListener("input", (e) => { state.actionValue = e.target.value; markDirty(); });
  $("#website-label").addEventListener("input", (e) => { state.actionExtra = e.target.value; markDirty(); });
  $("#app-name").addEventListener("input", (e) => { state.actionValue = e.target.value; markDirty(); });
  $("#app-path").addEventListener("input", (e) => { state.actionExtra = e.target.value; markDirty(); });
  $("#app-args").addEventListener("input", (e) => { state.actionArguments = e.target.value; markDirty(); });
  $("#mouse-kind").addEventListener("change", (e) => { state.actionValue = e.target.value; markDirty(); });
  $("#inspector-macro").addEventListener("change", (e) => {
    state.actionValue = e.target.value;
    updateMacroDetail({ kind: "macroReference", macroId: e.target.value });
    markDirty();
  });

  function markDirty() {
    state.editorDirty = true;
    syncEditorInputs();
  }

  function editorError(text) {
    state.editorError = text;
    state.editorDirty = true;
    syncEditorInputs();
    setStatus("Assignment not saved");
  }

  async function saveAssignment() {
    if (!state.activeProfile) {
      editorError("No active profile is loaded.");
      return;
    }
    if (state.editorKind === "website" && state.actionValue && !/^https?:\/\//i.test(state.actionValue)) {
      editorError("Website actions must use an http:// or https:// address.");
      return;
    }
    let parsedArguments = "[]";
    if (state.editorKind === "application") {
      try {
        const parsed = JSON.parse(state.actionArguments || "[]");
        if (!Array.isArray(parsed) || parsed.some((value) => typeof value !== "string")) {
          throw new Error("Arguments must be a JSON array of strings");
        }
        parsedArguments = JSON.stringify(parsed);
      } catch (_) {
        editorError("Arguments must be a JSON array of strings.");
        return;
      }
    }
    state.editorError = "";
    try {
      const workspace = await invoke("save_assignment", {
        input: {
          profileId: state.activeProfile.profileId,
          keyId: physicalIdFor(state.selectedKey),
          layer: state.layer,
          kind: state.editorKind,
          value: state.actionValue,
          extra: state.actionExtra,
          arguments: parsedArguments,
        },
      });
      applyWorkspace(workspace);
      const label = (keyById[state.selectedKey] || {}).label || state.selectedKey;
      setStatus("Saved " + label + " assignment · " + state.profileName);
    } catch (error) {
      editorError(message(error));
    }
  }

  async function clearAssignment() {
    if (!state.activeProfile) return;
    state.editorError = "";
    try {
      const workspace = await invoke("save_assignment", {
        input: {
          profileId: state.activeProfile.profileId,
          keyId: physicalIdFor(state.selectedKey),
          layer: state.layer,
          kind: "none",
          value: "",
          extra: "",
          arguments: "[]",
        },
      });
      applyWorkspace(workspace);
      setStatus("Cleared assignment · " + state.profileName);
    } catch (error) {
      editorError(message(error));
    }
  }

  $("#save-action").addEventListener("click", saveAssignment);
  $("#clear-action").addEventListener("click", clearAssignment);
  $("#cancel-action").addEventListener("click", () => { loadAssignment(); updateInspector(); });

  window.addEventListener("keydown", (e) => {
    if (!state.capturingShortcut) return;
    if (e.key === "Escape") { state.capturingShortcut = false; syncEditorInputs(); return; }
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
    e.preventDefault();
    let combo = "";
    if (e.ctrlKey) combo += "CTRL+";
    if (e.shiftKey) combo += "SHIFT+";
    if (e.altKey) combo += "ALT+";
    if (e.metaKey) combo += "META+";
    let key = e.key === " " ? "SPACE" : e.key;
    if (key.length === 1) key = key.toUpperCase();
    combo += key;
    state.actionValue = combo;
    state.capturingShortcut = false;
    state.editorDirty = true;
    syncEditorInputs();
  });

  /* ---------------- scene ---------------- */
  const sceneWrap = $("#scene-wrap");
  const sceneFrame = $("#scene-frame");
  const sceneInner = $("#scene-inner");
  const sceneCanvas = $("#scene-canvas");
  let sceneHandle = null;
  let entranceTimer = null;

  async function loadScene() {
    if (!window.KohlrauschScene) return;
    try {
      sceneHandle = await window.KohlrauschScene.render(sceneCanvas);
      layoutScene();
      refreshSelectionVisual();
    } catch (error) {
      console.error("Scene render failed", error);
    }
  }

  function layoutScene() {
    const w = sceneWrap.clientWidth;
    const h = sceneWrap.clientHeight;
    if (!w || !h) return;
    const frameW = Math.min(w, (h * 840) / 540);
    const frameH = (frameW * 540) / 840;
    sceneFrame.style.width = frameW + "px";
    sceneFrame.style.height = frameH + "px";
    sceneInner.style.transform = "scale(" + frameW / 840 + ")";
  }

  new ResizeObserver(layoutScene).observe(sceneWrap);

  function sceneCoords(event) {
    const rect = sceneCanvas.getBoundingClientRect();
    return {
      x: ((event.clientX - rect.left) / rect.width) * 840,
      y: ((event.clientY - rect.top) / rect.height) * 540,
    };
  }

  function anchorFor(keyId) {
    if (!sceneHandle) return null;
    return sceneHandle.anchorFor(keyId);
  }

  function refreshSelectionVisual() {
    const box = $("#selection-box");
    const anchor = anchorFor(state.selectedKey);
    if (!anchor) { box.style.display = "none"; return; }
    box.style.display = "block";
    box.style.left = clamp(anchor[0] - 23, 3, 840 - 49) + "px";
    box.style.top = clamp(anchor[1] - 23, 3, 540 - 49) + "px";
  }

  function updateHover(event) {
    if (!sceneHandle) return;
    const { x, y } = sceneCoords(event);
    const keyId = sceneHandle.keyAt(x, y);
    const card = $("#hover-card");
    if (!keyId) { card.style.display = "none"; sceneWrap.style.cursor = "default"; return; }
    const key = keyById[keyId];
    const anchor = sceneHandle.anchorFor(keyId) || [x, y];
    sceneWrap.style.cursor = "pointer";
    const assignment = assignmentLabel(rawAssignmentFor(state.layer, keyId));
    $("#hover-title").textContent = "KEY " + key.physicalReference + "   ·   LAYER " + state.layer;
    $("#hover-physical").textContent = "Physical  " + key.physicalReference;
    $("#hover-matrix").textContent = "Matrix  unverified";
    $("#hover-gpio").textContent = "GPIO  unverified";
    $("#hover-action").textContent = "Action  " + assignment;
    const cardW = 240, cardH = 132;
    const left = clamp(anchor[0] + (anchor[0] < 420 ? 82 : -cardW - 82), 8, 840 - cardW - 8);
    const top = clamp(anchor[1] + (anchor[1] < 175 ? 20 : -cardH - 14), 8, 540 - cardH - 8);
    card.style.left = left + "px";
    card.style.top = top + "px";
    card.style.display = "flex";
  }

  sceneWrap.addEventListener("mousemove", updateHover);
  sceneWrap.addEventListener("mouseleave", () => { $("#hover-card").style.display = "none"; });
  sceneWrap.addEventListener("click", (event) => {
    if (!sceneHandle) return;
    const { x, y } = sceneCoords(event);
    const keyId = sceneHandle.keyAt(x, y);
    if (keyId) selectKey(keyId);
  });

  function selectKey(keyId) {
    state.selectedKey = keyId;
    loadAssignment();
    updateInspector();
    refreshSelectionVisual();
  }

  function startEntrance() {
    if (entranceTimer) clearInterval(entranceTimer);
    let seq = 0;
    sceneFrame.classList.add("entering");
    applyReveal(seq);
    entranceTimer = setInterval(() => {
      seq += 1;
      if (seq >= 3) sceneFrame.classList.remove("entering");
      applyReveal(seq);
      if (seq >= 10) { clearInterval(entranceTimer); entranceTimer = null; }
    }, 110);
  }

  function applyReveal(seq) {
    const set = (id, value) => { const node = document.getElementById(id); if (node) node.style.opacity = value; };
    const lyr = clamp((seq - 6) / 2, 0, 1);
    const rp = clamp((seq - 7) / 2, 0, 1);
    const keys = clamp((seq - 8) / 2, 0, 1);
    set("path-lyr-1", seq >= 7 ? lyr : 0);
    set("path-lyr-2", seq >= 7 ? lyr : 0);
    set("callout-lyr", seq >= 7 ? lyr : 0);
    set("path-rp-1", seq >= 8 ? rp : 0);
    set("path-rp-2", seq >= 8 ? rp : 0);
    set("callout-rp", seq >= 8 ? rp : 0);
    set("path-keys-1", seq >= 9 ? keys : 0);
    set("path-keys-2", seq >= 9 ? keys : 0);
    set("callout-keys", seq >= 9 ? keys : 0);
  }

  /* ---------------- status ---------------- */
  function setStatus(text) { $("#status-message").textContent = text; }

  /* ---------------- macros ---------------- */
  function macroStepLabel(step) {
    return step.value + (step.delayMs ? "  ·  " + step.delayMs + "ms" : "");
  }

  function macroStepEditor(action) {
    const editor = editorForAction(action);
    const mode = editor.kind === "keyChord" ? "Key chord"
      : editor.kind === "website" ? "Website"
      : editor.kind === "application" ? "Software"
      : editor.kind === "mouseAction" ? "Clicks"
      : "Key chord";
    return { mode, value: editor.value, delayMs: 0 };
  }

  function loopEditor(loop) {
    if (!loop || loop.mode === "once") return { label: "Once", count: 3 };
    if (loop.mode === "untilStop") return { label: "Until stop", count: 3 };
    if (loop.mode === "untilAnyClick") return { label: "Until any click", count: 3 };
    if (loop.mode === "count") return { label: "Loop count", count: loop.count || 3 };
    return { label: "Once", count: 3 };
  }

  function renderMacroList() {
    const list = $("#macro-list");
    list.innerHTML = "";
    if (state.macros.length === 0) {
      list.appendChild(el("div", "macro-empty", "No saved macros"));
    } else {
      state.macros.forEach((macro) => {
        const row = el(
          "div",
          "macro-row" + (macro.macroId === state.macroDraftId ? " selected" : ""),
          macro.name
        );
        row.addEventListener("click", () => selectMacro(macro.macroId));
        list.appendChild(row);
      });
    }
    $("#delete-macro").disabled = !state.macroDraftId;

    const macroSelect = $("#inspector-macro");
    macroSelect.innerHTML = "";
    if (state.macros.length === 0) {
      macroSelect.appendChild(new Option("No saved macros", ""));
    } else {
      state.macros.forEach((macro) => {
        macroSelect.appendChild(
          new Option(macro.name + " · " + macro.steps.length + " step(s)", macro.macroId)
        );
      });
    }
    const action = rawAssignmentFor(state.layer, state.selectedKey);
    if (action && action.kind === "macroReference") macroSelect.value = action.macroId;
  }

  function selectMacro(macroId) {
    const macro = state.macros.find((item) => item.macroId === macroId);
    if (!macro) return;
    state.macroDraftId = macro.macroId;
    state.macroSteps = macro.steps.map((step) => {
      const editor = macroStepEditor(step.action);
      editor.delayMs = step.delayMs || 0;
      return editor;
    });
    state.macroEventIndex = -1;
    state.macroTiming = macro.timing === "recorded" ? "Recorded delay" : "Fixed delay";
    const loop = loopEditor(macro.loopMode);
    state.macroLoop = loop.label;
    state.macroLoopCount = loop.count;
    $("#macro-name").value = macro.name;
    $("#macro-timing").value = state.macroTiming;
    $("#macro-fixed-delay").value = String(macro.fixedDelayMs || 0);
    $("#macro-fixed-delay").classList.toggle("hidden", state.macroTiming !== "Fixed delay");
    $("#macro-loop").value = state.macroLoop;
    $("#macro-loop-count").value = String(state.macroLoopCount);
    $("#macro-loop-count").classList.toggle("hidden", state.macroLoop !== "Loop count");
    hideMacroError();
    renderMacroList();
    renderEventList();
    setStatus("Selected macro “" + macro.name + "”");
  }

  function renderEventList() {
    const list = $("#event-list");
    list.innerHTML = "";
    if (state.macroSteps.length === 0) {
      const empty = el("div", "event-empty");
      const thumbs = el("div", "thumbs");
      thumbs.innerHTML = '<span class="mini-key hot"></span><span class="mini-sep"></span><span class="mini-key"></span><span class="mini-sep"></span><span class="mini-key"></span>';
      empty.appendChild(thumbs);
      empty.appendChild(el("div", "e-title", "No macro events yet"));
      empty.appendChild(el("div", "e-sub", "Record keys or add an event below"));
      list.appendChild(empty);
    } else {
      state.macroSteps.forEach((step, index) => {
        const row = el("div", "event-row" + (index === state.macroEventIndex ? " selected" : ""));
        row.appendChild(el("span", "col-no", String(index)));
        row.appendChild(el("span", "col-type", step.mode.toUpperCase()));
        row.appendChild(el("span", "col-value", macroStepLabel(step)));
        row.addEventListener("click", () => { state.macroEventIndex = index; renderEventList(); });
        list.appendChild(row);
      });
    }
    $("#remove-event").disabled = state.macroEventIndex < 0;
    $("#clear-events").disabled = state.macroSteps.length === 0;
    $("#macro-preview").textContent = state.macroSteps.length
      ? state.macroSteps.length + " event(s) · " + macroStepLabel(state.macroSteps[0]) +
        (state.macroSteps.length > 1 ? " …" : "")
      : "No actions saved";
  }

  function showMacroError(text) {
    $("#macro-error").textContent = text;
    $("#macro-error").classList.remove("hidden");
  }

  function hideMacroError() {
    $("#macro-error").textContent = "";
    $("#macro-error").classList.add("hidden");
  }

  $("#macro-mode").addEventListener("change", (e) => {
    state.macroMode = e.target.value;
    const value = $("#macro-value");
    value.placeholder = state.macroMode === "Website" ? "https://…"
      : state.macroMode === "Software" ? "Application name"
      : state.macroMode === "Clicks" ? "Left click" : "Ctrl+Shift+P";
  });

  $("#add-event").addEventListener("click", () => {
    const value = $("#macro-value").value.trim();
    const delayText = $("#macro-delay").value.trim() || "0";
    const delayMs = Number(delayText);
    if (!value) { showMacroError("Add an action value first."); return; }
    if (!Number.isInteger(delayMs) || delayMs < 0 || delayMs > 600000) {
      showMacroError("Delay must be a whole number from 0 to 600000 ms.");
      return;
    }
    hideMacroError();
    state.macroSteps.push({ mode: state.macroMode, value, delayMs });
    state.macroEventIndex = state.macroSteps.length - 1;
    $("#macro-value").value = "";
    renderEventList();
  });

  $("#remove-event").addEventListener("click", () => {
    if (state.macroEventIndex < 0) return;
    state.macroSteps.splice(state.macroEventIndex, 1);
    state.macroEventIndex = Math.min(state.macroEventIndex, state.macroSteps.length - 1);
    renderEventList();
  });
  $("#clear-events").addEventListener("click", () => {
    state.macroSteps = [];
    state.macroEventIndex = -1;
    renderEventList();
  });

  $("#toggle-record").addEventListener("click", () => {
    state.macroRecording = !state.macroRecording;
    const btn = $("#toggle-record");
    btn.classList.toggle("active", state.macroRecording);
    if (state.macroRecording) {
      btn.lastChild.textContent = " Stop recording";
      const label = $("#macro-recording-state");
      label.textContent = "● RECORDING · APP FOCUSED";
      label.classList.add("recording");
    } else {
      btn.lastChild.textContent = " Start recording";
      const label = $("#macro-recording-state");
      label.textContent = "LOCAL MACRO LIBRARY";
      label.classList.remove("recording");
    }
    $("#macro-name").disabled = state.macroRecording;
    $("#macro-value").disabled = state.macroRecording;
    $("#add-event").disabled = state.macroRecording;
    $("#save-macro").disabled = state.macroRecording;
  });

  window.addEventListener("keydown", (e) => {
    if (!state.macroRecording) return;
    if (["Control", "Shift", "Alt", "Meta"].includes(e.key)) return;
    if (e.target && (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA")) return;
    e.preventDefault();
    let combo = "";
    if (e.ctrlKey) combo += "CTRL+";
    if (e.shiftKey) combo += "SHIFT+";
    if (e.altKey) combo += "ALT+";
    if (e.metaKey) combo += "META+";
    combo += e.key === " " ? "SPACE" : e.key === "+" ? "PLUS" : e.key;
    state.macroSteps.push({ mode: "Key chord", value: combo, delayMs: 0 });
    state.macroEventIndex = state.macroSteps.length - 1;
    renderEventList();
  });

  $("#new-macro").addEventListener("click", () => {
    state.macroDraftId = null;
    state.macroSteps = [];
    state.macroEventIndex = -1;
    state.macroTiming = "Recorded delay";
    state.macroLoop = "Once";
    state.macroLoopCount = 3;
    $("#macro-fixed-delay").value = "0";
    $("#macro-fixed-delay").classList.add("hidden");
    $("#macro-timing").value = state.macroTiming;
    $("#macro-loop").value = state.macroLoop;
    $("#macro-loop-count").value = "3";
    $("#macro-loop-count").classList.add("hidden");
    hideMacroError();
    renderMacroList();
    renderEventList();
    $("#macro-name").focus();
  });

  $("#delete-macro").addEventListener("click", async () => {
    if (!state.macroDraftId) return;
    const name = macroName(state.macroDraftId);
    try {
      const workspace = await invoke("delete_macro", { macroId: state.macroDraftId });
      state.macroDraftId = null;
      state.macroSteps = [];
      state.macroEventIndex = -1;
      applyWorkspace(workspace);
      renderEventList();
      setStatus("Deleted macro “" + name + "”");
    } catch (error) {
      showMacroError(message(error));
    }
  });

  $("#save-macro").addEventListener("click", async () => {
    const name = $("#macro-name").value.trim();
    if (!name) {
      showMacroError("A macro name is required.");
      return;
    }
    if (state.macroSteps.length === 0) {
      showMacroError("Add at least one event before saving.");
      return;
    }
    const fixedText = $("#macro-fixed-delay").value.trim() || "0";
    const fixedDelayMs = Number(fixedText);
    if (!Number.isInteger(fixedDelayMs) || fixedDelayMs < 0 || fixedDelayMs > 600000) {
      showMacroError("Fixed delay must be a whole number from 0 to 600000 ms.");
      return;
    }
    const loopCount = Number($("#macro-loop-count").value.trim() || "1");
    if (state.macroLoop === "Loop count" && (!Number.isInteger(loopCount) || loopCount < 1 || loopCount > 10000)) {
      showMacroError("Loop count must be a whole number from 1 to 10,000.");
      return;
    }
    hideMacroError();
    try {
      const result = await invoke("save_macro", {
        draft: {
          name,
          macroId: state.macroDraftId,
          steps: state.macroSteps,
          timing: state.macroTiming,
          fixedDelayMs,
          loopMode: state.macroLoop,
          loopCount: Number.isInteger(loopCount) ? loopCount : 1,
        },
      });
      applyWorkspace(result.workspace);
      selectMacro(result.macroDefinition.macroId);
      setStatus("Saved macro “" + result.macroDefinition.name + "” · local library");
    } catch (error) {
      showMacroError(message(error));
    }
  });

  $("#macro-timing").addEventListener("change", (e) => {
    state.macroTiming = e.target.value;
    $("#macro-fixed-delay").classList.toggle("hidden", state.macroTiming !== "Fixed delay");
  });
  $("#macro-loop").addEventListener("change", (e) => {
    state.macroLoop = e.target.value;
    $("#macro-loop-count").classList.toggle("hidden", state.macroLoop !== "Loop count");
  });
  $("#open-macro-editor").addEventListener("click", () => showSection("Macros"));

  /* ---------------- profiles ---------------- */
  function toggleProfiles() {
    const popup = $("#profiles-popup");
    const open = popup.classList.contains("hidden");
    if (open) { openProfiles(); } else { closeProfiles(); }
  }
  function openProfiles() {
    $("#profiles-popup").classList.remove("hidden");
    renderProfiles();
    $$(".nav-item").forEach((n) => n.classList.toggle("active", n.dataset.nav === "Profiles"));
  }
  function closeProfiles() {
    $("#profiles-popup").classList.add("hidden");
    state.profileArchiveConfirm = false;
    $("#profile-archive-zone").classList.add("hidden");
    $("#profile-normal-actions").classList.remove("hidden");
    if (state.section !== "Profiles") {
      $$(".nav-item").forEach((n) => n.classList.toggle("active", n.dataset.nav === state.section));
    }
  }
  function renderProfiles() {
    const list = $("#profile-list");
    list.innerHTML = "";
    if (state.profiles.length === 0) {
      list.appendChild(el("div", "macro-empty", "No profiles found"));
    } else {
      state.profiles.forEach((profile) => {
        const row = el(
          "button",
          "action-btn pp-row" + (profile.profileId === state.selectedProfileId ? " active" : ""),
          profile.name
        );
        row.addEventListener("click", () => {
          state.selectedProfileId = profile.profileId;
          state.profileArchiveConfirm = false;
          renderProfiles();
          updateProfileActions();
        });
        list.appendChild(row);
      });
    }
    updateProfileActions();
  }
  function updateProfileActions() {
    const valid = state.profiles.some((profile) => profile.profileId === state.selectedProfileId);
    $("#switch-profile").disabled = !valid;
    $("#delete-profile").disabled = !valid || state.profiles.length < 2;
  }
  $("#profiles-close").addEventListener("click", closeProfiles);
  $("#create-profile").addEventListener("click", async () => {
    const name = $("#new-profile-name").value.trim();
    if (!name) return;
    try {
      const workspace = await invoke("create_profile", { name });
      $("#new-profile-name").value = "";
      applyWorkspace(workspace, { reloadConfig: true });
      setStatus("Created profile “" + workspace.activeProfile.name + "”");
    } catch (error) {
      setStatus("Profile not created: " + message(error));
    }
  });
  $("#switch-profile").addEventListener("click", async () => {
    if (!state.selectedProfileId) return;
    try {
      const workspace = await invoke("switch_profile", { profileId: state.selectedProfileId });
      applyWorkspace(workspace, { reloadConfig: true });
      closeProfiles();
      setStatus("Switched to profile “" + workspace.activeProfile.name + "”");
    } catch (error) {
      setStatus("Profile not switched: " + message(error));
    }
  });
  $("#delete-profile").addEventListener("click", () => {
    if (!state.selectedProfileId) return;
    state.profileArchiveConfirm = true;
    $("#profile-archive-zone").classList.remove("hidden");
    $("#profile-normal-actions").classList.add("hidden");
  });
  $("#archive-profile").addEventListener("click", async () => {
    if (!state.selectedProfileId) return;
    try {
      const workspace = await invoke("archive_profile", { profileId: state.selectedProfileId });
      state.profileArchiveConfirm = false;
      $("#profile-archive-zone").classList.add("hidden");
      $("#profile-normal-actions").classList.remove("hidden");
      applyWorkspace(workspace, { reloadConfig: true });
      setStatus("Profile archived · active “" + workspace.activeProfile.name + "”");
    } catch (error) {
      setStatus("Profile not archived: " + message(error));
    }
  });
  $("#keep-profile").addEventListener("click", () => {
    state.profileArchiveConfirm = false;
    $("#profile-archive-zone").classList.add("hidden");
    $("#profile-normal-actions").classList.remove("hidden");
  });

  /* ---------------- workspace binding ---------------- */
  function applyWorkspace(workspace, options) {
    if (!workspace) return;
    const previousActive = state.activeProfile ? state.activeProfile.profileId : null;
    state.activeProfile = workspace.activeProfile;
    state.profileName = workspace.activeProfile.name;
    state.profiles = workspace.profiles || [];
    state.macros = (workspace.macros && workspace.macros.macros) || [];
    if (
      previousActive !== workspace.activeProfile.profileId ||
      !state.profiles.some((profile) => profile.profileId === state.selectedProfileId)
    ) {
      state.selectedProfileId = workspace.activeProfile.profileId;
    }
    $("#active-profile-name").textContent = state.profileName;
    $$(".profile-inline").forEach((node) => { node.textContent = state.profileName; });
    renderProfiles();
    renderMacroList();
    loadAssignment();
    updateInspector();
    refreshSelectionVisual();
    if (workspace.warning) setStatus(workspace.warning);
    if (options && options.reloadConfig) loadConfigFiles();
  }

  /* ---------------- about accordions ---------------- */
  $$(".accordion-head").forEach((head) => {
    head.addEventListener("click", () => {
      const kind = head.dataset.accordion;
      const body = document.getElementById("body-" + kind);
      const chevron = document.getElementById("chevron-" + kind);
      const open = body.classList.toggle("hidden");
      chevron.textContent = open ? "+" : "−";
    });
  });

  /* ---------------- config studio ---------------- */
  function renderConfig() {
    const list = $("#config-file-list");
    list.innerHTML = "";
    if (state.configFiles.length === 0) {
      list.appendChild(el("div", "macro-empty", "No configuration files loaded"));
    } else {
      if (
        state.selectedConfigFile < 0 ||
        state.selectedConfigFile >= state.configFiles.length
      ) {
        state.selectedConfigFile = 0;
      }
      state.configFiles.forEach((file, index) => {
        const row = el("div", "config-file-row file" + (index === state.selectedConfigFile ? " selected" : ""));
        const tree = el("div", "cf-tree-line");
        const arm = el("div", "cf-tree-arm");
        const icon = el("div", "file-icon " + (file.kind === "profile" ? "profile" : "json"), file.kind === "profile" ? "P" : "{}");
        const name = el(
          "div",
          "cf-file-name" + (index === state.selectedConfigFile ? " selected" : "") +
            (file.kind === "profile" ? "" : " mono"),
          file.name
        );
        row.append(tree, arm, icon, name);
        row.addEventListener("click", () => {
          state.selectedConfigFile = index;
          renderConfig();
          loadConfigFile(file);
        });
        list.appendChild(row);
      });
    }
    const current = state.configFiles[state.selectedConfigFile];
    $("#config-path").textContent = state.profileName + "  /  " + (current ? current.name : "layer-1.json");
    $("#config-note").textContent = current && current.kind === "profile"
      ? "Profile manifest · changes are validated when saved"
      : "Layer configuration · changes are validated when saved";
  }

  function loadConfigFile(file) {
    const text = file.content || "{}";
    $("#config-source").value = text;
    highlightConfig(text);
  }

  function highlightConfig(text) {
    const escaped = text
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;");
    const html = escaped.replace(/("(?:\\.|[^"\\])*"(\s*:)?|\b(?:true|false|null)\b|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?)/g, (match) => {
      let cls = "tok-num";
      if (match.startsWith('"')) cls = /:\s*$/.test(match) ? "tok-key" : "tok-str";
      else if (/true|false|null/.test(match)) cls = "tok-bool";
      return '<span class="' + cls + '">' + match + "</span>";
    });
    $("#config-highlight").innerHTML = html;
  }

  const configSource = $("#config-source");
  configSource.addEventListener("input", () => {
    highlightConfig(configSource.value);
    const file = state.configFiles[state.selectedConfigFile];
    if (file) file.content = configSource.value;
  });
  configSource.addEventListener("scroll", () => {
    const pre = $("#config-highlight");
    pre.style.transform = "translate(" + -configSource.scrollLeft + "px," + -configSource.scrollTop + "px)";
  });

  async function loadConfigFiles(preferredName) {
    if (!state.activeProfile) return;
    const previous = state.configFiles[state.selectedConfigFile];
    const wanted = preferredName || (previous ? previous.name : "layer-" + state.layer + ".json");
    try {
      const files = await invoke("config_files", {
        profileId: state.activeProfile.profileId,
      });
      state.configFiles = files;
      let index = files.findIndex((file) => file.name === wanted);
      if (index < 0) index = files.findIndex((file) => file.name === "layer-" + state.layer + ".json");
      if (index < 0 && files.length > 0) index = 0;
      state.selectedConfigFile = index;
      renderConfig();
      if (index >= 0) loadConfigFile(files[index]);
      $("#config-note").textContent = "Loaded from " + state.profileName + " · changes are validated when saved";
    } catch (error) {
      $("#config-note").textContent = "Cannot load configuration · " + message(error);
    }
  }

  $("#save-layer").addEventListener("click", async () => {
    const file = state.configFiles[state.selectedConfigFile];
    if (!file || !state.activeProfile) return;
    const source = configSource.value;
    try {
      const parsed = JSON.parse(source || "{}");
      if (parsed === null || typeof parsed !== "object") {
        throw new Error("Configuration must be an object");
      }
    } catch (error) {
      $("#config-note").textContent = "Invalid JSON · " + message(error);
      setStatus("Configuration rejected: invalid JSON");
      return;
    }
    try {
      const result = await invoke("save_config", {
        input: {
          profileId: state.activeProfile.profileId,
          fileName: file.name,
          source,
        },
      });
      applyWorkspace(result.workspace);
      state.configFiles = result.files;
      const index = result.files.findIndex((item) => item.name === file.name);
      state.selectedConfigFile = index >= 0 ? index : 0;
      renderConfig();
      if (state.selectedConfigFile >= 0) loadConfigFile(result.files[state.selectedConfigFile]);
      $("#config-note").textContent = "Validated · saved to " + state.profileName + " / " + file.name;
      setStatus("Configuration saved · " + file.name);
    } catch (error) {
      $("#config-note").textContent = "Rejected · " + message(error);
      setStatus("Configuration rejected by validation");
    }
  });

  bindImport("#import-file", false);
  bindImport("#import-folder", true);

  function bindImport(selector, folder) {
    document.querySelector(selector).addEventListener("click", async () => {
      try {
        const outcome = await invoke("import_config", { folder });
        if (!outcome) return;
        applyWorkspace(outcome.workspace, { reloadConfig: true });
        const imported = state.profiles.find(
          (profile) => profile.profileId === outcome.importedProfileId
        );
        const importedName = imported ? imported.name : outcome.importedProfileId;
        if (outcome.importedKind === "layer") {
          setStatus("Imported layer into “" + state.profileName + "”");
        } else if (outcome.importedProfileId === state.activeProfile.profileId) {
          setStatus("Imported profile “" + importedName + "”");
        } else {
          setStatus("Imported profile “" + importedName + "” · switch from Profiles");
        }
      } catch (error) {
        setStatus("Import rejected: " + message(error));
      }
    });
  }

  /* ---------------- device and ports ---------------- */
  function renderPorts(ports, status) {
    const select = $("#port-select");
    select.innerHTML = "";
    if (!ports || ports.length === 0) {
      select.appendChild(new Option("No serial ports", ""));
    } else {
      ports.forEach((port) => select.appendChild(new Option(port, port)));
    }
    select.disabled = true;
    $("#port-status").textContent = status || "No device connection · handshake unavailable";
  }

  $("#refresh-ports").addEventListener("click", async () => {
    try {
      const result = await invoke("scan_ports");
      renderPorts(result.ports, result.status);
      setStatus(
        result.ports.length
          ? result.ports.length + " serial port(s) detected · no verified device"
          : "Port scan completed · no serial ports found"
      );
    } catch (error) {
      setStatus("Port scan failed: " + message(error));
    }
  });
  $("#board-select").addEventListener("change", () => setStatus("Board selection does not establish a connection"));
  $("#flash-button").addEventListener("click", () => setStatus("Flashing requires a verified device handshake"));

  /* ---------------- boot ---------------- */
  async function init() {
    updateInspector();
    renderMacroList();
    renderEventList();
    renderProfiles();
    renderConfig();
    showSection("Keymaps");
    loadScene();
    try {
      const boot = await invoke("bootstrap");
      applyWorkspace(boot.workspace, { reloadConfig: true });
      renderPorts(boot.ports, boot.portStatus);
      if (boot.startupSection) showSection(boot.startupSection);
      if (!boot.workspace.warning) setStatus("Profile ready");
    } catch (error) {
      console.error(error);
      setStatus("Backend unavailable · " + message(error));
    }
  }

  init();
})();
