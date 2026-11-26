//! Modal sheets and windows: export, settings (general / shortcuts / export), canvas popover,
//! text prompts, alerts and the close-images confirmation.

use egui::{vec2, Color32, CornerRadius, Id, Stroke, Ui};

use super::theme::*;
use super::widgets::*;
use crate::app::{AppState, Dialog, PromptKind};
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
    let mut tab: usize = ctx.data(|d| d.get_temp(id.with("tab")).unwrap_or(0)).min(1);
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
            let tabs = vec![("0".to_string(), tr("General")), ("1".to_string(), tr("Shortcuts"))];
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
                    ui.add_space(10.0);
                    update_section(ui, app);
                }
                _ => shortcut_settings(ui),
            }
        });
    ctx.data_mut(|d| d.insert_temp(id.with("tab"), tab));
    if !open {
        app.show_settings = false;
    }
}

/// "Updates" block of the General tab: check GitHub, download and install new versions.
fn update_section(ui: &mut Ui, app: &mut AppState) {
    use crate::updater::State;
    ui.label(egui::RichText::new(spaced_upper(&tr("Updates"))).font(section_font()).color(text_dim()));
    ui.add_space(4.0);
    let st = crate::updater::current();
    let repaint = ui.ctx().clone();
    let notify = move || repaint.request_repaint();
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(trf("You have version %@", &[&crate::ui::VERSION])).font(regular(12.0)).color(text()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let busy = matches!(st, State::Checking | State::Downloading { .. } | State::Installing);
            if ui.add_enabled(!busy, egui::Button::new(format!("{}  {}", icon_text("arrow.triangle.2.circlepath"), tr("Check for Updates")))).clicked() {
                crate::updater::check(notify.clone());
            }
        });
    });
    ui.add_space(4.0);
    let small = |ui: &mut Ui, t: &str, c: egui::Color32| {
        ui.label(egui::RichText::new(t).font(regular(11.5)).color(c));
    };
    match &st {
        State::Idle => {}
        State::Checking => {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(11.0));
                small(ui, &tr("Checking GitHub…"), text_dim());
            });
        }
        State::UpToDate => small(ui, &tr("EFFECTOR is up to date."), text_dim()),
        State::Failed(e) => small(ui, &format!("{} {e}", tr("Update failed:")), egui::Color32::from_rgb(235, 90, 80)),
        State::Available { version, notes, size, page, .. } => {
            small(ui, &trf("Version %@ is available (%@ MB).", &[version, &format!("{:.0}", *size as f64 / 1e6)]), accent_text());
            if !notes.is_empty() {
                egui::CollapsingHeader::new(tr("What's new")).id_salt("upd-notes").show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                        ui.label(egui::RichText::new(notes.as_str()).font(regular(11.0)).color(text_dim()));
                    });
                });
            }
            ui.horizontal(|ui| {
                if pill_button(ui, &tr("Download and Install"), true).clicked() {
                    crate::updater::download(notify.clone());
                }
                if !page.is_empty() && pill_button(ui, &tr("Open Release Page"), false).clicked() {
                    let _ = open::that(page);
                }
            });
        }
        State::Downloading { done, total, .. } => {
            let f = if *total > 0 { *done as f32 / *total as f32 } else { 0.0 };
            ui.add(egui::ProgressBar::new(f).desired_width(ui.available_width()).text(format!("{:.0} / {:.0} MB", *done as f64 / 1e6, *total as f64 / 1e6)));
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
        }
        State::Ready { version, .. } => {
            small(ui, &trf("Version %@ is downloaded. EFFECTOR will close, update and reopen.", &[version]), accent_text());
            if app.is_dirty() && !settings::get().reopen_last_session {
                small(ui, &tr("Save your project first: unsaved changes will be lost."), egui::Color32::from_rgb(235, 160, 60));
            }
            if pill_button(ui, &tr("Install and Restart"), true).clicked() {
                match crate::updater::install() {
                    Ok(true) => app.quit_for_update = true,
                    Ok(false) => app.flash(&tr("Opened the downloaded update")),
                    Err(e) => app.alert(&tr("Update failed:"), &e),
                }
            }
        }
        State::Installing => small(ui, &tr("Installing…"), text_dim()),
    }
    if let Some(v) = toggle_row(ui, "Check for updates automatically", settings::get().check_updates) {
        settings::update(|s| s.check_updates = v);
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

// MARK: - Update pop-up

/// Release notes (Markdown) as simple rich text: headings, bullets and bold, without the
/// installation instructions that follow the `---` line.
fn release_notes(ui: &mut Ui, notes: &str) {
    let body = notes.split("\n---").next().unwrap_or(notes);
    for line in body.lines() {
        let l = line.trim_end();
        if l.trim().is_empty() {
            ui.add_space(3.0);
            continue;
        }
        let clean = |s: &str| s.replace("**", "").replace('`', "");
        if let Some(h) = l.strip_prefix("### ").or_else(|| l.strip_prefix("## ")).or_else(|| l.strip_prefix("# ")) {
            if h.to_lowercase().starts_with("patch notes") {
                continue;
            }
            ui.add_space(2.0);
            ui.label(egui::RichText::new(spaced_upper(&clean(h))).font(section_font()).color(text_dim()));
        } else if let Some(b) = l.trim_start().strip_prefix("- ") {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(egui::RichText::new("•").font(regular(11.5)).color(accent_text()));
                ui.add(egui::Label::new(egui::RichText::new(clean(b)).font(regular(11.5)).color(text())).wrap());
            });
        } else {
            ui.add(egui::Label::new(egui::RichText::new(clean(l)).font(regular(11.5)).color(text())).wrap());
        }
    }
}

/// Shown when the launch check finds a new version: what's new, then download → install, all here.
pub fn update_popup(ctx: &egui::Context, app: &mut AppState) {
    use crate::updater::State;
    let st = crate::updater::current();
    let repaint = ctx.clone();
    let notify = move || repaint.request_repaint();
    let mut close = false;
    egui::Modal::new(Id::new("update-popup")).frame(modal_frame()).backdrop_color(Color32::from_black_alpha(90)).show(ctx, |ui| {
        ui.set_width(420.0);
        ui.spacing_mut().item_spacing.y = 8.0;
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(icon_text("arrow.down.circle")).font(icon_font(20.0)).color(accent_text()));
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 1.0;
                ui.label(egui::RichText::new(tr("A new version of EFFECTOR is available")).font(semibold(15.0)).color(text()));
                let v = match &st {
                    State::Available { version, .. } | State::Downloading { version, .. } | State::Ready { version, .. } => version.clone(),
                    _ => String::new(),
                };
                if !v.is_empty() {
                    ui.label(egui::RichText::new(trf("Version %@ · you have %@", &[&v, &crate::ui::VERSION])).font(regular(11.5)).color(text_dim()));
                }
            });
        });
        ui.add_space(2.0);
        if let State::Available { notes, .. } = &st {
            if !notes.trim().is_empty() {
                egui::Frame::new().fill(field()).corner_radius(CornerRadius::same(8)).inner_margin(12).show(ui, |ui| {
                    egui::ScrollArea::vertical().max_height(240.0).auto_shrink([false, true]).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        release_notes(ui, notes);
                    });
                });
            }
        }
        match &st {
            State::Downloading { done, total, .. } => {
                let f = if *total > 0 { *done as f32 / *total as f32 } else { 0.0 };
                ui.add(egui::ProgressBar::new(f).desired_width(ui.available_width()).text(format!("{:.0} / {:.0} MB", *done as f64 / 1e6, *total as f64 / 1e6)));
                ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
            }
            State::Ready { .. } => {
                ui.label(egui::RichText::new(tr("Downloaded. EFFECTOR will close, update and reopen.")).font(regular(12.0)).color(text_dim()));
                if app.is_dirty() && !settings::get().reopen_last_session {
                    ui.label(egui::RichText::new(tr("Save your project first: unsaved changes will be lost.")).font(regular(11.5)).color(Color32::from_rgb(235, 160, 60)));
                }
            }
            State::Installing => {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(12.0));
                    ui.label(egui::RichText::new(tr("Installing…")).font(regular(12.0)).color(text_dim()));
                });
            }
            State::Failed(e) => {
                ui.label(egui::RichText::new(format!("{} {e}", tr("Update failed:"))).font(regular(12.0)).color(Color32::from_rgb(235, 90, 80)));
            }
            _ => {}
        }
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if let State::Available { version, .. } = &st {
                let skip = ui.add(egui::Label::new(egui::RichText::new(tr("Skip This Version")).font(regular(11.5)).color(text_faint())).sense(egui::Sense::click()));
                if skip.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if skip.on_hover_text(tr("Don't show this again until a newer version comes out")).clicked() {
                    let v = version.clone();
                    settings::update(|s| s.skipped_version = v);
                    close = true;
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match &st {
                    State::Available { .. } => {
                        if pill_button(ui, &tr("Download and Install"), true).clicked() {
                            crate::updater::download(notify.clone());
                        }
                        if pill_button(ui, &tr("Later"), false).clicked() {
                            close = true;
                        }
                    }
                    State::Downloading { .. } => {
                        if pill_button(ui, &tr("Hide"), false).on_hover_text(tr("The download continues; install it later from Settings › Updates")).clicked() {
                            close = true;
                        }
                    }
                    State::Ready { .. } => {
                        if pill_button(ui, &tr("Install and Restart"), true).clicked() {
                            match crate::updater::install() {
                                Ok(true) => app.quit_for_update = true,
                                Ok(false) => {
                                    app.flash(&tr("Opened the downloaded update"));
                                    close = true;
                                }
                                Err(e) => app.alert(&tr("Update failed:"), &e),
                            }
                        }
                        if pill_button(ui, &tr("Later"), false).clicked() {
                            close = true;
                        }
                    }
                    State::Installing => {}
                    _ => {
                        if pill_button(ui, &tr("Close"), false).clicked() {
                            close = true;
                        }
                    }
                }
            });
        });
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) && !matches!(st, State::Installing) {
            close = true;
        }
    });
    if close {
        app.show_update_popup = false;
    }
}
