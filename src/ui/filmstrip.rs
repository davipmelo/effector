//! Lightroom-style strip of open images under the canvas, with the Sync panel.

use std::collections::HashSet;

use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use uuid::Uuid;

use super::theme::*;
use super::widgets::*;
use crate::app::{AppState, SyncOptions};
use crate::l10n::{tr, trf};

pub fn filmstrip(ui: &mut Ui, app: &mut AppState) {
    let rect = ui.available_rect_before_wrap();
    ui.painter().rect_filled(rect, CornerRadius::ZERO, panel_bg());
    ui.painter().rect_filled(Rect::from_min_size(rect.min, vec2(rect.width(), 1.0)), CornerRadius::ZERO, separator());
    ui.horizontal(|ui| {
        ui.add_space(6.0);
        ui.vertical(|ui| {
            ui.add_space(14.0);
            if icon_button(ui, "plus", &keys_hint("Add images… (⌘O)"), false, 13.0).clicked() {
                app.open_panel();
            }
            let others: HashSet<Uuid> = app.strip_selection.iter().copied().filter(|i| Some(*i) != app.active_id).collect();
            let sync = ui.add_enabled_ui(!others.is_empty(), |ui| icon_button(ui, "arrow.triangle.2.circlepath", &keys_hint("Sync settings from the active image to the selected ones (⌥⌘S)"), false, 13.0)).inner;
            if sync.clicked() {
                app.show_sync = true;
            }
            if app.show_sync {
                sync_panel(ui, app, sync.rect);
            }
        });
        ui.add_space(6.0);
        let (sep, _) = ui.allocate_exact_size(vec2(1.0, rect.height()), Sense::hover());
        ui.painter().rect_filled(sep, CornerRadius::ZERO, separator());
        egui::ScrollArea::horizontal().id_salt("filmstrip").auto_shrink([false, false]).show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(8.0);
                ui.spacing_mut().item_spacing.x = 6.0;
                let ids: Vec<Uuid> = app.docs.iter().map(|d| d.id).collect();
                for (i, id) in ids.iter().enumerate() {
                    cell(ui, app, *id, i + 1);
                }
                ui.add_space(8.0);
            });
        });
    });
}

fn cell(ui: &mut Ui, app: &mut AppState, id: Uuid, index: usize) {
    let (r, resp) = ui.allocate_exact_size(vec2(104.0, 70.0), Sense::click());
    let active = app.active_id == Some(id);
    let selected = app.strip_selection.contains(&id);
    let hover = resp.hovered();
    let Some(doc) = app.docs.iter().find(|d| d.id == id) else { return };
    let opacity = if active || selected || hover { 1.0 } else { 0.82 };
    ui.painter().rect_filled(r, CornerRadius::same(5), field());
    match &doc.thumb {
        Some(t) => {
            let ts = t.size_vec2();
            let s = (r.width() / ts.x).min(r.height() / ts.y);
            let ir = Rect::from_center_size(r.center(), ts * s);
            ui.painter().image(t.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), with_alpha(Color32::WHITE, opacity));
        }
        None => {
            ui.put(Rect::from_center_size(r.center(), vec2(16.0, 16.0)), egui::Spinner::new().size(14.0));
        }
    }
    let n = format!("{index}");
    let g = ui.painter().layout_no_wrap(n, bold(9.0), Color32::WHITE);
    let br = Rect::from_min_size(r.min + vec2(3.0, 3.0), g.size() + vec2(8.0, 2.0));
    ui.painter().rect_filled(br, CornerRadius::same(255), Color32::from_black_alpha(140));
    ui.painter().galley(br.min + vec2(4.0, 1.0), g, Color32::WHITE);
    let has_edits = !doc.state.layers.is_empty() || doc.state.active_canvas().is_some();
    if has_edits {
        let c = pos2(r.right() - 10.0, r.bottom() - 10.0);
        ui.painter().circle_filled(c, 8.0, Color32::from_black_alpha(140));
        paint_icon(ui, Rect::from_center_size(c, vec2(10.0, 10.0)), "wand.and.stars", 9.0, ACCENT);
    }
    let (stroke_c, sw) = if active {
        (ACCENT, 2.0)
    } else if selected {
        (with_alpha(ACCENT, 0.55), 1.5)
    } else if hover {
        (text_faint(), 1.0)
    } else {
        (separator(), 1.0)
    };
    ui.painter().rect_stroke(r, CornerRadius::same(5), Stroke::new(sw, stroke_c), StrokeKind::Inside);
    let name = doc.file_name.clone();
    let file_path = doc.file_path.clone();
    if resp.clicked() {
        let m = ui.input(|i| i.modifiers);
        app.strip_click(id, m);
    }
    let resp = resp.on_hover_text(format!("{name} · {}", keys_hint(&tr("click to open · ⌘/⇧-click to select several"))));
    resp.context_menu(|ui| {
        let targets: HashSet<Uuid> = if app.strip_selection.contains(&id) { app.strip_selection.clone() } else { [id].into_iter().collect() };
        if ui.button(tr("Open")).clicked() {
            app.activate(id);
            ui.close();
        }
        ui.separator();
        if Some(id) != app.active_id && ui.button(tr("Sync Settings from Active Image")).clicked() {
            app.sync_settings(&targets, SyncOptions { layers: true, canvas: true, ..Default::default() });
            ui.close();
        }
        if ui.button(tr("Copy Settings")).clicked() {
            app.activate(id);
            app.copy_settings();
            ui.close();
        }
        if ui.add_enabled(app.has_copied_settings(), egui::Button::new(tr("Paste Settings"))).clicked() {
            app.paste_settings(&targets);
            ui.close();
        }
        if ui.button(tr("Reset Settings")).clicked() {
            app.reset_settings(&targets);
            ui.close();
        }
        ui.separator();
        if let Some(p) = &file_path {
            let label = if crate::shortcuts::IS_MAC { tr("Show in Finder") } else { tr("Show in Explorer") };
            if ui.button(label).clicked() {
                reveal(p);
                ui.close();
            }
        }
        if ui.button(tr("Select All Images")).clicked() {
            app.strip_selection = app.docs.iter().map(|d| d.id).collect();
            ui.close();
        }
        ui.separator();
        let label = if targets.len() > 1 { trf("Close %d Images", &[&targets.len()]) } else { tr("Close Image") };
        if ui.button(egui::RichText::new(label).color(Color32::from_rgb(235, 80, 70))).clicked() {
            app.request_close(targets);
            ui.close();
        }
    });
}

/// Opens the file manager with the file selected.
pub fn reveal(p: &std::path::Path) {
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(format!("/select,{}", p.display())).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg("-R").arg(p).spawn();
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(d) = p.parent() {
            let _ = open::that(d);
        }
    }
}

#[derive(Clone, Copy)]
struct SyncMem {
    layers: bool,
    add_on_top: bool,
    crop: bool,
    canvas: bool,
}

/// Choose what to copy from the active image to the selected ones.
pub fn sync_panel(ui: &mut Ui, app: &mut AppState, anchor: Rect) {
    let id = Id::new("sync-panel");
    let mut m: SyncMem = ui.data(|d| d.get_temp(id)).unwrap_or(SyncMem { layers: true, add_on_top: false, crop: false, canvas: true });
    let targets: HashSet<Uuid> = app.strip_selection.iter().copied().filter(|i| Some(*i) != app.active_id).collect();
    let mut close = false;
    let area = egui::Area::new(id.with("area")).order(egui::Order::Foreground).fixed_pos(pos2(anchor.left(), anchor.top() - 290.0)).constrain(true);
    let inner = area.show(ui.ctx(), |ui| {
        egui::Frame::new()
            .fill(panel_bg())
            .stroke(Stroke::new(1.0, separator()))
            .corner_radius(CornerRadius::same(10))
            .shadow(egui::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(110) })
            .inner_margin(16)
            .show(ui, |ui| {
                ui.set_width(300.0);
                ui.spacing_mut().item_spacing.y = 12.0;
                ui.label(egui::RichText::new(tr("Sync Settings")).font(semibold(13.0)).color(text()));
                ui.label(egui::RichText::new(trf("Copy from the active image to %d selected images.", &[&targets.len()])).font(regular(11.0)).color(text_dim()));
                if let Some(v) = toggle_row(ui, "Layers & Effects", m.layers) {
                    m.layers = v;
                }
                if m.layers {
                    let o = [("replace".to_string(), "Replace Their Layers".to_string()), ("add".to_string(), "Add on Top".to_string())];
                    if let Some(v) = segmented_row(ui, "", &o, if m.add_on_top { "add" } else { "replace" }) {
                        m.add_on_top = v == "add";
                    }
                }
                if let Some(v) = toggle_row(ui, "Crop", m.crop) {
                    m.crop = v;
                }
                if let Some(v) = toggle_row(ui, "Canvas & Borders", m.canvas) {
                    m.canvas = v;
                }
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let ok = !targets.is_empty() && (m.layers || m.crop || m.canvas);
                        if ui.add_enabled_ui(ok, |ui| pill_button(ui, &trf("Sync to %d Images", &[&targets.len()]), true)).inner.clicked() {
                            app.sync_settings(&targets, SyncOptions { layers: m.layers, add_on_top: m.add_on_top, crop: m.crop, canvas: m.canvas });
                            close = true;
                        }
                        if pill_button(ui, &tr("Cancel"), false).clicked() {
                            close = true;
                        }
                    });
                });
            });
    });
    let r = inner.response.rect;
    if ui.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().map(|p| !r.contains(p) && !anchor.contains(p)).unwrap_or(false)) {
        close = true;
    }
    if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
        close = true;
    }
    if close {
        app.show_sync = false;
    }
    ui.data_mut(|d| d.insert_temp(id, m));
    let _ = Align2::CENTER_CENTER;
}
