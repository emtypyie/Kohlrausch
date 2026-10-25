use crate::board::{self, LOGICAL_KEYS};
use std::{collections::HashMap, sync::OnceLock};

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            yaw: -0.25,
            pitch: 1.08,
            zoom: 1.34,
        }
    }
}

pub struct SceneFrame {
    pub rgba: Vec<u8>,
    pub hit_ids: Vec<i8>,
    pub width: usize,
    pub height: usize,
    anchors: Vec<(&'static str, f32, f32)>,
}

impl SceneFrame {
    pub fn key_at(&self, x: f32, y: f32) -> Option<&'static str> {
        let x = x.floor() as isize;
        let y = y.floor() as isize;
        if x < 0 || y < 0 || x >= self.width as isize || y >= self.height as isize {
            return None;
        }

        let index = self.hit_ids[y as usize * self.width + x as usize];
        (index >= 0).then(|| LOGICAL_KEYS[index as usize].id)
    }

    pub fn anchor_for(&self, key_id: &str) -> Option<(f32, f32)> {
        self.anchors
            .iter()
            .find(|(id, _, _)| *id == key_id)
            .map(|(_, x, y)| (*x, *y))
    }
}

#[derive(Clone, Copy)]
struct V3 {
    x: f32,
    y: f32,
    z: f32,
}

impl V3 {
    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    fn cross(self, other: Self) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    fn norm(self) -> Self {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z)
            .sqrt()
            .max(0.0001);
        Self {
            x: self.x / length,
            y: self.y / length,
            z: self.z / length,
        }
    }
}

#[derive(Clone, Copy)]
struct Projected {
    x: f32,
    y: f32,
    depth: f32,
}

#[derive(Clone, Copy)]
struct Triangle {
    points: [V3; 3],
    color: [u8; 3],
    shade: f32,
    key: i8,
}

#[derive(Clone, Copy)]
struct Edge {
    start: V3,
    end: V3,
    color: [u8; 3],
    thickness: f32,
}

struct CadShell {
    triangles: Vec<Triangle>,
    edges: Vec<Edge>,
}

#[derive(Clone, Copy)]
struct EdgeUse {
    start: V3,
    end: V3,
    normal: V3,
    count: usize,
    sharp: bool,
}

pub fn render(width: usize, height: usize, camera: Camera) -> SceneFrame {
    let width = width.max(1);
    let height = height.max(1);
    let mut rgba = vec![0u8; width * height * 4];
    let mut hit_ids = vec![-1i8; width * height];
    let mut z_buffer = vec![f32::INFINITY; width * height];

    paint_background(&mut rgba, width, height);

    let shell = cad_shell();
    let focal = height as f32 * 1.20 * camera.zoom.clamp(0.72, 1.65);
    let screen_center = (width as f32 * 0.51, height as f32 * 0.51);

    for triangle in &shell.triangles {
        let Some(a) = project(triangle.points[0], camera, focal, screen_center) else {
            continue;
        };
        let Some(b) = project(triangle.points[1], camera, focal, screen_center) else { continue; };
        let Some(c) = project(triangle.points[2], camera, focal, screen_center) else { continue; };

        raster_triangle(
            &mut rgba,
            &mut hit_ids,
            &mut z_buffer,
            width,
            height,
            [a, b, c],
            triangle.color,
            triangle.shade,
            triangle.key,
        );
    }

    for edge in &shell.edges {
        raster_edge(
            &mut rgba,
            &z_buffer,
            width,
            height,
            project(edge.start, camera, focal, screen_center),
            project(edge.end, camera, focal, screen_center),
            edge.color,
            edge.thickness,
        );
    }

    let (lyr_x, lyr_y) = board::lyr_position_mm();
    if let Some(point) = project(V3 { x: lyr_x / 20.0, y: lyr_y / 20.0, z: 0.55 }, camera, focal, screen_center) {
        draw_label(&mut rgba, width, height, point.x, point.y, "LYR", [255, 195, 172]);
    }
    let (usb_x, usb_y) = board::usb_position_mm();
    if let Some(point) = project(V3 { x: usb_x / 20.0, y: usb_y / 20.0, z: 0.28 }, camera, focal, screen_center) {
        draw_label(&mut rgba, width, height, point.x, point.y, "USB", [176, 180, 193]);
    }

    let hit_anchors = hit_anchors(&hit_ids, width);
    let mut anchors = Vec::with_capacity(LOGICAL_KEYS.len());
    for key in &LOGICAL_KEYS {
        let (x, y) = key_position(key);
        if let Some(point) = project(V3 { x, y, z: 0.52 }, camera, focal, screen_center) {
            let (anchor_x, anchor_y) = hit_anchors[key_index(key) as usize]
                .unwrap_or((point.x, point.y));
            anchors.push((key.id, anchor_x, anchor_y));
            draw_label(&mut rgba, width, height, point.x, point.y, key.physical_reference, [236, 229, 238]);
        }
    }

    SceneFrame { rgba, hit_ids, width, height, anchors }
}

fn hit_anchors(ids: &[i8], width: usize) -> [Option<(f32, f32)>; 9] {
    let mut totals = [(0usize, 0usize, 0usize); 9];
    for (index, id) in ids.iter().enumerate() {
        if *id >= 0 {
            let total = &mut totals[*id as usize];
            total.0 += index % width;
            total.1 += index / width;
            total.2 += 1;
        }
    }
    let centers = std::array::from_fn::<_, 9, _>(|key| {
        let (x, y, count) = totals[key];
        (count > 0).then_some((x as f32 / count as f32, y as f32 / count as f32))
    });
    let mut best = [(f32::INFINITY, 0usize); 9];
    for (index, id) in ids.iter().enumerate() {
        if *id < 0 { continue; }
        let key = *id as usize;
        let Some((center_x, center_y)) = centers[key] else { continue; };
        let x = index % width;
        let y = index / width;
        let distance = (x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2);
        if distance < best[key].0 {
            best[key] = (distance, index);
        }
    }
    std::array::from_fn(|key| {
        centers[key].map(|_| {
            let index = best[key].1;
            ((index % width) as f32 + 0.5, (index / width) as f32 + 0.5)
        })
    })
}

fn key_index(key: &crate::board::LogicalKey) -> i8 {
    LOGICAL_KEYS.iter().position(|candidate| candidate.id == key.id).unwrap_or(0) as i8
}

fn paint_background(rgba: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        let t = y as f32 / height as f32;
        let base = [
            (9.0 + 3.0 * (1.0 - t)) as u8,
            (9.0 + 2.0 * (1.0 - t)) as u8,
            (11.0 + 4.0 * (1.0 - t)) as u8,
        ];
        for x in 0..width {
            let offset = (y * width + x) * 4;
            rgba[offset..offset + 3].copy_from_slice(&base);
            rgba[offset + 3] = 255;
        }
    }
}

fn cad_shell() -> &'static CadShell {
    static SHELL: OnceLock<CadShell> = OnceLock::new();
    SHELL.get_or_init(|| {
        let mut triangles = Vec::with_capacity(34_000);
        let mut edges = Vec::with_capacity(20_000);
        append_stl(
            include_bytes!("../ui/assets/kohlrausch_case_tray_v6.stl"),
            [57, 58, 63],
            [133, 130, 139],
            &mut triangles,
            &mut edges,
            V3 { x: 0.0, y: 0.0, z: 0.0 },
            -1,
        );
        append_stl(
            include_bytes!("../ui/assets/kohlrausch_pcb_v6.stl"),
            [32, 55, 42],
            [94, 132, 105],
            &mut triangles,
            &mut edges,
            V3 { x: 0.0, y: 0.0, z: 0.0 },
            -1,
        );
        append_stl(
            include_bytes!("../ui/assets/kohlrausch_pcb_modules_v6.stl"),
            [83, 85, 99],
            [174, 177, 192],
            &mut triangles,
            &mut edges,
            V3 { x: 0.0, y: 0.0, z: 0.0 },
            -1,
        );
        append_stl(
            include_bytes!("../ui/assets/kohlrausch_usb_port_v6.stl"),
            [119, 123, 132],
            [190, 194, 204],
            &mut triangles,
            &mut edges,
            V3 { x: 0.0, y: 0.0, z: 0.0 },
            -1,
        );
        let switch_mesh = include_bytes!("../ui/assets/cherry_mx_switch_3pin.stl");
        for (index, key) in LOGICAL_KEYS.iter().enumerate() {
            let (x, y) = key_position(key);
            append_stl(
                switch_mesh,
                [67, 62, 76],
                [162, 148, 173],
                &mut triangles,
                &mut edges,
                V3 { x, y, z: 0.0 },
                index as i8,
            );
        }
        let (lyr_x, lyr_y) = board::lyr_position_mm();
        append_stl(
            switch_mesh,
            [108, 54, 39],
            [255, 125, 77],
            &mut triangles,
            &mut edges,
            V3 { x: lyr_x / 20.0, y: lyr_y / 20.0, z: 0.0 },
            -1,
        );
        CadShell { triangles, edges }
    })
}

fn append_stl(
    bytes: &[u8],
    color: [u8; 3],
    edge_color: [u8; 3],
    triangles: &mut Vec<Triangle>,
    edges: &mut Vec<Edge>,
    offset: V3,
    key: i8,
) {
    let Some(count_bytes) = bytes.get(80..84) else {
        return;
    };
    let facet_count = u32::from_le_bytes(count_bytes.try_into().expect("four bytes")) as usize;
    let mut edge_map: HashMap<[(i32, i32, i32); 2], EdgeUse> = HashMap::new();

    for facet in 0..facet_count {
        let facet_offset = 84 + facet * 50 + 12;
        let Some(facet_bytes) = bytes.get(facet_offset..facet_offset + 36) else {
            break;
        };
        let mut points = [V3 { x: 0.0, y: 0.0, z: 0.0 }; 3];
        for vertex in 0..3 {
            let start = vertex * 12;
            let read = |index: usize| {
                f32::from_le_bytes(facet_bytes[index..index + 4].try_into().expect("four bytes"))
                / 20.0
            };
            points[vertex] = V3 {
                x: read(start) + offset.x,
                y: read(start + 4) + offset.y,
                z: read(start + 8) + offset.z,
            };
        }

        let normal = points[1].sub(points[0]).cross(points[2].sub(points[0])).norm();
        let light = V3 { x: -0.35, y: -0.48, z: 0.82 }.norm();
        let shade = (0.50 + 0.50 * normal.dot(light).abs()).clamp(0.50, 1.0);
        triangles.push(Triangle { points, color, shade, key });

        for edge_index in 0..3 {
            let a = points[edge_index];
            let b = points[(edge_index + 1) % 3];
            let a_key = vertex_key(a);
            let b_key = vertex_key(b);
            let key = if a_key <= b_key { [a_key, b_key] } else { [b_key, a_key] };
            edge_map
                .entry(key)
                .and_modify(|edge| {
                    if edge.normal.dot(normal) < 0.78 {
                        edge.sharp = true;
                    }
                    edge.count += 1;
                })
                .or_insert(EdgeUse {
                    start: a,
                    end: b,
                    normal,
                    count: 1,
                    sharp: false,
                });
        }
    }

    for edge in edge_map.into_values().filter(|edge| edge.count == 1 || edge.sharp) {
        edges.push(Edge {
            start: edge.start,
            end: edge.end,
            color: edge_color,
            thickness: if edge.count == 1 { 1.25 } else { 0.95 },
        });
    }
}

fn vertex_key(point: V3) -> (i32, i32, i32) {
    (
        (point.x * 100_000.0).round() as i32,
        (point.y * 100_000.0).round() as i32,
        (point.z * 100_000.0).round() as i32,
    )
}

fn key_position(key: &crate::board::LogicalKey) -> (f32, f32) {
    let (x, y) = board::model_position_mm(key.physical_id).unwrap_or((0.0, 0.0));
    (x / 20.0, y / 20.0)
}

fn project(point: V3, camera: Camera, focal: f32, center: (f32, f32)) -> Option<Projected> {
    let (sin_yaw, cos_yaw) = camera.yaw.sin_cos();
    let model_y = -point.y;
    let x = point.x * cos_yaw - model_y * sin_yaw;
    let y = point.x * sin_yaw + model_y * cos_yaw;
    let (sin_pitch, cos_pitch) = camera.pitch.clamp(0.25, 1.15).sin_cos();
    let vertical = point.z * cos_pitch - y * sin_pitch;
    let depth = 8.8 - (y * cos_pitch + point.z * sin_pitch);
    if depth < 0.1 {
        return None;
    }

    let scale = focal / depth;
    Some(Projected {
        x: center.0 + x * scale,
        y: center.1 - vertical * scale,
        depth,
    })
}

fn raster_triangle(
    rgba: &mut [u8],
    ids: &mut [i8],
    z_buffer: &mut [f32],
    width: usize,
    height: usize,
    points: [Projected; 3],
    base: [u8; 3],
    shade: f32,
    key: i8,
) {
    let area = (points[1].x - points[0].x) * (points[2].y - points[0].y)
        - (points[1].y - points[0].y) * (points[2].x - points[0].x);
    if area.abs() < 0.001 {
        return;
    }

    let min_x = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
    let max_x = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max).ceil().min(width as f32 - 1.0) as usize;
    let min_y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min).floor().max(0.0) as usize;
    let max_y = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max).ceil().min(height as f32 - 1.0) as usize;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = ((points[1].x - px) * (points[2].y - py)
                - (points[1].y - py) * (points[2].x - px))
                / area;
            let w1 = ((points[2].x - px) * (points[0].y - py)
                - (points[2].y - py) * (points[0].x - px))
                / area;
            let w2 = 1.0 - w0 - w1;
            if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                continue;
            }

            let depth = w0 * points[0].depth + w1 * points[1].depth + w2 * points[2].depth;
            let index = y * width + x;
            if depth >= z_buffer[index] {
                continue;
            }

            z_buffer[index] = depth;
            let offset = index * 4;
            for channel in 0..3 {
                rgba[offset + channel] = (base[channel] as f32 * shade) as u8;
            }
            rgba[offset + 3] = 255;
            ids[index] = key;
        }
    }
}

fn raster_edge(
    rgba: &mut [u8],
    z_buffer: &[f32],
    width: usize,
    height: usize,
    start: Option<Projected>,
    end: Option<Projected>,
    color: [u8; 3],
    thickness: f32,
) {
    let (Some(start), Some(end)) = (start, end) else {
        return;
    };

    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let steps = (dx.abs().max(dy.abs()) * 1.5).ceil().max(1.0) as usize;
    let radius = thickness * 0.5;

    for step in 0..=steps {
        let t = step as f32 / steps as f32;
        let x = start.x + dx * t;
        let y = start.y + dy * t;
        let depth = start.depth + (end.depth - start.depth) * t;
        let min_x = (x - radius - 1.0).floor().max(0.0) as usize;
        let max_x = (x + radius + 1.0).ceil().min(width as f32 - 1.0) as usize;
        let min_y = (y - radius - 1.0).floor().max(0.0) as usize;
        let max_y = (y + radius + 1.0).ceil().min(height as f32 - 1.0) as usize;

        for py in min_y..=max_y {
            for px in min_x..=max_x {
                let distance = ((px as f32 + 0.5 - x).powi(2) + (py as f32 + 0.5 - y).powi(2)).sqrt();
                let coverage = (radius + 0.7 - distance).clamp(0.0, 1.0);
                let index = py * width + px;
                if coverage == 0.0 || depth > z_buffer[index] + 0.035 {
                    continue;
                }

                blend_pixel(rgba, index * 4, color, coverage);
            }
        }
    }
}

fn blend_pixel(rgba: &mut [u8], offset: usize, color: [u8; 3], alpha: f32) {
    for channel in 0..3 {
        let existing = rgba[offset + channel] as f32;
        rgba[offset + channel] = (existing * (1.0 - alpha) + color[channel] as f32 * alpha) as u8;
    }
}

fn draw_label(rgba: &mut [u8], width: usize, height: usize, x: f32, y: f32, label: &str, color: [u8; 3]) {
    let scale = 1usize;
    let mut cursor = x as isize - (label.len() as isize * 6 * scale as isize) / 2;
    let top = y as isize - 7 * scale as isize / 2;

    let left = cursor - 2;
    let right = cursor + label.len() as isize * 6 * scale as isize;
    let upper = top - 2;
    let lower = top + 7 * scale as isize + 1;
    for py in upper..=lower {
        for px in left..=right {
            if px < 0 || py < 0 || px >= width as isize || py >= height as isize {
                continue;
            }
            let border = px == left || px == right || py == upper || py == lower;
            let shade = if border { [91, 75, 96] } else { [13, 12, 16] };
            let offset = (py as usize * width + px as usize) * 4;
            blend_pixel(rgba, offset, shade, if border { 0.9 } else { 0.88 });
        }
    }

    for character in label.chars() {
        let glyph = match character {
            '0' => [14, 17, 19, 21, 25, 17, 14],
            '1' => [4, 12, 4, 4, 4, 4, 14],
            '2' => [14, 17, 1, 2, 4, 8, 31],
            '3' => [30, 1, 1, 14, 1, 1, 30],
            'L' => [16, 16, 16, 16, 16, 16, 31],
            'Y' => [17, 17, 10, 4, 4, 4, 4],
            'R' => [30, 17, 17, 30, 20, 18, 17],
            'U' => [17, 17, 17, 17, 17, 17, 14],
            'S' => [15, 16, 16, 14, 1, 1, 30],
            'B' => [30, 17, 17, 30, 17, 17, 30],
            '.' => [0, 0, 0, 0, 0, 6, 6],
            _ => [0; 7],
        };

        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) == 0 {
                    continue;
                }
                for y_scale in 0..scale {
                    for x_scale in 0..scale {
                        let px = cursor + column as isize * scale as isize + x_scale as isize;
                        let py = top + row as isize * scale as isize + y_scale as isize;
                        if px < 0 || py < 0 || px >= width as isize || py >= height as isize {
                            continue;
                        }
                        let offset = (py as usize * width + px as usize) * 4;
                        rgba[offset..offset + 3].copy_from_slice(&color);
                    }
                }
            }
        }
        cursor += 6 * scale as isize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projected_scene_has_pickable_keys_and_camera_changes_positions() {
        let first = render(540, 440, Camera::default());
        assert!(first.hit_ids.iter().any(|id| *id == 0));
        for key in 0..9 {
            assert!(first.hit_ids.iter().any(|id| *id == key));
        }

        let rotated = render(540, 440, Camera { yaw: 0.35, ..Camera::default() });
        assert_ne!(first.hit_ids, rotated.hit_ids);
    }

    #[test]
    fn every_rendered_key_anchor_hits_its_logical_identity_after_orbit_and_zoom() {
        for camera in [
            Camera::default(),
            Camera { yaw: 0.5, pitch: 0.9, zoom: 1.3 },
        ] {
            let frame = render(720, 540, camera);
            for key in LOGICAL_KEYS {
                let (x, y) = frame.anchor_for(key.id).expect("key anchor must be rendered");
                assert_eq!(frame.key_at(x, y), Some(key.id), "anchor for {} at {x:.1},{y:.1}", key.id);
            }
        }
    }

    #[test]
    fn bundled_case_pcb_usb_and_ten_switch_meshes_use_the_case_v6_design() {
        let shell = cad_shell();
        assert_eq!(shell.triangles.len(), 11_744 + 11_248 + 11_712 + 5_996 + 2_159 * 10);
        assert!(shell.edges.len() > 5_000);

        let x_span = shell
            .triangles
            .iter()
            .flat_map(|triangle| triangle.points.iter().map(|point| point.x))
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), x| {
                (min.min(x), max.max(x))
            });
        assert!((x_span.1 - x_span.0 - 3.525).abs() < 0.02);
    }

    #[test]
    fn cad_placements_keep_the_usb_and_lyr_switch_at_the_case_top() {
        let row_centers = (0..3)
            .map(|row| {
                let values = LOGICAL_KEYS
                    .iter()
                    .filter(|key| key.screen_row == row)
                    .map(key_position)
                    .collect::<Vec<_>>();
                assert_eq!(values.len(), 3);
                let min_y = values.iter().map(|(_, y)| y).copied().fold(f32::INFINITY, f32::min);
                let max_y = values.iter().map(|(_, y)| y).copied().fold(f32::NEG_INFINITY, f32::max);
                assert!(max_y - min_y < 0.04, "keys in one row should align");
                values.iter().map(|(_, y)| y).sum::<f32>() / values.len() as f32
            })
            .collect::<Vec<_>>();
        assert!(row_centers[0] > row_centers[1]);
        assert!(row_centers[1] > row_centers[2]);
        let (_, lyr_y) = board::lyr_position_mm();
        let (_, usb_y) = board::usb_position_mm();
        assert!(usb_y > lyr_y);
        assert!(lyr_y > board::model_position_mm("key-3-1").unwrap().1);
        assert!(board::lyr_position_mm().0 < 0.0);
    }
}
