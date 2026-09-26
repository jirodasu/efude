// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 jirodasu
//! Small, lossless Pyxel sprite workspace. Each frame is a raster layer in
//! the native .efude document; only the active frame is visible for PNG export.

use super::*;

const MODE_KEY: &str = "efude.pyxel.sprite";
const MODE_VERSION: &str = "32x32-v1";
const FRAME_SIDE: u32 = 32;
const FRAME_PREFIX: &str = "Pyxel Frame ";

// docs/pyxel.gpl in kitao/pyxel (the default, user-replaceable Pyxel palette).
pub(crate) const PYXEL_COLORS: [[u8; 3]; 16] = [
    [0, 0, 0],
    [43, 51, 95],
    [126, 32, 114],
    [25, 149, 156],
    [139, 72, 82],
    [57, 92, 152],
    [169, 193, 255],
    [238, 238, 238],
    [212, 24, 108],
    [211, 132, 65],
    [233, 195, 91],
    [112, 198, 169],
    [118, 150, 222],
    [163, 163, 163],
    [255, 151, 152],
    [237, 199, 176],
];

pub(crate) fn pyxel_pixel_valid(pixel: [u8; 4]) -> bool {
    pixel[3] == 0
        || (pixel[3] == 255 && PYXEL_COLORS.iter().any(|rgb| rgb.as_slice() == &pixel[..3]))
}

fn nearest_pyxel_color(color: Color32) -> [u8; 4] {
    let rgb = PYXEL_COLORS
        .iter()
        .min_by_key(|candidate| {
            candidate
                .iter()
                .enumerate()
                .map(|(i, &channel)| {
                    let delta = i32::from(channel) - i32::from(color.to_array()[i]);
                    delta * delta
                })
                .sum::<i32>()
        })
        .expect("the Pyxel palette is nonempty");
    [rgb[0], rgb[1], rgb[2], 255]
}

impl EfudeApp {
    pub(crate) fn is_pyxel_document(&self) -> bool {
        self.doc
            .metadata
            .get(MODE_KEY)
            .is_some_and(|v| v == MODE_VERSION)
    }

    pub(crate) fn new_pyxel_document(&mut self) {
        self.open_document_tab();
        self.replace_document(Document::new(FRAME_SIDE, FRAME_SIDE), None);
        self.doc.layers[0].name = format!("{FRAME_PREFIX}001");
        self.history
            .set_metadata(&mut self.doc, MODE_KEY, Some(MODE_VERSION.into()));
        self.color = Color32::from_rgb(0, 0, 0);
        self.transparent_color = false;
        self.status = self
            .text("Pyxel スプライトを作成しました", "Pyxel sprite created")
            .into();
    }

    /// Reject edits made through general illustration commands that would
    /// break the 16-colour/transparent guarantee. This runs before both
    /// native saves and image exports, including automatic backups.
    pub(crate) fn validate_pyxel_document(&self) -> Result<(), String> {
        if !self.is_pyxel_document() {
            return Ok(());
        }
        if self.doc.width != FRAME_SIDE
            || self.doc.height != FRAME_SIDE
            || self.doc.layers.is_empty()
            || self.doc.layers.iter().filter(|layer| layer.visible).count() != 1
        {
            return Err("Pyxel: 32×32 と表示フレーム1枚を確認してください".into());
        }
        for layer in &self.doc.layers {
            if !layer.name.starts_with(FRAME_PREFIX)
                || layer.kind != LayerKind::Raster
                || layer.opacity != 1.0
                || layer.blend != BlendMode::Normal
                || layer.mask.is_some()
                || layer.parent_id.is_some()
                || layer.tone.is_some()
                || layer.vector.is_some()
            {
                return Err("Pyxel: フレーム以外のレイヤー設定は保存・書き出しできません".into());
            }
            for y in 0..FRAME_SIDE {
                for x in 0..FRAME_SIDE {
                    if !pyxel_pixel_valid(layer.pixels.pixel(x, y)) {
                        return Err(format!(
                            "Pyxel: {} の ({x},{y}) に16色以外または半透明の色があります",
                            layer.name
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn active_pyxel_frame(&self) -> usize {
        self.doc
            .layers
            .iter()
            .position(|layer| layer.visible)
            .unwrap_or(0)
    }

    fn switch_pyxel_frame(&mut self, index: usize) {
        if index >= self.doc.layers.len() || index == self.active_pyxel_frame() {
            return;
        }
        self.history.begin();
        for (i, layer) in self.doc.layers.iter_mut().enumerate() {
            let before = layer.property_state();
            layer.visible = i == index;
            self.history
                .record_layer_properties(layer.id, before, layer.property_state());
        }
        self.history.commit();
        self.selected_layer = index;
        self.canvas_texture_dirty = true;
        self.navigator_texture_dirty = true;
    }

    fn add_pyxel_frame(&mut self, copy: bool) {
        if self.doc.layers.len() >= 64 {
            self.status = self.text("最大64フレームです", "Maximum 64 frames").into();
            return;
        }
        let old = self.active_pyxel_frame();
        let id = self
            .doc
            .layers
            .iter()
            .map(|layer| layer.id)
            .max()
            .unwrap_or(0)
            + 1;
        let mut layer =
            efude_canvas::Layer::new(id, format!("{FRAME_PREFIX}{id:03}"), FRAME_SIDE, FRAME_SIDE);
        if copy {
            layer.pixels = self.doc.layers[old].pixels.clone();
        }
        self.history.begin();
        let before = self.doc.layers[old].property_state();
        self.doc.layers[old].visible = false;
        self.history.record_layer_properties(
            self.doc.layers[old].id,
            before,
            self.doc.layers[old].property_state(),
        );
        let index = old + 1;
        self.history
            .insert_layer(&mut self.doc.layers, index, layer);
        self.history.commit();
        self.selected_layer = index;
        self.canvas_texture_dirty = true;
        self.navigator_texture_dirty = true;
    }

    fn delete_pyxel_frame(&mut self) {
        if self.doc.layers.len() <= 1 {
            return;
        }
        let old = self.active_pyxel_frame();
        let next = if old > 0 { old - 1 } else { 1 };
        self.history.begin();
        let before = self.doc.layers[next].property_state();
        self.doc.layers[next].visible = true;
        self.history.record_layer_properties(
            self.doc.layers[next].id,
            before,
            self.doc.layers[next].property_state(),
        );
        self.history.delete_layer(&mut self.doc.layers, old);
        self.history.commit();
        self.selected_layer = if old < next { next - 1 } else { next };
        self.canvas_texture_dirty = true;
        self.navigator_texture_dirty = true;
    }

    pub(crate) fn pyxel_palette_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("PYXEL 16");
        ui.label(self.text(
            "透明または公式16色で描画",
            "Draw with transparency or the 16 default colors",
        ));
        if ui
            .selectable_label(
                self.transparent_color,
                self.text("透明（消しゴム）", "Transparent (eraser)"),
            )
            .clicked()
        {
            self.transparent_color = true;
        }
        ui.horizontal_wrapped(|ui| {
            for (index, rgb) in PYXEL_COLORS.iter().enumerate() {
                let selected = !self.transparent_color && self.color.to_array()[..3] == rgb[..];
                let label = if selected {
                    format!("●{index:X}")
                } else {
                    format!(" {index:X} ")
                };
                let swatch = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                if ui
                    .add(
                        egui::Button::new(label)
                            .fill(swatch)
                            .min_size(Vec2::splat(30.0)),
                    )
                    .on_hover_text(format!(
                        "#{index:X} — {:02X}{:02X}{:02X}",
                        rgb[0], rgb[1], rgb[2]
                    ))
                    .clicked()
                {
                    self.color = swatch;
                    self.transparent_color = false;
                }
            }
        });
        ui.separator();
        ui.label(self.text(
            "フレームは .efude に保存。PNGには表示中の1枚を書き出します。",
            "Frames are saved in .efude; PNG exports the visible frame.",
        ));
    }

    pub(crate) fn pyxel_canvas_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(self.text("FRAME", "FRAME"));
            let active = self.active_pyxel_frame();
            let mut chosen = None;
            for index in 0..self.doc.layers.len() {
                if ui
                    .selectable_label(index == active, format!("{}", index + 1))
                    .clicked()
                {
                    chosen = Some(index);
                }
            }
            if let Some(index) = chosen {
                self.switch_pyxel_frame(index);
            }
        });
        ui.horizontal(|ui| {
            if ui
                .button(self.text("＋空フレーム", "+ Blank frame"))
                .clicked()
            {
                self.add_pyxel_frame(false);
            }
            if ui
                .button(self.text("＋複製フレーム", "+ Duplicate frame"))
                .clicked()
            {
                self.add_pyxel_frame(true);
            }
            if ui
                .add_enabled(
                    self.doc.layers.len() > 1,
                    egui::Button::new(self.text("－削除", "Delete frame")),
                )
                .clicked()
            {
                self.delete_pyxel_frame();
            }
            ui.label(format!("{}×{}", FRAME_SIDE, FRAME_SIDE));
        });
        ui.label(self.text(
            "左ドラッグで1pxずつ描画。色は「カラー」タブから選択。",
            "Drag to draw 1px pixels. Choose a color in the Color panel.",
        ));
        let available = ui.available_size();
        let cell = (available.x / FRAME_SIDE as f32)
            .min(available.y / FRAME_SIDE as f32)
            .floor()
            .clamp(1.0, 20.0);
        let side = cell * FRAME_SIDE as f32;
        let (response, painter) =
            ui.allocate_painter(Vec2::splat(side), egui::Sense::click_and_drag());
        let active = self.active_pyxel_frame();
        let layer = &self.doc.layers[active];
        for y in 0..FRAME_SIDE {
            for x in 0..FRAME_SIDE {
                let point = response.rect.min + Vec2::new(x as f32 * cell, y as f32 * cell);
                let square = Rect::from_min_size(point, Vec2::splat(cell));
                let px = layer.pixels.pixel(x, y);
                let color = if px[3] == 0 {
                    if (x / 2 + y / 2) % 2 == 0 {
                        Color32::from_gray(205)
                    } else {
                        Color32::from_gray(155)
                    }
                } else {
                    Color32::from_rgba_unmultiplied(px[0], px[1], px[2], px[3])
                };
                painter.rect_filled(square, 0.0, color);
            }
        }
        if response.drag_started() {
            self.history.begin();
        }
        if (response.dragged() || response.drag_started() || response.clicked())
            && let Some(pointer) = response.interact_pointer_pos()
            && response.rect.contains(pointer)
        {
            let x = ((pointer.x - response.rect.min.x) / cell) as u32;
            let y = ((pointer.y - response.rect.min.y) / cell) as u32;
            if x < FRAME_SIDE && y < FRAME_SIDE {
                let pixel = if self.transparent_color {
                    [0; 4]
                } else {
                    nearest_pyxel_color(self.color)
                };
                let index = ((y * FRAME_SIDE + x) * 4) as usize;
                if self.doc.layers[active].pixels.pixel(x, y) != pixel {
                    self.history.record_pixel(&self.doc.layers[active], index);
                    self.doc.layers[active].pixels.set_pixel(x, y, pixel);
                    self.canvas_texture_dirty = true;
                    self.navigator_texture_dirty = true;
                }
            }
        }
        if response.drag_stopped() || response.clicked() {
            self.history.commit();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn palette_allows_only_opaque_standard_colors_or_transparency() {
        assert_eq!(PYXEL_COLORS.len(), 16);
        assert!(pyxel_pixel_valid([0, 0, 0, 0]));
        assert!(pyxel_pixel_valid([43, 51, 95, 255]));
        assert!(!pyxel_pixel_valid([43, 51, 95, 128]));
        assert!(!pyxel_pixel_valid([43, 51, 96, 255]));
        assert_eq!(
            nearest_pyxel_color(Color32::from_rgb(43, 51, 96)),
            [43, 51, 95, 255]
        );
    }

    #[test]
    fn frames_and_palette_survive_native_document_round_trip() {
        let mut app = EfudeApp::default();
        app.new_pyxel_document();
        assert!(app.is_pyxel_document());
        app.doc.layers[0].pixels.set_pixel(3, 4, [43, 51, 95, 255]);
        app.add_pyxel_frame(true);
        app.doc.layers[1]
            .pixels
            .set_pixel(3, 4, [212, 24, 108, 255]);
        assert_eq!(app.active_pyxel_frame(), 1);
        assert!(app.validate_pyxel_document().is_ok());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("frames.efude");
        efude_io::save(&path, &app.doc).unwrap();
        let loaded = efude_io::load(&path).unwrap();
        assert_eq!(
            loaded.metadata.get(MODE_KEY).map(String::as_str),
            Some(MODE_VERSION)
        );
        assert_eq!(loaded.layers.len(), 2);
        assert_eq!(loaded.layers[0].pixels.pixel(3, 4), [43, 51, 95, 255]);
        assert_eq!(loaded.layers[1].pixels.pixel(3, 4), [212, 24, 108, 255]);
        assert!(!loaded.layers[0].visible && loaded.layers[1].visible);
        app.replace_document(loaded, None);
        assert!(app.validate_pyxel_document().is_ok());
        app.doc.layers[0].pixels.set_pixel(3, 4, [44, 51, 95, 255]);
        assert!(app.validate_pyxel_document().is_err());
        app.doc.layers[0].pixels.set_pixel(3, 4, [43, 51, 95, 255]);
        app.delete_pyxel_frame();
        assert_eq!(app.doc.layers.len(), 1);
        assert!(app.doc.layers[0].visible);
        app.undo();
        assert_eq!(app.doc.layers.len(), 2);
        assert!(app.doc.layers[1].visible);
    }
}
