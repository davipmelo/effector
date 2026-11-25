//! Modal sheets and windows: export, settings (general / shortcuts / export), canvas popover,
//! text prompts, alerts and the close-images confirmation.

use std::collections::HashSet;

use egui::{vec2, Color32, CornerRadius, Id, Stroke, Ui};
use uuid::Uuid;

use super::theme::*;
use super::widgets::*;
use crate::app::{AppState, Dialog, PromptKind};
use crate::imageio::Format;
use crate::l10n::{tr, trf};
use crate::model::ParamValue;
use crate::settings::{self, Language, Theme};
use crate::shortcuts::{self, Action, KeyCombo};

fn modal_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(panel_bg())
        .stroke(Stroke::new(1.0, separator()))
        .corner_radius(CornerRadius::same(14))
        .shadow(egui::Shadow { offset: [0, 12], blur: 30, spread: 0, color: Color32::from_black_alpha(110) })
        .inner_margin(22)
}

#[derive(Clone)]
struct ExportMem {
    format: Format,
    scale: usize,
    quality: f64,
    scope: String,
    suffix: String,
    init: bool,
}

pub fn export_sheet(ctx: &egui::Context, app: &mut AppState) {
    let id = Id::new("export-sheet");
    let s = settings::get();
    let mut m: ExportMem = ctx.data(|d| d.get_temp(id)).unwrap_or(ExportMem {
        format: s.export_format,
        scale: s.export_scale.max(1) as usize,
        quality: s.export_quality,
        scope: "this".into(),
        suffix: "-edit".into(),
        init: true,
    });
    let mut close = false;
    egui::Modal::new(id.with("modal")).frame(modal_frame()).backdrop_color(Color32::from_black_alpha(90)).show(ctx, |ui| {
        ui.set_width(316.0);
        ui.spacing_mut().item_spacing.y = 14.0;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(icon_text("square.and.arrow.up")).font(icon_font(18.0)).color(accent_text()));
            ui.label(egui::RichText::new(tr("Export Image")).font(semibold(16.0)).color(text()));
        });
        if app.docs.len() > 1 {
            let o = vec![
                ("this".to_string(), tr("This Image")),
                ("selected".to_string(), trf("Selected (%d)", &[&app.strip_selection.len()])),
                ("all".to_string(), trf("All (%d)", &[&app.docs.len()])),
            ];
            if let Some(v) = segmented_row(ui, "Images", &o, &m.scope) {
                m.scope = v;
            }
        }
        let fo = vec![("png".to_string(), "PNG".to_string()), ("jpg".to_string(), "JPG".to_string())];
        if let Some(v) = segmented_row(ui, "Format", &fo, m.format.ext()) {
            m.format = if v == "jpg" { Format::Jpg } else { Format::Png };
        }
        let so = vec![("1".to_string(), "1×".to_string()), ("2".to_string(), "2×".to_string())];
        if let Some(v) = segmented_row(ui, "Size", &so, &m.scale.to_string()) {
            m.scale = v.parse().unwrap_or(1);
        }
        if m.format == Format::Jpg {
            if let Some(v) = value_slider(ui, id.with("q"), SliderSpec::new("Quality", m.quality * 100.0, 50.0, 100.0, 92.0).step(1.0).unit("%")) {
                m.quality = v / 100.0;
            }
        }
        if m.scope == "this" {
            let (w, h) = app.export_size(m.scale);
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tr("Output")).font(label_font()).color(text_dim()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("{w} × {h} px")).font(value_font()).color(text()));
                });
            });
        } else {
            ui.label(egui::RichText::new(tr("File name suffix")).font(label_font()).color(text_dim()));
            ui.add(egui::TextEdit::singleline(&mut m.suffix).font(regular(12.0)).desired_width(f32::INFINITY));
            ui.label(egui::RichText::new(tr("Each image is exported with its own effects into a folder you choose.")).font(regular(11.0)).color(text_faint()));
        }
        if m.format == Format::Png && app.source().map(|s| s.has_alpha).unwrap_or(false) {
            ui.label(egui::RichText::new(format!("{}  {}", icon_text("checkerboard.rectangle"), tr("Transparency will be preserved"))).font(regular(11.0)).color(text_faint()));
        }
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let batch: HashSet<Uuid> = match m.scope.as_str() {
                    "selected" => app.strip_selection.clone(),
                    "all" => app.docs.iter().map(|d| d.id).collect(),
                    _ => HashSet::new(),
                };
                let label = if app.is_exporting {
                    app.export_progress.clone().unwrap_or_else(|| "…".into())
                } else if m.scope == "this" {
                    tr("Export…")
                } else {
                    trf("Export %d Images…", &[&batch.len()])
                };
                let go = ui.add_enabled_ui(!app.is_exporting, |ui| pill_button(ui, &label, true)).inner;
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if go.clicked() || (enter && !app.is_exporting) {
                    let (f, sc, q) = (m.format, m.scale, m.quality);
                    settings::update(|s| {
                        s.export_format = f;
                        s.export_scale = sc as i32;
                        s.export_quality = q;
                    });
                    if m.scope == "this" {
                        app.export(f, q, sc);
                    } else {
                        app.export_batch(&batch, f, q, sc, &m.suffix.clone());
                    }
                }
                if pill_button(ui, &tr("Cancel"), false).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    close = true;
                }
            });
        });
    });
    if close {
        app.show_export = false;
    }
    if app.show_export {
        m.init = false;
        ctx.data_mut(|d| d.insert_temp(id, m));
    } else {
        ctx.data_mut(|d| d.remove::<ExportMem>(id));
    }
}

pub fn canvas_popover(ctx: &egui::Context, app: &mut AppState, anchor: egui::Rect) {
    let id = Id::new("canvas-popover");
    let area = egui::Area::new(id).order(egui::Order::Foreground).fixed_pos(egui::pos2(anchor.center().x - 150.0, anchor.bottom() + 6.0)).constrain(true);
    let inner = area.show(ctx, |ui| {
        egui::Frame::new()
            .fill(panel_bg())
            .stroke(Stroke::new(1.0, separator()))
            .corner_radius(CornerRadius::same(10))
            .shadow(egui::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(110) })
            .show(ui, |ui| {
                ui.set_width(300.0);
                header(ui, &tr("Canvas & Borders"), Some("aspectratio"));
                egui::ScrollArea::vertical().max_height(560.0).show(ui, |ui| {
                    egui::Frame::new().inner_margin(14).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 10.0;
                        super::inspector::canvas_controls(ui, app);
                    });
                });
            });
    });
    let r = inner.response.rect;
    let any_popup = egui::Popup::is_any_open(ctx) || ctx.data(|d| d.get_temp::<u64>(Id::new("picker-open-frame")).map(|f| f + 1 >= ctx.cumulative_frame_nr()).unwrap_or(false));
    if !any_popup && ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().map(|p| !r.contains(p) && !anchor.contains(p)).unwrap_or(false)) {
        app.show_canvas_popover = false;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        app.show_canvas_popover = false;
    }
}

pub fn dialogs(ctx: &egui::Context, app: &mut AppState) {
    let Some(mut d) = app.dialog.take() else { return };
    let mut keep = true;
    let mut next: Option<Dialog> = None;
    match d {
        Dialog::Alert { ref title, ref message } => {
            egui::Modal::new(Id::new("alert")).frame(modal_frame()).show(ctx, |ui| {
                ui.set_width(320.0);
                ui.label(egui::RichText::new(title).font(semibold(14.0)).color(text()));
                ui.add_space(6.0);
                ui.label(egui::RichText::new(message).font(regular(12.0)).color(text_dim()));
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, &tr("OK"), true).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape)) {
                        keep = false;
                    }
                });
            });
        }
        Dialog::ConfirmClose { ref ids } => {
            egui::Modal::new(Id::new("confirm-close")).frame(modal_frame()).show(ctx, |ui| {
                ui.set_width(340.0);
                let t = if ids.len() == 1 { tr("Close this image?") } else { trf("Close %d images?", &[&ids.len()]) };
                ui.label(egui::RichText::new(t).font(semibold(14.0)).color(text()));
                ui.add_space(6.0);
                ui.label(egui::RichText::new(tr("Their effects will be removed from the workspace. Saved projects are not affected.")).font(regular(12.0)).color(text_dim()));
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, &tr("Close"), true).clicked() {
                        app.close_documents(ids);
                        keep = false;
                    }
                    if pill_button(ui, &tr("Cancel"), false).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        keep = false;
                    }
                });
            });
        }
        Dialog::Prompt { ref title, ref label, text: ref mut input, ref kind } => {
            let mut ok = false;
            egui::Modal::new(Id::new("prompt")).frame(modal_frame()).show(ctx, |ui| {
                ui.set_width(320.0);
                ui.label(egui::RichText::new(title).font(semibold(14.0)).color(text()));
                ui.add_space(4.0);
                ui.label(egui::RichText::new(label).font(regular(12.0)).color(text_dim()));
                let te = ui.add(egui::TextEdit::singleline(input).font(regular(13.0)).desired_width(f32::INFINITY));
                if !te.has_focus() && !te.lost_focus() {
                    te.request_focus();
                }
                ui.add_space(10.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, &tr("OK"), true).clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        ok = true;
                    }
                    if pill_button(ui, &tr("Cancel"), false).clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        keep = false;
                    }
                });
            });
            let name = input.trim().to_string();
            if ok {
                keep = false;
                if !name.is_empty() {
                    match kind {
                        PromptKind::SaveStackPreset => {
                            let layers = app.layers().to_vec();
                            app.add_preset(&name, &layers);
                            app.flash(&trf("Preset saved: %@", &[&name]));
                        }
                        PromptKind::SaveLayersPreset(layers) => {
                            app.add_preset(&name, layers);
                            app.flash(&tr("Preset saved"));
                        }
                        PromptKind::RenamePreset(id) => {
                            crate::presets::rename(*id, &name);
                        }
                        PromptKind::SaveGradient { layer, key, gradient } => {
                            let mut g = gradient.clone();
                            g.name = name.clone();
                            crate::swatches::save_gradient(g.clone());
                            app.set_param(*layer, key, ParamValue::Gradient(g), false);
                        }
                        PromptKind::SavePalette { layer, key, palette } => {
                            let mut p = palette.clone();
                            p.name = name.clone();
                            crate::swatches::save_palette(p.clone());
                            app.set_param(*layer, key, ParamValue::Palette(p), false);
                        }
                    }
                }
            }
        }
    }
    if keep && next.is_none() {
        next = Some(d);
    }
    if app.dialog.is_none() {
        app.dialog = next;
    }
}

// MARK: - Settings window

pub fn settings_window(ctx: &egui::Context, app: &mut AppState) {
    let id = Id::new("settings-win");
    let mut tab: usize = ctx.data(|d| d.get_temp(id.with("tab")).unwrap_or(0));
    let mut open = true;
    egui::Window::new(tr("Settings"))
        .id(id)
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_size(vec2(520.0, 520.0))
        .frame(egui::Frame::window(&ctx.global_style()).fill(panel_bg()).inner_margin(16))
        .show(ctx, |ui| {
            ui.set_width(488.0);
            ui.set_min_height(460.0);
            let tabs = vec![("0".to_string(), tr("General")), ("1".to_string(), tr("Shortcuts")), ("2".to_string(), tr("Export"))];
            if let Some(v) = segmented_row(ui, "", &tabs, &tab.to_string()) {
                tab = v.parse().unwrap_or(0);
            }
            ui.add_space(8.0);
            let s = settings::get();
            match tab {
                0 => {
                    labeled_row(ui, "Language", |ui| {
                        let items = vec![
                            PickerItem::new("en", "English", None),
                            PickerItem::new("pt", "Português", None),
                            PickerItem::new("system", &tr("Follow System"), None),
                        ];
                        let cur = match s.language {
                            Language::En => "en",
                            Language::Pt => "pt",
                            Language::System => "system",
                        };
                        if let Some(v) = picker_field(ui, "settings.language", &items, cur, None, &[], false).selected {
                            let l = match v.as_str() {
                                "pt" => Language::Pt,
                                "system" => Language::System,
                                _ => Language::En,
                            };
                            settings::update(|s| s.language = l);
                        }
                    });
                    let themes = vec![("dark".to_string(), tr("Dark")), ("light".to_string(), tr("Light")), ("system".to_string(), tr("Follow System"))];
                    let cur = match s.theme {
                        Theme::Dark => "dark",
                        Theme::Light => "light",
                        Theme::System => "system",
                    };
                    if let Some(v) = segmented_row(ui, "Appearance", &themes, cur) {
                        let t = match v.as_str() {
                            "light" => Theme::Light,
                            "system" => Theme::System,
                            _ => Theme::Dark,
                        };
                        settings::update(|s| s.theme = t);
                    }
                    ui.add_space(6.0);
                    if let Some(v) = toggle_row(ui, "Reopen the last image and layers on launch", s.reopen_last_session) {
                        settings::update(|s| s.reopen_last_session = v);
                    }
                    if let Some(v) = toggle_row(ui, "Preview effects and presets on hover", s.hover_preview) {
                        settings::update(|s| s.hover_preview = v);
                    }
                    ui.label(
                        egui::RichText::new(tr("Settings, shortcuts, favorites, presets, palettes, gradients and LUTs are always saved automatically."))
                            .font(regular(11.0))
                            .color(text_faint()),
                    );
                    if let Some(v) = toggle_row(ui, "Show built-in presets", s.factory_presets_visible) {
                        settings::update(|s| s.factory_presets_visible = v);
                        app.regenerate_preset_thumbs();
                    }
                }
                1 => shortcut_settings(ui),
                _ => {
                    let fo = vec![("png".to_string(), "PNG".to_string()), ("jpg".to_string(), "JPG".to_string())];
                    if let Some(v) = segmented_row(ui, "Default format", &fo, s.export_format.ext()) {
                        settings::update(|s| s.export_format = if v == "jpg" { Format::Jpg } else { Format::Png });
                    }
                    let so = vec![("1".to_string(), "1×".to_string()), ("2".to_string(), "2×".to_string())];
                    if let Some(v) = segmented_row(ui, "Default scale", &so, &s.export_scale.to_string()) {
                        settings::update(|s| s.export_scale = v.parse().unwrap_or(1));
                    }
                    if let Some(v) = value_slider(ui, id.with("jq"), SliderSpec::new("JPG quality", s.export_quality * 100.0, 50.0, 100.0, 92.0).step(1.0).unit("%")) {
                        settings::update(|s| s.export_quality = v / 100.0);
                    }
                }
            }
        });
    ctx.data_mut(|d| d.insert_temp(id.with("tab"), tab));
    if !open {
        app.show_settings = false;
    }
}

/// Shortcut list; the action being recorded is kept in temp memory.
fn shortcut_settings(ui: &mut Ui) {
    let rec_id = Id::new("recording-shortcut");
    let mut recording: Option<usize> = ui.data(|d| d.get_temp(rec_id));
    // capture the next key press
    if let Some(i) = recording {
        let ev = ui.input(|inp| {
            inp.events.iter().find_map(|e| if let egui::Event::Key { key, pressed: true, modifiers, .. } = e { Some((*key, *modifiers)) } else { None })
        });
        if let Some((key, m)) = ev {
            let a = Action::ALL[i];
            if let Some(c) = KeyCombo::from_event(key, m) {
                if c.key == "escape" && !c.has_modifier() {
                    recording = None;
                } else if (c.key == "delete" || c.key == "fdelete") && !c.has_modifier() {
                    shortcuts::set(a, None);
                    recording = None;
                } else {
                    shortcuts::set(a, Some(c));
                    recording = None;
                }
            }
        }
    }
    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
        for (i, a) in Action::ALL.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new(tr(a.label())).font(regular(12.5)).color(text()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let rec = recording == Some(i);
                    let t = if rec { tr("Type shortcut…") } else { shortcuts::combo(*a).map(|c| c.display()).unwrap_or_else(|| "—".into()) };
                    let g = ui.painter().layout_no_wrap(t, mono(12.0), text());
                    let w = g.size().x.max(90.0) + 12.0;
                    let (r, resp) = ui.allocate_exact_size(vec2(w, 22.0), egui::Sense::click());
                    ui.painter().rect_filled(r, CornerRadius::same(5), if rec { with_alpha(ACCENT, 0.3) } else { field() });
                    ui.painter().galley(r.center() - g.size() / 2.0, g, text());
                    if resp.clicked() {
                        recording = if rec { None } else { Some(i) };
                    }
                });
            });
        }
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(keys_hint(&tr("Click a shortcut, then press the new keys. ⎋ cancels, ⌫ clears."))).font(regular(11.0)).color(text_faint()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if pill_button(ui, &tr("Restore Defaults"), false).clicked() {
                shortcuts::reset_all();
            }
        });
    });
    ui.data_mut(|d| match recording {
        Some(r) => {
            d.insert_temp(rec_id, r);
        }
        None => d.remove::<usize>(rec_id),
    });
}

/// True while a shortcut is being recorded (global shortcuts must not fire).
pub fn is_recording_shortcut(ctx: &egui::Context) -> bool { ctx.data(|d| d.get_temp::<usize>(Id::new("recording-shortcut")).is_some()) }
