/*
 * Port of src/scene.rs: renders the Kohlrausch 3x3 macropad from the bundled
 * binary STL meshes into a 840x540 canvas, with the same camera, z-buffer,
 * edge pass, labels and hit-testing used by the Slint build.
 */
(function (global) {
  "use strict";

  const CAMERA = { yaw: -0.25, pitch: 1.08, zoom: 1.0 };
  const LIGHT = normalize({ x: -0.35, y: -0.48, z: 0.82 });

  const KEYS = [
    { id: "logical-1-1", label: "3.1", physicalId: "key-3-1", physicalReference: "3.1", pos: [-19.77, 7.92] },
    { id: "logical-1-2", label: "2.1", physicalId: "key-2-1", physicalReference: "2.1", pos: [-0.72, 7.92] },
    { id: "logical-1-3", label: "1.1", physicalId: "key-1-1", physicalReference: "1.1", pos: [18.33, 7.92] },
    { id: "logical-2-1", label: "3.2", physicalId: "key-3-2", physicalReference: "3.2", pos: [-19.39, -11.0] },
    { id: "logical-2-2", label: "2.2", physicalId: "key-2-2", physicalReference: "2.2", pos: [-0.47, -11.0] },
    { id: "logical-2-3", label: "1.2", physicalId: "key-1-2", physicalReference: "1.2", pos: [18.84, -11.0] },
    { id: "logical-3-1", label: "3.3", physicalId: "key-3-3", physicalReference: "3.3", pos: [-19.52, -30.18] },
    { id: "logical-3-2", label: "2.3", physicalId: "key-2-3", physicalReference: "2.3", pos: [-0.47, -30.18] },
    { id: "logical-3-3", label: "1.3", physicalId: "key-1-3", physicalReference: "1.3", pos: [18.97, -30.18] },
  ];
  const LYR_POS = [-19.05, 27.13];
  const USB_POS = [0.38, 40.94];

  function sub(a, b) { return { x: a.x - b.x, y: a.y - b.y, z: a.z - b.z }; }
  function cross(a, b) {
    return { x: a.y * b.z - a.z * b.y, y: a.z * b.x - a.x * b.z, z: a.x * b.y - a.y * b.x };
  }
  function dot(a, b) { return a.x * b.x + a.y * b.y + a.z * b.z; }
  function normalize(v) {
    const len = Math.max(Math.sqrt(v.x * v.x + v.y * v.y + v.z * v.z), 0.0001);
    return { x: v.x / len, y: v.y / len, z: v.z / len };
  }
  function vkey(p) {
    return Math.round(p.x * 100000) + "," + Math.round(p.y * 100000) + "," + Math.round(p.z * 100000);
  }

  async function loadStl(name) {
    const response = await fetch("assets/" + name);
    if (!response.ok) throw new Error("Failed to load " + name);
    return new Uint8Array(await response.arrayBuffer());
  }

  function appendStl(bytes, color, edgeColor, triangles, edges, ox, oy, oz, key) {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const count = view.getUint32(80, true);
    const edgeMap = new Map();

    for (let facet = 0; facet < count; facet++) {
      const base = 84 + facet * 50 + 12;
      if (base + 36 > bytes.byteLength) break;
      const pts = [];
      for (let vertex = 0; vertex < 3; vertex++) {
        const o = base + vertex * 12;
        pts.push({
          x: view.getFloat32(o, true) / 20 + ox,
          y: view.getFloat32(o + 4, true) / 20 + oy,
          z: view.getFloat32(o + 8, true) / 20 + oz,
        });
      }
      const normal = normalize(cross(sub(pts[1], pts[0]), sub(pts[2], pts[0])));
      const shade = Math.min(1, Math.max(0.5, 0.5 + 0.5 * Math.abs(dot(normal, LIGHT))));
      triangles.push({ pts, color, shade, key });

      for (let e = 0; e < 3; e++) {
        const a = pts[e];
        const b = pts[(e + 1) % 3];
        const ak = vkey(a);
        const bk = vkey(b);
        const k = ak <= bk ? ak + "|" + bk : bk + "|" + ak;
        const existing = edgeMap.get(k);
        if (existing) {
          if (dot(existing.normal, normal) < 0.78) existing.sharp = true;
          existing.count += 1;
        } else {
          edgeMap.set(k, { start: a, end: b, normal, count: 1, sharp: false });
        }
      }
    }

    for (const edge of edgeMap.values()) {
      if (edge.count === 1 || edge.sharp) {
        edges.push({
          start: edge.start,
          end: edge.end,
          color: edgeColor,
          thickness: edge.count === 1 ? 1.25 : 0.95,
        });
      }
    }
  }

  async function buildShell() {
    const triangles = [];
    const edges = [];
    const [tray, pcb, modules, usb, sw] = await Promise.all([
      loadStl("kohlrausch_case_tray_v6.stl"),
      loadStl("kohlrausch_pcb_v6.stl"),
      loadStl("kohlrausch_pcb_modules_v6.stl"),
      loadStl("kohlrausch_usb_port_v6.stl"),
      loadStl("cherry_mx_switch_3pin.stl"),
    ]);
    appendStl(tray, [57, 58, 63], [133, 130, 139], triangles, edges, 0, 0, 0, -1);
    appendStl(pcb, [32, 55, 42], [94, 132, 105], triangles, edges, 0, 0, 0, -1);
    appendStl(modules, [83, 85, 99], [174, 177, 192], triangles, edges, 0, 0, 0, -1);
    appendStl(usb, [119, 123, 132], [190, 194, 204], triangles, edges, 0, 0, 0, -1);
    for (let i = 0; i < KEYS.length; i++) {
      const key = KEYS[i];
      appendStl(sw, [67, 62, 76], [162, 148, 173], triangles, edges, key.pos[0] / 20, key.pos[1] / 20, 0, i);
    }
    appendStl(sw, [108, 54, 39], [255, 125, 77], triangles, edges, LYR_POS[0] / 20, LYR_POS[1] / 20, 0, -1);
    return { triangles, edges };
  }

  function project(point, focal, centerX, centerY) {
    const sinYaw = Math.sin(CAMERA.yaw);
    const cosYaw = Math.cos(CAMERA.yaw);
    const modelY = -point.y;
    const x = point.x * cosYaw - modelY * sinYaw;
    const y = point.x * sinYaw + modelY * cosYaw;
    const pitch = Math.min(1.15, Math.max(0.25, CAMERA.pitch));
    const sinPitch = Math.sin(pitch);
    const cosPitch = Math.cos(pitch);
    const vertical = point.z * cosPitch - y * sinPitch;
    const depth = 8.8 - (y * cosPitch + point.z * sinPitch);
    if (depth < 0.1) return null;
    const scale = focal / depth;
    return { x: centerX + x * scale, y: centerY - vertical * scale, depth };
  }

  function paintBackground(rgba, width, height) {
    for (let y = 0; y < height; y++) {
      const t = y / height;
      const r = (9 + 3 * (1 - t)) | 0;
      const g = (9 + 2 * (1 - t)) | 0;
      const b = (11 + 4 * (1 - t)) | 0;
      for (let x = 0; x < width; x++) {
        const o = (y * width + x) * 4;
        rgba[o] = r;
        rgba[o + 1] = g;
        rgba[o + 2] = b;
        rgba[o + 3] = 255;
      }
    }
  }

  function rasterTriangle(rgba, ids, zbuf, width, height, points, base, shade, key) {
    const area = (points[1].x - points[0].x) * (points[2].y - points[0].y) -
      (points[1].y - points[0].y) * (points[2].x - points[0].x);
    if (Math.abs(area) < 0.001) return;

    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    for (const p of points) {
      if (p.x < minX) minX = p.x; if (p.x > maxX) maxX = p.x;
      if (p.y < minY) minY = p.y; if (p.y > maxY) maxY = p.y;
    }
    minX = Math.max(0, Math.floor(minX));
    maxX = Math.min(width - 1, Math.ceil(maxX));
    minY = Math.max(0, Math.floor(minY));
    maxY = Math.min(height - 1, Math.ceil(maxY));

    for (let y = minY; y <= maxY; y++) {
      for (let x = minX; x <= maxX; x++) {
        const px = x + 0.5;
        const py = y + 0.5;
        const w0 = ((points[1].x - px) * (points[2].y - py) - (points[1].y - py) * (points[2].x - px)) / area;
        const w1 = ((points[2].x - px) * (points[0].y - py) - (points[2].y - py) * (points[0].x - px)) / area;
        const w2 = 1 - w0 - w1;
        if (w0 < 0 || w1 < 0 || w2 < 0) continue;

        const depth = w0 * points[0].depth + w1 * points[1].depth + w2 * points[2].depth;
        const index = y * width + x;
        if (depth >= zbuf[index]) continue;

        zbuf[index] = depth;
        const o = index * 4;
        rgba[o] = (base[0] * shade) | 0;
        rgba[o + 1] = (base[1] * shade) | 0;
        rgba[o + 2] = (base[2] * shade) | 0;
        rgba[o + 3] = 255;
        ids[index] = key;
      }
    }
  }

  function blendPixel(rgba, offset, color, alpha) {
    rgba[offset] = rgba[offset] * (1 - alpha) + color[0] * alpha;
    rgba[offset + 1] = rgba[offset + 1] * (1 - alpha) + color[1] * alpha;
    rgba[offset + 2] = rgba[offset + 2] * (1 - alpha) + color[2] * alpha;
  }

  function rasterEdge(rgba, zbuf, width, height, start, end, color, thickness) {
    if (!start || !end) return;
    const dx = end.x - start.x;
    const dy = end.y - start.y;
    const steps = Math.max(1, Math.ceil(Math.max(Math.abs(dx), Math.abs(dy)) * 1.5));
    const radius = thickness * 0.5;

    for (let step = 0; step <= steps; step++) {
      const t = step / steps;
      const x = start.x + dx * t;
      const y = start.y + dy * t;
      const depth = start.depth + (end.depth - start.depth) * t;
      const minX = Math.max(0, Math.floor(x - radius - 1));
      const maxX = Math.min(width - 1, Math.ceil(x + radius + 1));
      const minY = Math.max(0, Math.floor(y - radius - 1));
      const maxY = Math.min(height - 1, Math.ceil(y + radius + 1));
      for (let py = minY; py <= maxY; py++) {
        for (let px = minX; px <= maxX; px++) {
          const distance = Math.sqrt(Math.pow(px + 0.5 - x, 2) + Math.pow(py + 0.5 - y, 2));
          const coverage = Math.min(1, Math.max(0, radius + 0.7 - distance));
          const index = py * width + px;
          if (coverage === 0 || depth > zbuf[index] + 0.035) continue;
          blendPixel(rgba, index * 4, color, coverage);
        }
      }
    }
  }

  const GLYPHS = {
    "0": [14, 17, 19, 21, 25, 17, 14],
    "1": [4, 12, 4, 4, 4, 4, 14],
    "2": [14, 17, 1, 2, 4, 8, 31],
    "3": [30, 1, 1, 14, 1, 1, 30],
    L: [16, 16, 16, 16, 16, 16, 31],
    Y: [17, 17, 10, 4, 4, 4, 4],
    R: [30, 17, 17, 30, 20, 18, 17],
    U: [17, 17, 17, 17, 17, 17, 14],
    S: [15, 16, 16, 14, 1, 1, 30],
    B: [30, 17, 17, 30, 17, 17, 30],
    ".": [0, 0, 0, 0, 0, 6, 6],
  };

  function drawLabel(rgba, width, height, x, y, label, color) {
    let cursor = Math.trunc(x) - Math.trunc((label.length * 6) / 2);
    const top = Math.trunc(y) - Math.trunc(7 / 2);
    const left = cursor - 2;
    const right = cursor + label.length * 6;
    const upper = top - 2;
    const lower = top + 7 + 1;

    for (let py = upper; py <= lower; py++) {
      for (let px = left; px <= right; px++) {
        if (px < 0 || py < 0 || px >= width || py >= height) continue;
        const border = px === left || px === right || py === upper || py === lower;
        const shade = border ? [91, 75, 96] : [13, 12, 16];
        blendPixel(rgba, (py * width + px) * 4, shade, border ? 0.9 : 0.88);
      }
    }

    for (const character of label) {
      const glyph = GLYPHS[character] || [0, 0, 0, 0, 0, 0, 0];
      for (let row = 0; row < 7; row++) {
        for (let column = 0; column < 5; column++) {
          if ((glyph[row] & (1 << (4 - column))) === 0) continue;
          const px = cursor + column;
          const py = top + row;
          if (px < 0 || py < 0 || px >= width || py >= height) continue;
          const o = (py * width + px) * 4;
          rgba[o] = color[0];
          rgba[o + 1] = color[1];
          rgba[o + 2] = color[2];
        }
      }
      cursor += 6;
    }
  }

  function hitAnchors(ids, width) {
    const totals = Array.from({ length: 9 }, () => ({ x: 0, y: 0, count: 0 }));
    for (let index = 0; index < ids.length; index++) {
      const id = ids[index];
      if (id < 0) continue;
      const total = totals[id];
      total.x += index % width;
      total.y += Math.floor(index / width);
      total.count += 1;
    }
    const centers = totals.map((t) => (t.count > 0 ? [t.x / t.count, t.y / t.count] : null));
    const best = Array.from({ length: 9 }, () => [Infinity, 0]);
    for (let index = 0; index < ids.length; index++) {
      const id = ids[index];
      if (id < 0) continue;
      const center = centers[id];
      if (!center) continue;
      const x = index % width;
      const y = Math.floor(index / width);
      const distance = Math.pow(x - center[0], 2) + Math.pow(y - center[1], 2);
      if (distance < best[id][0]) best[id] = [distance, index];
    }
    return centers.map((center, key) => {
      if (!center) return null;
      const index = best[key][1];
      return [(index % width) + 0.5, Math.floor(index / width) + 0.5];
    });
  }

  async function render(canvas) {
    const width = 840;
    const height = 540;
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d");
    const image = ctx.createImageData(width, height);
    const rgba = image.data;
    const ids = new Int8Array(width * height).fill(-1);
    const zbuf = new Float32Array(width * height).fill(Infinity);

    paintBackground(rgba, width, height);

    const shell = await buildShell();

    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity;
    for (const triangle of shell.triangles) {
      for (const point of triangle.pts) {
        const p = project(point, 1, 0, 0);
        if (!p) continue;
        if (p.x < minX) minX = p.x; if (p.x > maxX) maxX = p.x;
        if (p.y < minY) minY = p.y; if (p.y > maxY) maxY = p.y;
      }
    }
    const boundsWidth = Math.max(maxX - minX, 0.001);
    const boundsHeight = Math.max(maxY - minY, 0.001);
    const fitScale = Math.min((width * 0.84) / boundsWidth, (height * 0.84) / boundsHeight);
    const focal = fitScale * CAMERA.zoom;
    const centerX = width * 0.5 - (minX + maxX) * focal * 0.5;
    const centerY = height * 0.5 - (minY + maxY) * focal * 0.5;

    for (const triangle of shell.triangles) {
      const a = project(triangle.pts[0], focal, centerX, centerY);
      if (!a) continue;
      const b = project(triangle.pts[1], focal, centerX, centerY);
      if (!b) continue;
      const c = project(triangle.pts[2], focal, centerX, centerY);
      if (!c) continue;
      rasterTriangle(rgba, ids, zbuf, width, height, [a, b, c], triangle.color, triangle.shade, triangle.key);
    }

    for (const edge of shell.edges) {
      rasterEdge(rgba, zbuf, width, height,
        project(edge.start, focal, centerX, centerY),
        project(edge.end, focal, centerX, centerY),
        edge.color, edge.thickness);
    }

    const lyr = project({ x: LYR_POS[0] / 20, y: LYR_POS[1] / 20, z: 0.55 }, focal, centerX, centerY);
    if (lyr) drawLabel(rgba, width, height, lyr.x, lyr.y, "LYR", [255, 195, 172]);
    const usb = project({ x: USB_POS[0] / 20, y: USB_POS[1] / 20, z: 0.28 }, focal, centerX, centerY);
    if (usb) drawLabel(rgba, width, height, usb.x, usb.y, "USB", [176, 180, 193]);

    const anchors = hitAnchors(ids, width);
    for (let i = 0; i < KEYS.length; i++) {
      const key = KEYS[i];
      const p = project({ x: key.pos[0] / 20, y: key.pos[1] / 20, z: 0.52 }, focal, centerX, centerY);
      if (!p) continue;
      drawLabel(rgba, width, height, p.x, p.y, key.physicalReference, [236, 229, 238]);
    }

    ctx.putImageData(image, 0, 0);

    const anchorMap = {};
    for (let i = 0; i < KEYS.length; i++) {
      anchorMap[KEYS[i].id] = anchors[i];
    }

    return {
      width,
      height,
      hitIds: ids,
      anchors: anchorMap,
      keys: KEYS,
      keyAt(x, y) {
        const ix = Math.floor(x);
        const iy = Math.floor(y);
        if (ix < 0 || iy < 0 || ix >= width || iy >= height) return null;
        const id = ids[iy * width + ix];
        return id >= 0 ? KEYS[id].id : null;
      },
      anchorFor(id) {
        return anchorMap[id] || null;
      },
    };
  }

  global.KohlrauschScene = { render, KEYS };
})(window);
