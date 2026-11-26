//! Export dialog (Lightroom Classic style): export presets on the left; Export Location, File
//! Naming (template made of blocks), File Settings, Image Sizing and After Export on the right.

use std::collections::HashSet;
use std::path::PathBuf;

use egui::{pos2, vec2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use uuid::Uuid;

use super::theme::*;
use super::widgets::*;
use crate::app::AppState;
use crate::export::*;
use crate::imageio::Format;
use crate::l10n::{tr, trf};
use crate::settings;

#[derive(Clone)]
struct Mem {
    s: ExportSettings,
    scope: String,
    preset: Option<Uuid>,
    renaming: Option<(Uuid, String)>,
    /// Files that already exist (asks what to do), with the folder picked for "Choose later".
    conflict: Option<(usize, Option<PathBuf>)>,
    error: Option<String>,
    text_block: String,
}

const LEFT_W: f32 = 196.0;

fn reveal_label() -> String { tr(if cfg!(target_os = "macos") { "Show in Finder" } else { "Show in Explorer" }) }

pub fn export_dialog(ctx: &egui::Context, app: &mut AppState) {
    let id = Id::new("export-dialog");
    let mut m: Mem = ctx.data(|d| d.get_temp(id)).unwrap_or_else(|| Mem {
        s: settings::get().export,
        scope: if app.strip_selection.len() > 1 { "selected".into() } else { "this".into() },
        preset: None,
        renaming: None,
        conflict: None,
        error: None,
        text_block: String::new(),
    });
    let ids: HashSet<Uuid> = match m.scope.as_str() {
        "selected" if !app.strip_selection.is_empty() => app.strip_selection.clone(),
        "all" => app.docs.iter().map(|d| d.id).collect(),
        _ => app.active_id.into_iter().collect(),
    };
    let mut close = false;
    let screen = ctx.content_rect();
    let height = (screen.height() - 90.0).clamp(420.0, 700.0);
    let width = (screen.width() - 80.0).clamp(640.0, 900.0);
    egui::Modal::new(id.with("modal"))
        .frame(egui::Frame::new().fill(panel_bg()).stroke(Stroke::new(1.0, separator())).corner_radius(CornerRadius::same(14)).shadow(egui::Shadow {
            offset: [0, 12],
            blur: 30,
            spread: 0,
            color: Color32::from_black_alpha(110),
        }))
        .backdrop_color(Color32::from_black_alpha(90))
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.set_height(height);
            ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
            // title bar
            egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: 16, bottom: 12 }).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon_text("square.and.arrow.up")).font(icon_font(18.0)).color(accent_text()));
                    let t = if ids.len() == 1 { tr("Export 1 Image") } else { trf("Export %d Images", &[&ids.len()]) };
                    ui.label(egui::RichText::new(t).font(semibold(16.0)).color(text()));
                    if app.docs.len() > 1 {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.set_max_width(330.0);
                            let o = vec![
                                ("this".to_string(), tr("This Image")),
                                ("selected".to_string(), trf("Selected (%d)", &[&app.strip_selection.len().max(1)])),
                                ("all".to_string(), trf("All (%d)", &[&app.docs.len()])),
                            ];
                            if let Some(v) = segmented_row(ui, "", &o, &m.scope) {
                                m.scope = v;
                            }
                        });
                    }
                });
            });
            separator_line(ui);
            let body_h = height - 60.0 - 64.0;
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                // presets
                ui.allocate_ui_with_layout(vec2(LEFT_W, body_h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_min_size(vec2(LEFT_W, body_h));
                    presets_column(ui, &mut m);
                });
                let (sep, _) = ui.allocate_exact_size(vec2(1.0, body_h), Sense::hover());
                ui.painter().rect_filled(sep, CornerRadius::ZERO, separator());
                ui.allocate_ui_with_layout(vec2(ui.available_width(), body_h), egui::Layout::top_down(egui::Align::Min), |ui| {
                    ui.set_min_size(vec2(ui.available_width(), body_h));
                    egui::ScrollArea::vertical().id_salt("export-scroll").auto_shrink([false, false]).show(ui, |ui| {
                        let before = m.s.clone();
                        sections(ui, app, &mut m, &ids);
                        if m.s != before {
                            m.error = None;
                        }
                    });
                });
            });
            separator_line(ui);
            // bottom bar
            egui::Frame::new().inner_margin(egui::Margin { left: 20, right: 20, top: 12, bottom: 14 }).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(e) = &m.error {
                        ui.label(egui::RichText::new(e).font(regular(11.5)).color(Color32::from_rgb(235, 90, 80)));
                    } else {
                        ui.label(egui::RichText::new(summary(app, &m.s, &ids)).font(regular(11.5)).color(text_faint()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let label = if app.is_exporting { app.export_progress.clone().unwrap_or_else(|| "…".into()) } else { tr("Export") };
                        let go = ui.add_enabled_ui(!app.is_exporting && !ids.is_empty(), |ui| pill_button(ui, &label, true)).inner;
                        if go.clicked() {
                            begin(app, &mut m, &ids);
                        }
                        if pill_button(ui, &tr("Cancel"), false).clicked() {
                            close = true;
                        }
                    });
                });
            });
            if let Some((n, dir)) = m.conflict.clone() {
                conflict_overlay(ui, app, &mut m, &ids, n, dir);
            }
            let typing = ui.ctx().egui_wants_keyboard_input();
            if !typing && !egui::Popup::is_any_open(ui.ctx()) && ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                if m.conflict.is_some() {
                    m.conflict = None;
                } else {
                    close = true;
                }
            }
        });
    if m.s != settings::get().export {
        let s = m.s.clone();
        settings::update(|x| x.export = s);
    }
    if close {
        app.show_export = false;
    }
    if app.show_export {
        ctx.data_mut(|d| d.insert_temp(id, m));
    } else {
        ctx.data_mut(|d| d.remove::<Mem>(id));
    }
}

fn summary(app: &AppState, s: &ExportSettings, ids: &HashSet<Uuid>) -> String {
    let mut parts = vec![];
    parts.push(if s.format == Format::Jpg { format!("JPG {}%", (s.quality * 100.0).round() as i64) } else { "PNG".into() });
    if let Some(src) = app.source().filter(|_| ids.contains(&app.active_id.unwrap_or_default())) {
        let (w, h) = full_size(&src, &app.state());
        let (ow, oh) = s.out_size(w, h);
        parts.push(format!("{ow} × {oh} px"));
    } else if s.resize {
        parts.push(tr("Resized"));
    }
    if s.format == Format::Jpg && s.limit_size {
        parts.push(format!("≤ {} KB", s.limit_kb));
    }
    parts.join("  ·  ")
}

/// Starts the export: picks the folder if needed, checks for existing files.
fn begin(app: &mut AppState, m: &mut Mem, ids: &HashSet<Uuid>) {
    m.error = None;
    let jobs = app.export_jobs(ids);
    let needs_dir = m.s.location == Location::Ask || (m.s.location == Location::SameAsOriginal && jobs.iter().any(|j| j.original_dir().is_none()));
    let mut ask_dir = None;
    if needs_dir {
        let title = if jobs.len() == 1 { tr("Choose a folder for the image") } else { trf("Choose a folder for %d images", &[&jobs.len()]) };
        match rfd::FileDialog::new().set_title(title).pick_folder() {
            Some(d) => ask_dir = Some(d),
            None => return,
        }
    }
    match plan(&m.s, &jobs, ask_dir.as_deref()) {
        Err(e) => m.error = Some(e),
        Ok(planned) => {
            let n = planned.iter().filter(|p| p.exists).count();
            if n > 0 && m.s.existing == Existing::Ask {
                m.conflict = Some((n, ask_dir));
            } else {
                let mode = if m.s.existing == Existing::Ask { Existing::Rename } else { m.s.existing };
                app.start_export(m.s.clone(), jobs, planned, mode);
            }
        }
    }
}

fn conflict_overlay(ui: &mut Ui, app: &mut AppState, m: &mut Mem, ids: &HashSet<Uuid>, n: usize, dir: Option<PathBuf>) {
    let full = ui.min_rect();
    ui.painter().rect_filled(full, CornerRadius::same(14), Color32::from_black_alpha(120));
    let r = Rect::from_center_size(full.center(), vec2(380.0, 150.0));
    let mut choice: Option<Existing> = None;
    ui.scope_builder(egui::UiBuilder::new().max_rect(r), |ui| {
        egui::Frame::new().fill(panel_bg2()).stroke(Stroke::new(1.0, separator())).corner_radius(CornerRadius::same(12)).inner_margin(18).show(ui, |ui| {
            ui.set_width(r.width() - 36.0);
            let t = if n == 1 { tr("1 file already exists") } else { trf("%d files already exist", &[&n]) };
            ui.label(egui::RichText::new(t).font(semibold(14.0)).color(text()));
            ui.label(egui::RichText::new(tr("What should EFFECTOR do with them?")).font(regular(12.0)).color(text_dim()));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if pill_button(ui, &tr("Cancel"), false).clicked() {
                    m.conflict = None;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if pill_button(ui, &tr("Keep Both"), true).on_hover_text(tr("The new files get a number at the end")).clicked() {
                        choice = Some(Existing::Rename);
                    }
                    if pill_button(ui, &tr("Overwrite"), false).clicked() {
                        choice = Some(Existing::Overwrite);
                    }
                    if pill_button(ui, &tr("Skip"), false).clicked() {
                        choice = Some(Existing::Skip);
                    }
                });
            });
        });
    });
    if let Some(c) = choice {
        m.conflict = None;
        let jobs = app.export_jobs(ids);
        match plan(&m.s, &jobs, dir.as_deref()) {
            Ok(planned) => app.start_export(m.s.clone(), jobs, planned, c),
            Err(e) => m.error = Some(e),
        }
    }
}

// MARK: - Presets column

fn presets_column(ui: &mut Ui, m: &mut Mem) {
    let presets = settings::get().export_presets;
    egui::Frame::new().inner_margin(egui::Margin { left: 12, right: 8, top: 12, bottom: 8 }).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(spaced_upper(&tr("Export Presets"))).font(section_font()).color(text_dim()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, "plus", "Save the current settings as a preset", false, 12.0).clicked() {
                    let name = (1..1000).map(|i| format!("{} {i}", tr("Preset"))).find(|n| !presets.iter().any(|p| &p.name == n)).unwrap_or_default();
                    let p = ExportPreset { id: Uuid::new_v4(), name: name.clone(), settings: m.s.clone() };
                    let pid = p.id;
                    settings::update(|s| s.export_presets.push(p));
                    m.preset = Some(pid);
                    m.renaming = Some((pid, name));
                }
            });
        });
    });
    egui::ScrollArea::vertical().id_salt("export-presets").auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::new().inner_margin(egui::Margin { left: 8, right: 8, top: 0, bottom: 8 }).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            if presets.is_empty() {
                egui::Frame::new().inner_margin(egui::Margin::symmetric(4, 2)).show(ui, |ui| {
                    ui.label(egui::RichText::new(tr("No presets yet. Set everything up and press + to save it for next time.")).font(regular(11.0)).color(text_faint()));
                });
            }
            for p in &presets {
                let sel = m.preset == Some(p.id);
                let w = ui.available_width();
                if let Some((rid, mut text)) = m.renaming.clone().filter(|r| r.0 == p.id) {
                    let te = ui.add(egui::TextEdit::singleline(&mut text).font(regular(12.0)).desired_width(w));
                    if !te.has_focus() && !te.lost_focus() {
                        te.request_focus();
                    }
                    if te.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let n = text.trim().to_string();
                        if !n.is_empty() {
                            settings::update(|s| {
                                if let Some(x) = s.export_presets.iter_mut().find(|x| x.id == rid) {
                                    x.name = n.clone();
                                }
                            });
                        }
                        m.renaming = None;
                    } else {
                        m.renaming = Some((rid, text));
                    }
                    continue;
                }
                let (r, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
                if sel {
                    ui.painter().rect_filled(r, CornerRadius::same(6), row_selected());
                } else if resp.hovered() {
                    ui.painter().rect_filled(r, CornerRadius::same(6), row_hover());
                }
                let modified = sel && p.settings != m.s;
                let mut job = egui::text::LayoutJob::simple_singleline(p.name.clone(), if sel { medium(12.0) } else { regular(12.0) }, text());
                job.wrap = egui::text::TextWrapping::truncate_at_width(r.width() - 30.0);
                let g = ui.painter().layout_job(job);
                ui.painter().galley(pos2(r.left() + 10.0, r.center().y - g.size().y / 2.0), g, text());
                if modified {
                    ui.painter().circle_filled(pos2(r.right() - 10.0, r.center().y), 3.0, ACCENT);
                }
                let resp = if modified { resp.on_hover_text(tr("Changed since the preset was saved · right-click › Update")) } else { resp };
                if resp.clicked() {
                    m.s = p.settings.clone();
                    m.preset = Some(p.id);
                }
                if resp.double_clicked() {
                    m.renaming = Some((p.id, p.name.clone()));
                }
                let pid = p.id;
                let cur = m.s.clone();
                let mut rename = false;
                resp.context_menu(|ui| {
                    if ui.button(tr("Update with Current Settings")).clicked() {
                        settings::update(|s| {
                            if let Some(x) = s.export_presets.iter_mut().find(|x| x.id == pid) {
                                x.settings = cur.clone();
                            }
                        });
                        ui.close();
                    }
                    if ui.button(tr("Rename…")).clicked() {
                        rename = true;
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(egui::RichText::new(tr("Delete")).color(Color32::from_rgb(235, 80, 70))).clicked() {
                        settings::update(|s| s.export_presets.retain(|x| x.id != pid));
                        ui.close();
                    }
                });
                if rename {
                    m.renaming = Some((p.id, p.name.clone()));
                }
            }
        });
    });
}

// MARK: - Sections

fn sections(ui: &mut Ui, app: &mut AppState, m: &mut Mem, ids: &HashSet<Uuid>) {
    let section = |ui: &mut Ui, key: &str, title: &str, add: &mut dyn FnMut(&mut Ui)| {
        panel_section(ui, Id::new(("export-sec", key)), title, None, None, |ui| {
            ui.spacing_mut().item_spacing.y = 9.0;
            add(ui);
            ui.add_space(4.0);
        });
    };
    section(ui, "loc", "Export Location", &mut |ui| location(ui, m));
    section(ui, "name", "File Naming", &mut |ui| naming(ui, app, m, ids));
    section(ui, "file", "File Settings", &mut |ui| file_settings(ui, m));
    section(ui, "size", "Image Sizing", &mut |ui| sizing(ui, app, m));
    section(ui, "after", "After Export", &mut |ui| {
        let o = vec![("nothing".to_string(), tr("Do Nothing")), ("reveal".to_string(), reveal_label())];
        if let Some(v) = segmented_row(ui, "", &o, if m.s.after == After::Reveal { "reveal" } else { "nothing" }) {
            m.s.after = if v == "reveal" { After::Reveal } else { After::Nothing };
        }
    });
}

fn picker_row<T: Copy + PartialEq>(ui: &mut Ui, label: &str, scope: &str, options: &[(T, &str)], cur: T) -> Option<T> {
    let mut out = None;
    labeled_row(ui, label, |ui| {
        let items: Vec<PickerItem> = options.iter().enumerate().map(|(i, (_, l))| PickerItem::new(&i.to_string(), &tr(l), None)).collect();
        let sel = options.iter().position(|(v, _)| *v == cur).unwrap_or(0).to_string();
        if let Some(v) = picker_field(ui, scope, &items, &sel, None, &[], false).selected {
            out = v.parse::<usize>().ok().and_then(|i| options.get(i)).map(|o| o.0);
        }
    });
    out
}

fn number_row(ui: &mut Ui, label: &str, v: &mut f64, range: std::ops::RangeInclusive<f64>, decimals: usize, suffix: &str) {
    labeled_row(ui, label, |ui| {
        if !suffix.is_empty() {
            ui.label(egui::RichText::new(suffix).font(regular(11.5)).color(text_dim()));
        }
        ui.add_sized(vec2(84.0, 22.0), egui::DragValue::new(v).range(range).max_decimals(decimals).speed(if decimals == 0 { 1.0 } else { 0.1 }));
    });
}

fn location(ui: &mut Ui, m: &mut Mem) {
    let opts = [(Location::Folder, "Specific Folder"), (Location::SameAsOriginal, "Same Folder as Original"), (Location::Ask, "Choose Folder Later")];
    if let Some(v) = picker_row(ui, "Export To", "export.location", &opts, m.s.location) {
        m.s.location = v;
    }
    if m.s.location == Location::Folder {
        ui.horizontal(|ui| {
            let w = ui.available_width() - 90.0;
            let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
            ui.painter().rect_filled(r, CornerRadius::same(6), field());
            let t = if m.s.folder.is_empty() { tr("No folder chosen") } else { m.s.folder.clone() };
            let mut job = egui::text::LayoutJob::simple_singleline(t, regular(11.5), if m.s.folder.is_empty() { text_faint() } else { text() });
            job.wrap = egui::text::TextWrapping { max_width: r.width() - 16.0, max_rows: 1, break_anywhere: true, overflow_character: Some('…') };
            let g = ui.painter().layout_job(job);
            ui.painter().galley(pos2(r.left() + 8.0, r.center().y - g.size().y / 2.0), g, text());
            if pill_button(ui, &tr("Choose…"), false).clicked() {
                let mut d = rfd::FileDialog::new();
                if !m.s.folder.is_empty() {
                    d = d.set_directory(&m.s.folder);
                }
                if let Some(p) = d.pick_folder() {
                    m.s.folder = p.to_string_lossy().to_string();
                }
            }
        });
    } else if m.s.location == Location::SameAsOriginal {
        ui.label(egui::RichText::new(tr("Pasted images have no folder of their own: EFFECTOR asks for one.")).font(regular(11.0)).color(text_faint()));
    } else {
        ui.label(egui::RichText::new(tr("EFFECTOR asks for a folder when you press Export.")).font(regular(11.0)).color(text_faint()));
    }
    if let Some(v) = toggle_row(ui, "Put in Subfolder", m.s.use_subfolder) {
        m.s.use_subfolder = v;
    }
    if m.s.use_subfolder {
        ui.add(egui::TextEdit::singleline(&mut m.s.subfolder).hint_text(tr("Subfolder name")).font(regular(12.0)).desired_width(f32::INFINITY));
    }
    let opts = [(Existing::Ask, "Ask What to Do"), (Existing::Rename, "Choose a New Name"), (Existing::Overwrite, "Overwrite Without Warning"), (Existing::Skip, "Skip")];
    if let Some(v) = picker_row(ui, "Existing Files", "export.existing", &opts, m.s.existing) {
        m.s.existing = v;
    }
}

// MARK: - File naming (blocks)

fn naming(ui: &mut Ui, app: &AppState, m: &mut Mem, ids: &HashSet<Uuid>) {
    let presets = template_presets();
    let cur = presets.iter().position(|(_, t)| *t == m.s.template);
    labeled_row(ui, "Template", |ui| {
        let mut items: Vec<PickerItem> = presets.iter().enumerate().map(|(i, (n, _))| PickerItem::new(&i.to_string(), &tr(n), None)).collect();
        if cur.is_none() {
            items.push(PickerItem::new("custom", &tr("Custom"), None));
        }
        let sel = cur.map(|i| i.to_string()).unwrap_or_else(|| "custom".into());
        if let Some(v) = picker_field(ui, "export.template", &items, &sel, None, &[], false).selected {
            if let Some((_, t)) = v.parse::<usize>().ok().and_then(|i| presets.get(i)) {
                m.s.template = t.clone();
            }
        }
    });
    // the blocks
    let mut action: Option<(usize, BlockAction)> = None;
    egui::Frame::new().fill(field()).corner_radius(CornerRadius::same(8)).inner_margin(8).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 6.0);
            if m.s.template.is_empty() {
                ui.label(egui::RichText::new(tr("Add blocks below to build the name")).font(regular(11.5)).color(text_faint()));
            }
            let n = m.s.template.len();
            for i in 0..n {
                let tok = m.s.template[i].clone();
                if let Some(a) = block_chip(ui, i, &tok, n) {
                    action = Some((i, a));
                }
            }
        });
    });
    if let Some((i, a)) = action {
        match a {
            BlockAction::Remove => {
                m.s.template.remove(i);
            }
            BlockAction::Left if i > 0 => m.s.template.swap(i, i - 1),
            BlockAction::Right if i + 1 < m.s.template.len() => m.s.template.swap(i, i + 1),
            BlockAction::Set(t) => m.s.template[i] = t,
            _ => {}
        }
    }
    // add blocks
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(4.0, 6.0);
        let add = pill_icon_button(ui, "plus", &tr("Add Block"), false);
        egui::Popup::menu(&add).show(|ui| {
            ui.set_min_width(210.0);
            let mut push = |t: NameToken| m.s.template.push(t);
            if ui.button(tr("Filename")).clicked() {
                push(NameToken::Filename);
                ui.close();
            }
            if ui.button(tr("Custom Text")).clicked() {
                push(NameToken::Custom);
                ui.close();
            }
            if ui.button(tr("Sequence (001)")).clicked() {
                push(NameToken::Sequence(3));
                ui.close();
            }
            if ui.button(format!("{} (YYYY-MM-DD)", tr("Export Date"))).clicked() {
                push(NameToken::Date("YYYY-MM-DD".into()));
                ui.close();
            }
            if ui.button(format!("{} (YYYY-MM-DD)", tr("Capture Date"))).clicked() {
                push(NameToken::CaptureDate("YYYY-MM-DD".into()));
                ui.close();
            }
            if ui.button(tr("Dimensions (4000x3000)")).clicked() {
                push(NameToken::Dimensions);
                ui.close();
            }
        });
        for (sym, t) in [("-", "-"), ("_", "_"), ("␣", " "), (".", ".")] {
            if ui.add(egui::Button::new(egui::RichText::new(sym).font(mono(12.0))).min_size(vec2(26.0, 22.0))).on_hover_text(tr("Add separator")).clicked() {
                m.s.template.push(NameToken::Text(t.into()));
            }
        }
        let te = ui.add(egui::TextEdit::singleline(&mut m.text_block).hint_text(tr("Text…")).font(regular(12.0)).desired_width(90.0));
        let enter = te.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if (enter || ui.add_enabled(!m.text_block.is_empty(), egui::Button::new(tr("Add"))).clicked()) && !m.text_block.is_empty() {
            m.s.template.push(NameToken::Text(std::mem::take(&mut m.text_block)));
        }
    });
    if m.s.uses(|t| *t == NameToken::Custom) {
        labeled_row(ui, "Custom Text", |ui| {
            ui.add(egui::TextEdit::singleline(&mut m.s.custom_text).font(regular(12.0)).desired_width(220.0));
        });
    }
    if m.s.uses(|t| matches!(t, NameToken::Sequence(_))) {
        let mut v = m.s.start_number as f64;
        number_row(ui, "Start Number", &mut v, 0.0..=999_999.0, 0, "");
        m.s.start_number = v as u32;
    }
    // example
    let first = app.export_jobs(ids).into_iter().next();
    let (stem, size, taken) = match &first {
        Some(j) => (j.stem(), j.full_size, j.taken),
        None => ("IMG_0001".into(), (4000, 3000), None),
    };
    let size = if size.0 == 0 { (4000, 3000) } else { size };
    let name = file_name(&m.s, &NameInput { stem: &stem, index: 0, size: m.s.out_size(size.0, size.1), taken, now: local_now() });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tr("Example")).font(label_font()).color(text_dim()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(egui::RichText::new(format!("{name}.{}", m.s.format.ext())).font(mono(11.5)).color(text())).truncate());
        });
    });
}

#[derive(Clone)]
enum BlockAction {
    Remove,
    Left,
    Right,
    Set(NameToken),
}

/// One block of the template: click for its options (format, move, remove).
fn block_chip(ui: &mut Ui, i: usize, tok: &NameToken, n: usize) -> Option<BlockAction> {
    let label = tok.label();
    let font = if tok.is_text() { mono(11.5) } else { medium(11.5) };
    let g = ui.painter().layout_no_wrap(label, font, text());
    let size = vec2(g.size().x + 20.0, 24.0);
    let (r, resp) = ui.allocate_exact_size(size, Sense::click());
    let (fill, stroke) = if tok.is_text() { (Color32::TRANSPARENT, separator()) } else { (with_alpha(ACCENT, 0.16), with_alpha(ACCENT, 0.45)) };
    ui.painter().rect_filled(r, CornerRadius::same(6), if resp.hovered() { with_alpha(ACCENT, 0.26) } else { fill });
    ui.painter().rect_stroke(r, CornerRadius::same(6), Stroke::new(1.0, stroke), StrokeKind::Inside);
    ui.painter().galley(pos2(r.left() + 10.0, r.center().y - g.size().y / 2.0), g, if tok.is_text() { text_dim() } else { text() });
    let mut out = None;
    egui::Popup::menu(&resp).id(Id::new(("block-menu", i))).show(|ui| {
        ui.set_min_width(190.0);
        match tok {
            NameToken::Sequence(d) => {
                for k in 1..=5u8 {
                    let ex = format!("{:0width$}", 1, width = k as usize);
                    if ui.add(egui::Button::selectable(*d == k, ex)).clicked() {
                        out = Some(BlockAction::Set(NameToken::Sequence(k)));
                        ui.close();
                    }
                }
                ui.separator();
            }
            NameToken::Date(f) | NameToken::CaptureDate(f) => {
                for (k, ex) in DATE_FORMATS {
                    if ui.add(egui::Button::selectable(f == k, format!("{k}   ({ex})"))).clicked() {
                        out = Some(BlockAction::Set(match tok {
                            NameToken::Date(_) => NameToken::Date(k.into()),
                            _ => NameToken::CaptureDate(k.into()),
                        }));
                        ui.close();
                    }
                }
                ui.separator();
            }
            NameToken::Text(t) => {
                let mut s = t.clone();
                if ui.add(egui::TextEdit::singleline(&mut s).font(regular(12.0)).desired_width(170.0)).changed() && !s.is_empty() {
                    out = Some(BlockAction::Set(NameToken::Text(s)));
                }
                ui.separator();
            }
            _ => {}
        }
        if ui.add_enabled(i > 0, egui::Button::new(format!("←  {}", tr("Move Left")))).clicked() {
            out = Some(BlockAction::Left);
            ui.close();
        }
        if ui.add_enabled(i + 1 < n, egui::Button::new(format!("→  {}", tr("Move Right")))).clicked() {
            out = Some(BlockAction::Right);
            ui.close();
        }
        if ui.button(egui::RichText::new(tr("Remove")).color(Color32::from_rgb(235, 80, 70))).clicked() {
            out = Some(BlockAction::Remove);
            ui.close();
        }
    });
    out
}

// MARK: - File settings / sizing

fn file_settings(ui: &mut Ui, m: &mut Mem) {
    let fo = vec![("jpg".to_string(), "JPG".to_string()), ("png".to_string(), "PNG".to_string())];
    if let Some(v) = segmented_row(ui, "Image Format", &fo, m.s.format.ext()) {
        m.s.format = if v == "jpg" { Format::Jpg } else { Format::Png };
    }
    let jpg = m.s.format == Format::Jpg;
    ui.add_enabled_ui(jpg, |ui| {
        if !jpg {
            ui.set_opacity(0.4);
        }
        if let Some(v) = value_slider(ui, Id::new("export-quality"), SliderSpec::new("Quality", m.s.quality * 100.0, 1.0, 100.0, 92.0).step(1.0).unit("%")) {
            m.s.quality = v / 100.0;
        }
        ui.horizontal(|ui| {
            let mut on = m.s.limit_size && jpg;
            if ui.checkbox(&mut on, egui::RichText::new(tr("Limit File Size To")).font(label_font()).color(text_dim())).changed() {
                m.s.limit_size = on;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("KB").font(regular(11.5)).color(text_dim()));
                ui.add_enabled(m.s.limit_size, egui::DragValue::new(&mut m.s.limit_kb).range(10..=100_000).speed(10.0));
            });
        });
    });
    if !jpg {
        ui.label(egui::RichText::new(tr("PNG is lossless; transparency is kept when the photo has it. Quality and file size apply to JPG.")).font(regular(11.0)).color(text_faint()));
    }
}

fn sizing(ui: &mut Ui, app: &AppState, m: &mut Mem) {
    if let Some(v) = toggle_row(ui, "Resize to Fit", m.s.resize) {
        m.s.resize = v;
    }
    ui.add_enabled_ui(m.s.resize, |ui| {
        if !m.s.resize {
            ui.set_opacity(0.4);
        }
        let opts = [
            (ResizeMode::WidthHeight, "Width & Height"),
            (ResizeMode::Dimensions, "Dimensions"),
            (ResizeMode::LongEdge, "Long Edge"),
            (ResizeMode::ShortEdge, "Short Edge"),
            (ResizeMode::Megapixels, "Megapixels"),
            (ResizeMode::Percentage, "Percentage"),
        ];
        if let Some(v) = picker_row(ui, "Fit", "export.resize", &opts, m.s.resize_mode) {
            m.s.resize_mode = v;
        }
        let unit = match m.s.units {
            Units::Pixels => "px",
            Units::Inches => "in",
            Units::Centimeters => "cm",
        };
        let dec = if m.s.units == Units::Pixels { 0 } else { 2 };
        match m.s.resize_mode {
            ResizeMode::WidthHeight | ResizeMode::Dimensions => {
                number_row(ui, if m.s.resize_mode == ResizeMode::WidthHeight { "Width" } else { "Long Side" }, &mut m.s.width, 0.0..=100_000.0, dec, unit);
                number_row(ui, if m.s.resize_mode == ResizeMode::WidthHeight { "Height" } else { "Short Side" }, &mut m.s.height, 0.0..=100_000.0, dec, unit);
                if m.s.resize_mode == ResizeMode::Dimensions {
                    ui.label(egui::RichText::new(tr("Fits whatever the orientation: the long side of the photo goes to the larger number.")).font(regular(11.0)).color(text_faint()));
                }
            }
            ResizeMode::LongEdge => number_row(ui, "Long Edge", &mut m.s.long_edge, 1.0..=100_000.0, dec, unit),
            ResizeMode::ShortEdge => number_row(ui, "Short Edge", &mut m.s.short_edge, 1.0..=100_000.0, dec, unit),
            ResizeMode::Megapixels => number_row(ui, "Megapixels", &mut m.s.megapixels, 0.1..=400.0, 1, "MP"),
            ResizeMode::Percentage => number_row(ui, "Percentage", &mut m.s.percent, 1.0..=800.0, 0, "%"),
        }
        if !matches!(m.s.resize_mode, ResizeMode::Megapixels | ResizeMode::Percentage) {
            let uo = [(Units::Pixels, "Pixels"), (Units::Inches, "Inches"), (Units::Centimeters, "Centimeters")];
            if let Some(v) = picker_row(ui, "Units", "export.units", &uo, m.s.units) {
                m.s.units = v;
            }
        }
        if let Some(v) = toggle_row(ui, "Don't Enlarge", m.s.dont_enlarge) {
            m.s.dont_enlarge = v;
        }
    });
    if m.s.units != Units::Pixels && m.s.resize {
        number_row(ui, "Resolution", &mut m.s.ppi, 1.0..=2400.0, 0, &tr("pixels per inch"));
    }
    if let Some(src) = app.source() {
        let (w, h) = full_size(&src, &app.state());
        let (ow, oh) = m.s.out_size(w, h);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tr("Output (this image)")).font(label_font()).color(text_dim()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new(format!("{ow} × {oh} px · {:.1} MP", (ow * oh) as f64 / 1e6)).font(value_font()).color(text()));
            });
        });
    }
}
