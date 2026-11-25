//! Effect browser (floating panel), presets panel (docked under the layers) and the command palette.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use uuid::Uuid;

use super::theme::*;
use super::widgets::*;
use crate::app::{AppState, Dialog, PromptKind};
use crate::effects::{self, Category, Effect};
use crate::l10n::tr;
use crate::model::*;
use crate::presets::{self, Preset};
use crate::settings;
use crate::shortcuts::{self, Action};

fn thumb(ui: &Ui, app: &AppState, rect: Rect, tex: Option<&egui::TextureHandle>, symbol: &str, highlighted: bool, radius: u8) {
    let _ = app;
    ui.painter().rect_filled(rect, CornerRadius::same(radius), field());
    match tex {
        Some(t) => {
            // aspect fill
            let ts = t.size_vec2();
            let s = (rect.width() / ts.x).max(rect.height() / ts.y);
            let (uw, uh) = ((rect.width() / (ts.x * s)).min(1.0), (rect.height() / (ts.y * s)).min(1.0));
            let uv = Rect::from_min_max(pos2((1.0 - uw) / 2.0, (1.0 - uh) / 2.0), pos2((1.0 + uw) / 2.0, (1.0 + uh) / 2.0));
            let mut mesh = egui::Mesh::with_texture(t.id());
            mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
            // clip corners via painter clip
            ui.painter().with_clip_rect(rect).add(egui::Shape::mesh(mesh));
        }
        None => paint_icon(ui, rect, symbol, 22.0, text_faint()),
    }
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(radius),
        Stroke::new(if highlighted { 1.5 } else { 1.0 }, if highlighted { ACCENT } else { separator() }),
        StrokeKind::Inside,
    );
}

fn ordered_effects(query: &str) -> Vec<&'static Effect> {
    let q = query.trim().to_lowercase();
    let mut all: Vec<&'static Effect> = vec![];
    for c in Category::ALL {
        all.extend(effects::all().iter().filter(|e| e.category == c && !e.hidden));
    }
    if q.is_empty() {
        return all;
    }
    all.into_iter()
        .filter(|e| {
            tr(e.name).to_lowercase().contains(&q)
                || e.name.to_lowercase().contains(&q)
                || tr(e.category.label()).to_lowercase().contains(&q)
                || tr(e.summary).to_lowercase().contains(&q)
        })
        .collect()
}

#[derive(Clone, Default)]
struct BrowserMem {
    query: String,
    highlight: Option<String>,
    key_nav_mouse: Option<egui::Pos2>,
    focus_done: bool,
}

/// The floating "Add Effect" panel. Returns false when it should close.
pub fn effect_browser(ui: &mut Ui, app: &mut AppState) {
    let id = Id::new("effect-browser");
    let mut mem: BrowserMem = ui.data(|d| d.get_temp(id).unwrap_or_default());
    let list = ordered_effects(&mem.query);
    let mut list_mode = settings::get().browser_list;
    let mut add: Option<&'static str> = None;
    let mouse = ui.ctx().pointer_hover_pos();
    egui::Frame::new().inner_margin(12).show(ui, |ui| {
        ui.horizontal(|ui| {
            let avail = ui.available_width() - 70.0;
            ui.allocate_ui(vec2(avail, 34.0), |ui| {
                let before = mem.query.clone();
                let r = search_field(ui, id.with("q"), &tr("Search effects"), &mut mem.query, !mem.focus_done);
                mem.focus_done = true;
                if mem.query != before {
                    mem.highlight = None;
                    app.hover_preview(None);
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    add = mem.highlight.as_deref().and_then(|h| list.iter().find(|e| e.id == h).map(|e| e.id)).or(list.first().map(|e| e.id));
                }
            });
            if let Some(v) = view_mode_toggle(ui, id.with("vm"), list_mode) {
                list_mode = v;
                settings::update(|s| s.browser_list = v);
            }
        });
    });
    // keyboard
    let (up, down, enter, esc) = ui.input(|i| {
        (
            i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowLeft),
            i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowRight),
            i.key_pressed(egui::Key::Enter),
            i.key_pressed(egui::Key::Escape),
        )
    });
    let mut scroll_to_highlight = false;
    if esc {
        app.show_effect_browser = false;
        app.set_preview_layers(None);
    } else if (up || down) && !list.is_empty() {
        let d: i64 = if up { -1 } else { 1 };
        let i = mem.highlight.as_ref().and_then(|h| list.iter().position(|e| e.id == h)).map(|i| i as i64).unwrap_or(if d > 0 { -1 } else { list.len() as i64 });
        let n = (i + d).clamp(0, list.len() as i64 - 1) as usize;
        mem.key_nav_mouse = mouse;
        mem.highlight = Some(list[n].id.to_string());
        app.hover_preview(Some(vec![LayerNode::effect(list[n].id, None, None)]));
        scroll_to_highlight = true;
    } else if enter && add.is_none() {
        add = mem.highlight.as_deref().and_then(|h| list.iter().find(|e| e.id == h).map(|e| e.id)).or(list.first().map(|e| e.id));
    }
    let footer_h = 28.0;
    let avail_h = ui.available_height() - footer_h;
    egui::ScrollArea::vertical().id_salt("browser-scroll").max_height(avail_h).auto_shrink([false, false]).show(ui, |ui| {
        egui::Frame::new().inner_margin(egui::Margin { left: 12, right: 12, top: 0, bottom: 14 }).show(ui, |ui| {
            for cat in Category::ALL {
                let items: Vec<&&Effect> = list.iter().filter(|e| e.category == cat).collect();
                if items.is_empty() {
                    continue;
                }
                ui.label(egui::RichText::new(spaced_upper(&tr(cat.label()))).font(section_font()).color(text_dim()));
                ui.add_space(2.0);
                if list_mode {
                    for e in items {
                        let hl = mem.highlight.as_deref() == Some(e.id);
                        let w = ui.available_width();
                        let (r, resp) = ui.allocate_exact_size(vec2(w, 56.0), Sense::click());
                        if hl {
                            ui.painter().rect_filled(r, CornerRadius::same(8), row_selected());
                        } else if resp.hovered() {
                            ui.painter().rect_filled(r, CornerRadius::same(8), row_hover());
                        }
                        let tr_ = Rect::from_min_size(r.min + vec2(6.0, 7.0), vec2(64.0, 42.0));
                        thumb(ui, app, tr_, app.effect_thumbs.get(e.id), e.symbol, hl || resp.hovered(), 6);
                        let tx = tr_.right() + 10.0;
                        let g = ui.painter().layout_job(rich_job(&format!("{}  {}", icon_text(e.symbol), tr(e.name)), medium(12.5), text(), Some(r.right() - tx - 6.0)));
                        ui.painter().galley(pos2(tx, r.top() + 16.0 - g.size().y / 2.0), g, text());
                        if !e.summary.is_empty() {
                            let g = ui.painter().layout(tr(e.summary), regular(11.0), text_dim(), r.right() - tx - 6.0);
                            ui.painter().with_clip_rect(r).galley(pos2(tx, r.top() + 26.0), g, text_dim());
                        }
                        if hl && scroll_to_highlight {
                            ui.scroll_to_rect(r, None);
                        }
                        hover_logic(ui, app, &mut mem, &resp, e.id, mouse);
                        if resp.clicked() {
                            add = Some(e.id);
                        }
                    }
                } else {
                    let w = ui.available_width();
                    let cols = ((w + 10.0) / (116.0 + 10.0)).floor().max(1.0) as usize;
                    let cw = (w - (cols - 1) as f32 * 10.0) / cols as f32;
                    for chunk in items.chunks(cols) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 10.0;
                            for e in chunk {
                                let hl = mem.highlight.as_deref() == Some(e.id);
                                let (r, resp) = ui.allocate_exact_size(vec2(cw, 92.0), Sense::click());
                                let tr_ = Rect::from_min_size(r.min, vec2(cw, 70.0));
                                thumb(ui, app, tr_, app.effect_thumbs.get(e.id), e.symbol, hl || resp.hovered(), 7);
                                let g = ui.painter().layout_job(rich_job(&format!("{}  {}", icon_text(e.symbol), tr(e.name)), medium(11.5), text(), Some(cw)));
                                ui.painter().galley(pos2(r.left(), r.top() + 76.0), g, text());
                                if hl && scroll_to_highlight {
                                    ui.scroll_to_rect(r, None);
                                }
                                hover_logic(ui, app, &mut mem, &resp, e.id, mouse);
                                let resp = resp.on_hover_text(tr(e.summary));
                                if resp.clicked() {
                                    add = Some(e.id);
                                }
                            }
                        });
                        ui.add_space(4.0);
                    }
                }
                ui.add_space(14.0);
            }
        });
    });
    browser_footer(ui, &keys_hint(&tr("Hover or ↑↓ to preview · click or ↩ to add · ⎋ close")));
    if let Some(eid) = add {
        app.set_preview_layers(None);
        app.add_effect(eid);
        app.show_effect_browser = false;
        mem = BrowserMem::default();
    }
    if !app.show_effect_browser {
        mem = BrowserMem::default();
    }
    ui.data_mut(|d| d.insert_temp(id, mem));
}

fn hover_logic(ui: &Ui, app: &mut AppState, mem: &mut BrowserMem, resp: &egui::Response, eid: &str, mouse: Option<egui::Pos2>) {
    let _ = ui;
    if resp.hovered() {
        if let Some(m) = mem.key_nav_mouse {
            if Some(m) == mouse {
                return;
            }
            mem.key_nav_mouse = None;
        }
        if mem.highlight.as_deref() != Some(eid) {
            mem.highlight = Some(eid.to_string());
            app.hover_preview(Some(vec![LayerNode::effect(eid, None, None)]));
        }
    }
}

// MARK: - Presets panel

#[derive(Clone, Default)]
struct PresetMem {
    query: String,
    highlight: Option<Uuid>,
    key_nav_mouse: Option<egui::Pos2>,
    search_focused: bool,
}

fn preset_sections(list: &[Preset]) -> Vec<(String, Vec<Preset>)> {
    let mut out = vec![];
    let mine: Vec<Preset> = list.iter().filter(|p| !p.is_factory).cloned().collect();
    if !mine.is_empty() {
        out.push(("My Presets".to_string(), mine));
    }
    for c in presets::CATEGORIES {
        let items: Vec<Preset> = list.iter().filter(|p| p.is_factory && p.category.as_deref() == Some(c)).cloned().collect();
        if !items.is_empty() {
            out.push((c.to_string(), items));
        }
    }
    out
}

pub fn presets_panel(ui: &mut Ui, app: &mut AppState) {
    let id = Id::new("presets-panel");
    let mut mem: PresetMem = ui.data(|d| d.get_temp(id).unwrap_or_default());
    let q = mem.query.to_lowercase();
    let list: Vec<Preset> = presets::all()
        .into_iter()
        .filter(|p| q.is_empty() || p.display_name().to_lowercase().contains(&q) || tr(p.category.as_deref().unwrap_or("")).to_lowercase().contains(&q))
        .collect();
    let secs = preset_sections(&list);
    let flat: Vec<Preset> = secs.iter().flat_map(|s| s.1.clone()).collect();
    let mut list_mode = settings::get().presets_list;
    let mut apply: Option<Preset> = None;
    let panel_rect = ui.available_rect_before_wrap();
    let mouse = ui.ctx().pointer_hover_pos();
    let hovering = mouse.map(|m| panel_rect.contains(m)).unwrap_or(false);
    egui::Frame::new().inner_margin(egui::Margin::symmetric(10, 8)).show(ui, |ui| {
        ui.horizontal(|ui| {
            let avail = ui.available_width() - 62.0 - 26.0 - 26.0 - 18.0;
            ui.allocate_ui(vec2(avail, 34.0), |ui| {
                let before = mem.query.clone();
                let r = search_field(ui, id.with("q"), &tr("Search presets"), &mut mem.query, false);
                mem.search_focused = r.has_focus();
                if mem.query != before {
                    mem.highlight = None;
                    app.hover_preview(None);
                }
                if r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    apply = mem.highlight.and_then(|h| flat.iter().find(|p| p.id == h).cloned()).or(flat.first().cloned());
                }
            });
            if let Some(v) = view_mode_toggle(ui, id.with("vm"), list_mode) {
                list_mode = v;
                settings::update(|s| s.presets_list = v);
            }
            let more = icon_button(ui, "ellipsis.circle", "More preset options", false, 15.0);
            egui::Popup::menu(&more).show(|ui| {
                ui.set_min_width(230.0);
                if ui.add_enabled(!app.layers().is_empty(), egui::Button::new(tr("Save Current Stack as Preset…"))).clicked() {
                    app.save_stack_as_preset();
                    ui.close();
                }
                ui.separator();
                if ui.button(tr("Import Presets…")).clicked() {
                    import_presets(app);
                    ui.close();
                }
                let user = presets::user().lock().unwrap().clone();
                if ui.add_enabled(!user.is_empty(), egui::Button::new(tr("Export My Presets…"))).clicked() {
                    export_presets(&user);
                    ui.close();
                }
                ui.separator();
                let mut vis = settings::get().factory_presets_visible;
                if ui.checkbox(&mut vis, tr("Show Built-in Presets")).changed() {
                    settings::update(|s| s.factory_presets_visible = vis);
                    app.regenerate_preset_thumbs();
                }
            });
            if icon_button(ui, "xmark", &format!("{} ({})", tr("Close presets"), shortcuts::display(Action::ShowPresets)), false, 12.0).clicked() {
                app.set_show_presets(false);
            }
        });
    });
    // keys only while the pointer is over the panel or the search is focused
    let active = !app.show_effect_browser && (hovering || mem.search_focused) && !app.show_command_palette;
    let mut scroll_to = false;
    if active {
        let (up, down, enter, esc) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::ArrowLeft),
                i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::ArrowRight),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if esc {
            if mem.highlight.is_some() || mem.search_focused {
                mem.highlight = None;
                ui.memory_mut(|m| m.surrender_focus(id.with("q")));
                app.hover_preview(None);
            } else {
                app.set_show_presets(false);
            }
        } else if (up || down) && !flat.is_empty() {
            let d: i64 = if up { -1 } else { 1 };
            let i = mem.highlight.and_then(|h| flat.iter().position(|p| p.id == h)).map(|i| i as i64).unwrap_or(if d > 0 { -1 } else { flat.len() as i64 });
            let n = (i + d).clamp(0, flat.len() as i64 - 1) as usize;
            mem.key_nav_mouse = mouse;
            mem.highlight = Some(flat[n].id);
            app.hover_preview(Some(app.preset_preview_layers(&flat[n])));
            scroll_to = true;
        } else if enter && apply.is_none() && !mem.search_focused {
            apply = mem.highlight.and_then(|h| flat.iter().find(|p| p.id == h).cloned()).or(flat.first().cloned());
        }
    }
    if !hovering && !mem.search_focused && mem.highlight.is_some() && !scroll_to {
        mem.highlight = None;
        app.hover_preview(None);
    }
    egui::ScrollArea::vertical().id_salt("presets-scroll").auto_shrink([false, false]).show(ui, |ui| {
        if flat.is_empty() {
            egui::Frame::new().inner_margin(20).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(tr("No presets yet. Build a stack of effects and save it from the ⋯ menu.")).font(regular(11.5)).color(text_faint()));
                });
            });
        }
        for (title, items) in &secs {
            egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 14, top: 7, bottom: 7 }).fill(panel_bg()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(spaced_upper(&tr(title))).font(section_font()).color(text_dim()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{}", items.len())).font(regular(10.0)).color(text_faint()));
                    });
                });
            });
            egui::Frame::new().inner_margin(egui::Margin { left: 12, right: 12, top: 0, bottom: 10 }).show(ui, |ui| {
                if list_mode {
                    for p in items {
                        let hl = mem.highlight == Some(p.id);
                        let w = ui.available_width();
                        let (r, resp) = ui.allocate_exact_size(vec2(w, 56.0), Sense::click());
                        if hl {
                            ui.painter().rect_filled(r, CornerRadius::same(8), row_selected());
                        } else if resp.hovered() {
                            ui.painter().rect_filled(r, CornerRadius::same(8), row_hover());
                        }
                        let tr_ = Rect::from_min_size(r.min + vec2(6.0, 7.0), vec2(64.0, 42.0));
                        preset_thumb(ui, app, tr_, p, hl || resp.hovered());
                        let tx = tr_.right() + 10.0;
                        ui.painter().text(pos2(tx, r.top() + 16.0), Align2::LEFT_CENTER, p.display_name(), medium(12.5), text());
                        let fx: Vec<String> = p.layers.all_effect_ids().iter().filter_map(|i| effects::effect(i).map(|e| tr(e.name))).collect();
                        let g = ui.painter().layout(fx.join(" · "), regular(11.0), text_dim(), r.right() - tx - 6.0);
                        ui.painter().with_clip_rect(r).galley(pos2(tx, r.top() + 26.0), g, text_dim());
                        if hl && scroll_to {
                            ui.scroll_to_rect(r, None);
                        }
                        let resp = preset_hover(ui, app, &mut mem, resp, p, mouse);
                        if resp.clicked() {
                            apply = Some(p.clone());
                        }
                        preset_menu(app, &resp, p);
                    }
                } else {
                    let w = ui.available_width();
                    let cols = ((w + 8.0) / (105.0 + 8.0)).floor().max(1.0) as usize;
                    let cw = (w - (cols - 1) as f32 * 8.0) / cols as f32;
                    for chunk in items.chunks(cols) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            for p in chunk {
                                let hl = mem.highlight == Some(p.id);
                                let (r, resp) = ui.allocate_exact_size(vec2(cw, 98.0), Sense::click());
                                let tr_ = Rect::from_min_size(r.min, vec2(cw, 78.0));
                                preset_thumb(ui, app, tr_, p, hl || resp.hovered());
                                let mut job = egui::text::LayoutJob::simple_singleline(p.display_name(), medium(11.5), text());
                                job.wrap = egui::text::TextWrapping::truncate_at_width(cw);
                                let g = ui.painter().layout_job(job);
                                ui.painter().galley(pos2(r.left(), r.top() + 83.0), g, text());
                                if hl && scroll_to {
                                    ui.scroll_to_rect(r, None);
                                }
                                let resp = preset_hover(ui, app, &mut mem, resp, p, mouse).on_hover_text(tr("Click to add on top as a group"));
                                if resp.clicked() {
                                    apply = Some(p.clone());
                                }
                                preset_menu(app, &resp, p);
                            }
                        });
                        ui.add_space(6.0);
                    }
                }
            });
        }
    });
    if let Some(p) = apply {
        app.set_preview_layers(None);
        app.apply_preset(&p);
        mem.highlight = None;
    }
    ui.data_mut(|d| d.insert_temp(id, mem));
}

fn preset_thumb(ui: &Ui, app: &AppState, rect: Rect, p: &Preset, highlighted: bool) {
    thumb(ui, app, rect, app.preset_thumbs.get(&p.id), "wand.and.stars", highlighted, 7);
    if p.is_factory {
        paint_icon(ui, Rect::from_center_size(pos2(rect.right() - 9.0, rect.top() + 9.0), vec2(10.0, 10.0)), "seal.fill", 10.0, ACCENT);
    }
}

fn preset_hover(ui: &Ui, app: &mut AppState, mem: &mut PresetMem, resp: egui::Response, p: &Preset, mouse: Option<egui::Pos2>) -> egui::Response {
    let _ = ui;
    if resp.hovered() {
        if let Some(m) = mem.key_nav_mouse {
            if Some(m) == mouse {
                return resp;
            }
            mem.key_nav_mouse = None;
        }
        if mem.highlight != Some(p.id) {
            mem.highlight = Some(p.id);
            app.hover_preview(Some(app.preset_preview_layers(p)));
        }
    }
    resp
}

fn preset_menu(app: &mut AppState, resp: &egui::Response, p: &Preset) {
    resp.context_menu(|ui| {
        if ui.button(tr("Apply")).clicked() {
            app.apply_preset(p);
            ui.close();
        }
        if ui.button(tr("Export…")).clicked() {
            export_presets(std::slice::from_ref(p));
            ui.close();
        }
        if !p.is_factory {
            if ui.button(tr("Rename…")).clicked() {
                app.dialog = Some(Dialog::Prompt { title: tr("Rename Preset"), label: tr("Name"), text: p.name.clone(), kind: PromptKind::RenamePreset(p.id) });
                ui.close();
            }
            ui.separator();
            if ui.button(egui::RichText::new(tr("Delete")).color(Color32::from_rgb(235, 80, 70))).clicked() {
                presets::delete(p.id);
                app.regenerate_preset_thumbs();
                ui.close();
            }
        }
    });
}

pub fn import_presets(app: &mut AppState) {
    let Some(files) = rfd::FileDialog::new().add_filter("EFFECTOR Preset", &["effectorpreset", "json"]).pick_files() else { return };
    let mut n = 0;
    for f in files {
        match std::fs::read(&f).map_err(|e| e.to_string()).and_then(|d| presets::import_bytes(&d)) {
            Ok(k) => n += k,
            Err(e) => app.alert(&tr("Could not import presets"), &e),
        }
    }
    if n > 0 {
        app.flash(&crate::l10n::trf("%d presets imported", &[&n]));
        app.regenerate_preset_thumbs();
    }
}

pub fn export_presets(list: &[Preset]) {
    let name = if list.len() == 1 { list[0].name.clone() } else { "EFFECTOR Presets".into() };
    if let Some(path) = rfd::FileDialog::new().set_file_name(format!("{name}.effectorpreset")).add_filter("EFFECTOR Preset", &["effectorpreset"]).save_file() {
        let _ = std::fs::write(path, presets::export_bytes(list));
    }
}

// MARK: - Command palette

enum Cmd {
    Effect(&'static str),
    Preset(Preset),
    Action(Action),
}

struct PaletteItem {
    title: String,
    subtitle: String,
    symbol: &'static str,
    run: Cmd,
}

pub fn command_palette(ctx: &egui::Context, app: &mut AppState) {
    let id = Id::new("cmd-palette");
    let (mut query, mut index): (String, usize) = ctx.data(|d| d.get_temp(id).unwrap_or_default());
    let mut items: Vec<PaletteItem> = vec![];
    for e in effects::all().iter().filter(|e| !e.hidden) {
        items.push(PaletteItem { title: tr(e.name), subtitle: format!("{} · {}", tr("Add effect"), tr(e.category.label())), symbol: e.symbol, run: Cmd::Effect(e.id) });
    }
    for p in presets::all() {
        items.push(PaletteItem { title: p.display_name(), subtitle: tr("Apply preset"), symbol: "wand.and.stars", run: Cmd::Preset(p) });
    }
    for a in Action::ALL {
        if a != Action::CommandPalette {
            items.push(PaletteItem { title: tr(a.label()), subtitle: shortcuts::display(a), symbol: "command", run: Cmd::Action(a) });
        }
    }
    let q = query.trim().to_lowercase();
    if !q.is_empty() {
        items.retain(|i| i.title.to_lowercase().contains(&q) || i.subtitle.to_lowercase().contains(&q));
        items.sort_by_key(|i| if i.title.to_lowercase().starts_with(&q) { 0 } else { 1 });
    }
    items.truncate(80);
    // backdrop
    let screen = ctx.content_rect();
    egui::Area::new(id.with("bg")).order(egui::Order::Foreground).fixed_pos(screen.min).show(ctx, |ui| {
        let (r, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
        ui.painter().rect_filled(r, CornerRadius::ZERO, Color32::from_black_alpha(64));
        if resp.clicked() {
            app.show_command_palette = false;
        }
    });
    let mut run: Option<usize> = None;
    egui::Area::new(id.with("win")).order(egui::Order::Foreground).fixed_pos(pos2(screen.center().x - 260.0, screen.top() + 80.0)).show(ctx, |ui| {
        egui::Frame::new()
            .fill(panel_bg())
            .stroke(Stroke::new(1.0, separator()))
            .corner_radius(CornerRadius::same(14))
            .shadow(egui::Shadow { offset: [0, 12], blur: 30, spread: 0, color: Color32::from_black_alpha(100) })
            .show(ui, |ui| {
                ui.set_width(520.0);
                egui::Frame::new().inner_margin(14).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(icon_text("sparkle.magnifyingglass")).font(icon_font(15.0)).color(accent_text()));
                        let before = query.clone();
                        let te = ui.add(
                            egui::TextEdit::singleline(&mut query)
                                .id(id.with("q"))
                                .frame(egui::Frame::NONE)
                                .hint_text(tr("Search effects, presets and commands"))
                                .font(regular(15.0))
                                .desired_width(f32::INFINITY),
                        );
                        te.request_focus();
                        if query != before {
                            index = 0;
                        }
                    });
                });
                separator_line(ui);
                let (down, up, enter, esc) = ui.input(|i| {
                    (i.key_pressed(egui::Key::ArrowDown), i.key_pressed(egui::Key::ArrowUp), i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape))
                });
                if down {
                    index = (index + 1).min(items.len().saturating_sub(1));
                }
                if up {
                    index = index.saturating_sub(1);
                }
                if esc {
                    app.show_command_palette = false;
                }
                if enter && index < items.len() {
                    run = Some(index);
                }
                egui::ScrollArea::vertical().max_height(320.0).min_scrolled_height(320.0).auto_shrink([false, false]).show(ui, |ui| {
                    egui::Frame::new().inner_margin(6).show(ui, |ui| {
                        ui.spacing_mut().item_spacing.y = 1.0;
                        for (i, it) in items.iter().enumerate() {
                            let w = ui.available_width();
                            let (r, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
                            if i == index {
                                ui.painter().rect_filled(r, CornerRadius::same(7), row_selected());
                                if down || up {
                                    ui.scroll_to_rect(r, None);
                                }
                            }
                            paint_icon(ui, Rect::from_center_size(pos2(r.left() + 22.0, r.center().y), vec2(20.0, 20.0)), it.symbol, 14.0, if i == index { accent_text() } else { text_dim() });
                            ui.painter().text(pos2(r.left() + 40.0, r.center().y), Align2::LEFT_CENTER, &it.title, regular(13.0), text());
                            ui.painter().text(pos2(r.right() - 12.0, r.center().y), Align2::RIGHT_CENTER, &it.subtitle, regular(11.0), text_faint());
                            if resp.clicked() {
                                run = Some(i);
                            }
                        }
                    });
                });
            });
    });
    if let Some(i) = run {
        app.show_command_palette = false;
        let ctx2 = ctx.clone();
        match &items[i].run {
            Cmd::Effect(e) => app.add_effect(e),
            Cmd::Preset(p) => app.apply_preset(p),
            Cmd::Action(a) => app.perform(*a, &ctx2),
        }
    }
    if app.show_command_palette {
        ctx.data_mut(|d| d.insert_temp(id, (query, index)));
    } else {
        ctx.data_mut(|d| d.remove::<(String, usize)>(id));
    }
}
