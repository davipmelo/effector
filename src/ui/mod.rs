//! User interface (egui): main window layout, toolbar, menus, info bar, keyboard shortcuts,
//! file drops and the bridge between rendered GPU textures and the screen.

pub mod browsers;
pub mod canvas_view;
pub mod dialogs;
pub mod filmstrip;
pub mod icons;
pub mod inspector;
pub mod layers;
pub mod theme;
pub mod widgets;

use std::path::PathBuf;
use std::sync::Arc;

use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Ui};

use crate::app::AppState;
use crate::gpu::T;
use crate::l10n::{tr, trf};
use crate::model::LayerTreeRead;
use crate::settings::{self, Theme};
use crate::shortcuts::{self, Action, KeyCombo};
use theme::*;
use widgets::*;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

struct DisplayTex {
    id: Option<egui::TextureId>,
    tex: Option<T>,
    nearest: bool,
}

impl DisplayTex {
    fn new() -> DisplayTex { DisplayTex { id: None, tex: None, nearest: false } }

    /// Registers / updates the egui texture for `t` (identity- and filter-aware).
    fn sync(&mut self, rs: &egui_wgpu::RenderState, t: Option<&T>, nearest: bool) -> Option<egui::TextureId> {
        let t = t?;
        let same = self.tex.as_ref().map(|x| Arc::ptr_eq(x, t)).unwrap_or(false);
        if same && self.nearest == nearest {
            return self.id;
        }
        let filter = if nearest { eframe::wgpu::FilterMode::Nearest } else { eframe::wgpu::FilterMode::Linear };
        let mut r = rs.renderer.write();
        match self.id {
            Some(id) => r.update_egui_texture_from_wgpu_texture(&rs.device, &t.view, filter, id),
            None => self.id = Some(r.register_native_texture(&rs.device, &t.view, filter)),
        }
        self.tex = Some(t.clone());
        self.nearest = nearest;
        self.id
    }
}

pub struct EffectorUi {
    app: AppState,
    out_tex: DisplayTex,
    orig_tex: DisplayTex,
    crop_tex: DisplayTex,
    checker: egui::TextureHandle,
    last_theme: Option<bool>,
    last_title: String,
    first_frame: bool,
    pending_open: Vec<PathBuf>,
    confirm_quit: bool,
    allow_close: bool,
    canvas_btn_rect: Rect,
    presets_rect: Rect,
    /// Dev/test: save a screenshot of the window here, then quit (`--ui-screenshot <png> [script]`).
    shot: Option<(PathBuf, String)>,
    shot_start: Option<std::time::Instant>,
    shot_step: usize,
}

impl EffectorUi {
    pub fn new(cc: &eframe::CreationContext<'_>, open: Vec<PathBuf>) -> EffectorUi {
        let rs = cc.wgpu_render_state.as_ref().expect("wgpu");
        crate::gpu::init(rs.device.clone(), rs.queue.clone());
        // compile every compute kernel in the background (first use is then instant)
        std::thread::spawn(|| crate::gpu::gpu().warm_up());
        install_fonts(&cc.egui_ctx);
        let checker = cc.egui_ctx.load_texture(
            "checker",
            egui::ColorImage::from_rgba_unmultiplied(
                [2, 2],
                &[71, 71, 71, 255, 92, 92, 92, 255, 92, 92, 92, 255, 71, 71, 71, 255],
            ),
            egui::TextureOptions { magnification: egui::TextureFilter::Nearest, minification: egui::TextureFilter::Nearest, wrap_mode: egui::TextureWrapMode::Repeat, mipmap_mode: None },
        );
        let app = AppState::new(cc.egui_ctx.clone());
        EffectorUi {
            app,
            out_tex: DisplayTex::new(),
            orig_tex: DisplayTex::new(),
            crop_tex: DisplayTex::new(),
            checker,
            last_theme: None,
            last_title: String::new(),
            first_frame: true,
            pending_open: open,
            confirm_quit: false,
            allow_close: false,
            canvas_btn_rect: Rect::NOTHING,
            presets_rect: Rect::NOTHING,
            shot: std::env::var("EFFECTOR_SHOT").ok().map(|p| (PathBuf::from(p), std::env::var("EFFECTOR_SHOT_SCRIPT").unwrap_or_default())),
            shot_start: None,
            shot_step: 0,
        }
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let dark = match settings::get().theme {
            Theme::Dark => true,
            Theme::Light => false,
            Theme::System => ctx.system_theme().map(|t| t == egui::Theme::Dark).unwrap_or(true),
        };
        if self.last_theme != Some(dark) {
            set_dark(dark);
            apply_style(ctx);
            self.last_theme = Some(dark);
        }
    }

    // MARK: - Keyboard

    fn handle_keys(&mut self, ctx: &egui::Context) {
        if dialogs::is_recording_shortcut(ctx) || self.app.dialog.is_some() || self.app.show_export {
            return;
        }
        let events: Vec<(egui::Key, egui::Modifiers)> = ctx.input(|i| {
            i.events.iter().filter_map(|e| if let egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } = e { Some((*key, *modifiers)) } else { None }).collect()
        });
        let editing = ctx.egui_wants_keyboard_input();
        let focused = ctx.memory(|m| m.focused().is_some());
        let picker_open = widgets_picker_open(ctx);
        let presets_hover = ctx.pointer_hover_pos().map(|p| self.presets_rect.contains(p)).unwrap_or(false);
        for (key, m) in events {
            let Some(combo) = KeyCombo::from_event(key, m) else { continue };
            if combo.key == "escape" && !combo.has_modifier() && self.app.presenting && !editing {
                self.app.toggle_presentation(ctx);
                continue;
            }
            let Some(action) = shortcuts::action_for(&combo) else { continue };
            if editing && (!combo.has_modifier() || action.is_text_editing_standard()) {
                continue;
            }
            if !combo.has_modifier() {
                if picker_open || focused || self.app.show_command_palette || self.app.is_cropping {
                    continue;
                }
                // while a browser panel is open, plain keys belong to it
                if self.app.show_effect_browser && action != Action::ShowPresets {
                    continue;
                }
                if presets_hover && self.app.show_preset_browser && matches!(combo.key.as_str(), "left" | "right" | "up" | "down") {
                    continue;
                }
            }
            if action == Action::CommandPalette || action == Action::Export || !self.app.show_command_palette {
                self.app.perform(action, ctx);
            }
        }
        // select all layers (⌘⌥A / Ctrl+Alt+A)
        if !editing && ctx.input(|i| i.modifiers.command && i.modifiers.alt && i.key_pressed(egui::Key::A)) {
            self.app.select_all_layers();
        }
        // ⌘0 always fits; ⌘, opens settings
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Num0)) {
            self.app.set_zoom(None);
        }
        if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Comma)) {
            self.app.show_settings = !self.app.show_settings;
        }
    }

    // MARK: - Toolbar

    fn toolbar(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let rect = ui.available_rect_before_wrap();
        let has = self.app.has_image();
        // centre title
        let c = pos2(rect.center().x, rect.center().y - 4.0);
        let g = ui.painter().layout_no_wrap("EFFECTOR".into(), semibold(12.5), text());
        let pill = Rect::from_center_size(c, vec2(g.size().x + 32.0 + 6.0, 20.0));
        ui.painter().rect_filled(pill, CornerRadius::same(255), field());
        ui.painter().galley(pill.center() - g.size() / 2.0, g, text());
        ui.painter().text(pos2(c.x, pill.bottom() + 7.0), Align2::CENTER_CENTER, format!("v{VERSION}"), mono(8.5), with_alpha(text_faint(), 0.8));
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            ui.add_space(8.0);
            self.main_menu(ui);
            if tool_button(ui, "folder", &format!("{}  {}", tr("Open…"), shortcuts::display(Action::Open)), true, false).clicked() {
                self.app.perform(Action::Open, &ctx);
            }
            if tool_button(ui, "square.and.arrow.down", &format!("{}  {}", tr("Save Project"), shortcuts::display(Action::Save)), has, false).clicked() {
                self.app.save(false);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);
                if tool_button(ui, "gearshape", &format!("{}  {}", tr("Settings"), if shortcuts::IS_MAC { "⌘," } else { "Ctrl+," }), true, self.app.show_settings).clicked() {
                    self.app.show_settings = !self.app.show_settings;
                }
                if tool_button(ui, "square.and.arrow.up", &format!("{}  {}", tr("Export Image…"), shortcuts::display(Action::Export)), has, false).clicked() {
                    self.app.show_export = true;
                }
                if tool_button(ui, "square.split.2x1", &format!("{}  {}", tr("Toggle Before / After"), shortcuts::display(Action::BeforeAfter)), has, self.app.show_original).clicked() {
                    self.app.perform(Action::BeforeAfter, &ctx);
                }
                if tool_button(ui, "rectangle.split.2x1", &format!("{}  {}", tr("Split Before / After"), shortcuts::display(Action::Compare)), has, self.app.compare_split).clicked() {
                    self.app.perform(Action::Compare, &ctx);
                }
                if tool_button(ui, "crop", &format!("{}  {}", tr("Crop"), shortcuts::display(Action::Crop)), has, self.app.is_cropping).clicked() {
                    self.app.perform(Action::Crop, &ctx);
                }
                let cb = tool_button(ui, "aspectratio", &tr("Canvas & Borders: fit the image in another aspect ratio (Instagram etc.)"), has, self.app.show_canvas_popover);
                self.canvas_btn_rect = cb.rect;
                if cb.clicked() {
                    self.app.show_canvas_popover = !self.app.show_canvas_popover;
                }
                if tool_button(ui, "arrow.uturn.forward", &format!("{}  {}", tr("Redo"), shortcuts::display(Action::Redo)), self.app.can_redo(), false).clicked() {
                    self.app.redo();
                }
                if tool_button(ui, "arrow.uturn.backward", &format!("{}  {}", tr("Undo"), shortcuts::display(Action::Undo)), self.app.can_undo(), false).clicked() {
                    self.app.undo();
                }
            });
        });
    }

    fn main_menu(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        let resp = tool_button(ui, "line.3.horizontal", &tr("Menu"), true, false);
        let has = self.app.has_image();
        let app = &mut self.app;
        egui::Popup::menu(&resp).show(|ui| {
            ui.set_min_width(240.0);
            let mut item = |ui: &mut Ui, app: &mut AppState, a: Action, enabled: bool| {
                let b = egui::Button::new(tr(a.label())).shortcut_text(shortcuts::display(a));
                if ui.add_enabled(enabled, b).clicked() {
                    app.perform(a, &ctx);
                    ui.close();
                }
            };
            ui.menu_button(tr("File"), |ui| {
                item(ui, app, Action::Open, true);
                ui.separator();
                item(ui, app, Action::Save, has);
                item(ui, app, Action::SaveAs, has);
                ui.separator();
                item(ui, app, Action::CloseImage, has);
                ui.separator();
                item(ui, app, Action::Export, has);
                item(ui, app, Action::CopyResult, has);
            });
            ui.menu_button(tr("Edit"), |ui| {
                let (cu, cr) = (app.can_undo(), app.can_redo());
                item(ui, app, Action::Undo, cu);
                item(ui, app, Action::Redo, cr);
                ui.separator();
                item(ui, app, Action::Paste, true);
                ui.separator();
                let sel = !app.selected_ids.is_empty();
                item(ui, app, Action::CopyLayers, sel);
                item(ui, app, Action::PasteLayers, has);
                let b = egui::Button::new(tr("Select All Layers")).shortcut_text(if shortcuts::IS_MAC { "⌥⌘A" } else { "Ctrl+Alt+A" });
                if ui.add_enabled(has, b).clicked() {
                    app.select_all_layers();
                    ui.close();
                }
            });
            ui.menu_button(tr("Image"), |ui| {
                item(ui, app, Action::Crop, has);
                item(ui, app, Action::BeforeAfter, has);
                item(ui, app, Action::Compare, has);
                ui.separator();
                let many = app.strip_selection.len() >= 2;
                item(ui, app, Action::SyncSettings, many);
                if ui.add_enabled(has, egui::Button::new(tr("Copy Settings"))).clicked() {
                    app.copy_settings();
                    ui.close();
                }
                if ui.add_enabled(app.has_copied_settings(), egui::Button::new(tr("Paste Settings to Selected Images"))).clicked() {
                    let s = app.strip_selection.clone();
                    app.paste_settings(&s);
                    ui.close();
                }
            });
            ui.menu_button(tr("Layer"), |ui| {
                item(ui, app, Action::CommandPalette, has);
                item(ui, app, Action::AddEffect, has);
                ui.separator();
                let sel = app.selected_layer();
                item(ui, app, Action::Group, has);
                item(ui, app, Action::Ungroup, sel.as_ref().map(|l| l.is_group).unwrap_or(false));
                item(ui, app, Action::Duplicate, sel.is_some());
                item(ui, app, Action::Delete, !app.selected_ids.is_empty());
                item(ui, app, Action::ToggleVisibility, !app.selected_ids.is_empty());
                item(ui, app, Action::Randomize, sel.as_ref().map(|l| !l.is_group).unwrap_or(false));
                ui.separator();
                item(ui, app, Action::MoveUp, sel.is_some());
                item(ui, app, Action::MoveDown, sel.is_some());
            });
            ui.menu_button(tr("View"), |ui| {
                item(ui, app, Action::ZoomIn, has);
                item(ui, app, Action::ZoomOut, has);
                item(ui, app, Action::ZoomFit, has);
                item(ui, app, Action::Zoom100, has);
                item(ui, app, Action::Present, has);
                ui.separator();
                item(ui, app, Action::Filmstrip, true);
                let many = app.docs.len() >= 2;
                item(ui, app, Action::PrevImage, many);
                item(ui, app, Action::NextImage, many);
                ui.separator();
                item(ui, app, Action::ShowLayers, true);
                item(ui, app, Action::ShowPresets, has);
            });
            ui.separator();
            if ui.button(tr("Settings…")).clicked() {
                app.show_settings = true;
                ui.close();
            }
        });
    }

    // MARK: - Info bar

    fn info_bar(&mut self, ui: &mut Ui) {
        let font = regular(11.0);
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 10.0;
            ui.add_space(12.0);
            let sep = |ui: &mut Ui| {
                let (r, _) = ui.allocate_exact_size(vec2(1.0, 12.0), Sense::hover());
                ui.painter().rect_filled(r, CornerRadius::ZERO, separator());
            };
            if let Some(src) = self.app.source() {
                let st = self.app.state();
                let (w, h) = src.cropped_size(&st.crop);
                if self.app.docs.len() > 1 {
                    if let Some(i) = self.app.active_index() {
                        ui.label(egui::RichText::new(format!("{} / {}", i + 1, self.app.docs.len())).font(value_font()).color(text_dim()))
                            .on_hover_text(trf("Image %d of %d · ← → to switch", &[&(i + 1), &self.app.docs.len()]));
                        sep(ui);
                    }
                }
                info_item(ui, "photo", &src.file_name, "File");
                sep(ui);
                info_item(ui, "aspectratio", &format!("{w} × {h} px"), "Size");
                sep(ui);
                ui.label(egui::RichText::new(format!("{} · {}", aspect(w, h), megapixels(w, h))).font(font.clone()).color(text_faint())).on_hover_text(tr("Aspect ratio and megapixels"));
                if !st.crop.is_full() {
                    sep(ui);
                    info_item(ui, "crop", &trf("Cropped from %@", &[&format!("{} × {}", src.width, src.height)]), "Crop");
                }
                if st.active_canvas().is_some() {
                    let (cw, ch) = self.app.output_size();
                    sep(ui);
                    info_item(ui, "aspectratio", &trf("Canvas %@", &[&format!("{cw} × {ch}")]), "Canvas & Borders");
                }
                sep(ui);
                let ext = std::path::Path::new(&src.file_name).extension().map(|e| e.to_string_lossy().to_uppercase()).unwrap_or_else(|| src.kind.clone());
                ui.label(egui::RichText::new(format!("{}{}", ext, if src.has_alpha { format!(" · {}", tr("Alpha")) } else { String::new() })).font(font.clone()).color(text_faint()))
                    .on_hover_text(tr("File type"));
                sep(ui);
                let n = st.layers.flattened().iter().filter(|r| !r.0.is_group).count();
                info_item(ui, "square.3.layers.3d", &trf("%d layers", &[&n]), "Layers");
                if self.app.is_dirty() {
                    sep(ui);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 4.0;
                        let (r, _) = ui.allocate_exact_size(vec2(6.0, 6.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 3.0, ACCENT);
                        ui.label(egui::RichText::new(if self.app.project_path().is_none() { tr("Not saved") } else { tr("Edited") }).font(font.clone()).color(text_dim()));
                    })
                    .response
                    .on_hover_text(tr("Unsaved changes are kept automatically and come back when you reopen the app"));
                }
            } else {
                ui.label(egui::RichText::new(tr("No image open")).font(font.clone()).color(text_faint()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(12.0);
                self.zoom_control(ui);
                let fs = settings::get().show_filmstrip;
                if ui
                    .add(egui::Label::new(egui::RichText::new(icon_text("photo.stack")).font(icon_font(12.0)).color(if fs { accent_text() } else { text_dim() })).sense(Sense::click()))
                    .on_hover_text(format!("{}  {}", tr("Show / Hide Filmstrip"), shortcuts::display(Action::Filmstrip)))
                    .clicked()
                {
                    settings::update(|s| s.show_filmstrip = !s.show_filmstrip);
                }
                if self.app.is_rendering {
                    ui.add(egui::Spinner::new().size(11.0)).on_hover_text(tr("Rendering…"));
                }
            });
        });
    }

    fn zoom_control(&mut self, ui: &mut Ui) {
        let has = self.app.has_image();
        let ctx = ui.ctx().clone();
        let frame = egui::Frame::new().fill(field()).corner_radius(CornerRadius::same(255)).inner_margin(egui::Margin::symmetric(12, 2));
        ui.add_enabled_ui(has, |ui| {
            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    if small_icon(ui, "rectangle.inset.filled", &format!("{}  {}", tr("Full Screen Preview"), shortcuts::display(Action::Present))).clicked() {
                        self.app.toggle_presentation(&ctx);
                    }
                    if small_icon(ui, "arrow.up.left.and.arrow.down.right", &format!("{}  {}", tr("Fit to Window"), shortcuts::display(Action::ZoomFit))).clicked() {
                        self.app.set_zoom(None);
                    }
                    let (r, _) = ui.allocate_exact_size(vec2(1.0, 12.0), Sense::hover());
                    ui.painter().rect_filled(r, CornerRadius::ZERO, separator());
                    if small_icon(ui, "plus", &format!("{}  {}", tr("Zoom In"), shortcuts::display(Action::ZoomIn))).clicked() {
                        self.app.zoom_in();
                    }
                    let pct = if has { format!("{}%", (self.app.display_zoom() * self.app.pixels_per_point * 100.0).round() as i64) } else { "—".into() };
                    let zr = ui.add_sized(vec2(44.0, 18.0), egui::Label::new(egui::RichText::new(pct).font(value_font())).sense(Sense::click()));
                    let zr = zr.on_hover_text(tr("Zoom level · Z fits the image"));
                    egui::Popup::menu(&zr).show(|ui| {
                        if ui.button(format!("{}  Z", tr("Fit to Window"))).clicked() {
                            self.app.set_zoom(None);
                            ui.close();
                        }
                        ui.separator();
                        for (l, z) in [("50%", 0.5), ("100%", 1.0), ("200%", 2.0), ("400%", 4.0)] {
                            if ui.button(l).clicked() {
                                let ppp = self.app.pixels_per_point;
                                self.app.set_zoom(Some(z / ppp));
                                ui.close();
                            }
                        }
                    });
                    if small_icon(ui, "minus", &format!("{}  {}", tr("Zoom Out"), shortcuts::display(Action::ZoomOut))).clicked() {
                        self.app.zoom_out();
                    }
                });
            });
        });
    }

    fn handle_drops(&mut self, ctx: &egui::Context) {
        let files: Vec<PathBuf> = ctx.input(|i| i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect());
        if files.is_empty() {
            return;
        }
        ctx.input_mut(|i| i.raw.dropped_files.clear());
        let (luts, mut others): (Vec<PathBuf>, Vec<PathBuf>) = files.into_iter().partition(|p| p.extension().map(|e| e.eq_ignore_ascii_case("cube")).unwrap_or(false));
        if self.app.has_image() && !luts.is_empty() {
            if let Some(l) = inspector::add_luts(&luts) {
                self.app.add_lut_layer(l);
            }
        }
        if !others.is_empty() {
            others.sort_by_key(|p| p.file_name().map(|f| f.to_string_lossy().to_lowercase()).unwrap_or_default());
            self.app.open_paths(others);
        }
    }

    fn quit_dialog(&mut self, ctx: &egui::Context) {
        egui::Modal::new(Id::new("quit")).show(ctx, |ui| {
            ui.set_width(340.0);
            ui.label(egui::RichText::new(tr("Save changes to this project?")).font(semibold(14.0)).color(text()));
            ui.add_space(6.0);
            ui.label(egui::RichText::new(tr("Your changes will be lost if you don't save them.")).font(regular(12.0)).color(text_dim()));
            ui.add_space(12.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pill_button(ui, &tr("Save"), true).clicked() {
                    self.app.save(false);
                    if !self.app.is_dirty() {
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    self.confirm_quit = false;
                }
                if pill_button(ui, &tr("Don't Save"), false).clicked() {
                    self.allow_close = true;
                    self.confirm_quit = false;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                if pill_button(ui, &tr("Cancel"), false).clicked() {
                    self.confirm_quit = false;
                }
            });
        });
    }
}

fn widgets_picker_open(ctx: &egui::Context) -> bool { egui::Popup::is_any_open(ctx) || picker_open_flag(ctx) }

fn picker_open_flag(ctx: &egui::Context) -> bool {
    let frame = ctx.cumulative_frame_nr();
    ctx.data(|d| d.get_temp::<u64>(Id::new("picker-open-frame")).map(|f| f + 1 >= frame).unwrap_or(false))
}

fn tool_button(ui: &mut Ui, symbol: &str, help: &str, enabled: bool, active: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(32.0, 28.0), if enabled { Sense::click() } else { Sense::hover() });
    let hover = resp.hovered() && enabled;
    if active {
        ui.painter().rect_filled(rect, CornerRadius::same(7), field_strong());
    } else if hover {
        ui.painter().rect_filled(rect, CornerRadius::same(7), row_hover());
    }
    let c = if !enabled { with_alpha(text_dim(), 0.35) } else if active { accent_text() } else { text_dim() };
    paint_icon(ui, rect, symbol, 16.0, c);
    resp.on_hover_text(help)
}

fn small_icon(ui: &mut Ui, symbol: &str, help: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(14.0, 18.0), Sense::click());
    paint_icon(ui, rect, symbol, 11.0, if resp.hovered() { text() } else { text_dim() });
    resp.on_hover_text(help)
}

fn info_item(ui: &mut Ui, symbol: &str, text_: &str, help: &str) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.label(egui::RichText::new(icon_text(symbol)).font(icon_font(11.0)).color(text_faint()));
        ui.add(egui::Label::new(egui::RichText::new(text_).font(regular(11.0)).color(text_dim())).truncate());
    })
    .response
    .on_hover_text(tr(help));
}

fn aspect(w: usize, h: usize) -> String {
    fn gcd(a: usize, b: usize) -> usize { if b == 0 { a } else { gcd(b, a % b) } }
    let g = gcd(w, h).max(1);
    let (a, b) = (w / g, h / g);
    if a <= 32 && b <= 32 { format!("{a}:{b}") } else { format!("{:.2}:1", w as f64 / h.max(1) as f64) }
}
fn megapixels(w: usize, h: usize) -> String { format!("{:.1} MP", (w * h) as f64 / 1_000_000.0) }

impl eframe::App for EffectorUi {
    fn ui(&mut self, ui: &mut Ui, frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.apply_theme(&ctx);
        self.app.pixels_per_point = ctx.pixels_per_point() as f64;
        if self.first_frame {
            self.first_frame = false;
            let files = std::mem::take(&mut self.pending_open);
            if files.is_empty() {
                self.app.restore_session();
            } else {
                self.app.open_paths(files);
            }
        }
        // window title
        let title = match (self.app.project_path(), self.app.doc()) {
            (Some(p), _) => p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(),
            (None, Some(d)) => d.file_name.clone(),
            _ => "EFFECTOR".into(),
        };
        if title != self.last_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(if title == "EFFECTOR" { title.clone() } else { format!("{title} — EFFECTOR") }));
            self.last_title = title;
        }
        // close requests
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            let s = settings::get();
            if s.reopen_last_session {
                self.app.save_session(true);
            } else if self.app.is_dirty() && !self.app.layers().is_empty() {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.confirm_quit = true;
            }
        }
        self.app.tick(&ctx);
        self.handle_keys(&ctx);

        // GPU textures → egui
        let mut texs = canvas_view::CanvasTextures { output: None, original: None, crop: None, checker: self.checker.id(), out_size: vec2(0.0, 0.0) };
        if let Some(rs) = frame.wgpu_render_state() {
            let zoom_px = self.app.display_zoom() * self.app.pixels_per_point;
            let nearest = self.app.output.as_ref().map(|o| {
                let (ow, _) = self.app.output_size();
                let tex_per_screen = o.width() as f64 / (ow as f64 * zoom_px).max(1.0);
                tex_per_screen < 0.5
            }).unwrap_or(false);
            texs.output = self.out_tex.sync(rs, self.app.display.as_ref(), nearest);
            texs.original = self.orig_tex.sync(rs, self.app.display_original.as_ref(), nearest);
            texs.crop = self.crop_tex.sync(rs, self.app.crop_view.as_ref(), false);
        }

        let presenting = self.app.presenting;
        if !presenting {
            egui::Panel::top("toolbar").exact_size(46.0).frame(egui::Frame::new().fill(panel_bg())).show_separator_line(true).show(ui, |ui| self.toolbar(ui));
            let s = settings::get();
            let total = ui.available_width();
            let max_left = 200f32.max(480f32.min(total - s.right_width as f32 - 420.0));
            let max_right = 240f32.max(520f32.min(total - s.left_width as f32 - 420.0));
            let left = egui::Panel::left("left")
                .resizable(true)
                .default_size(s.left_width as f32)
                .size_range(200.0..=max_left)
                .frame(egui::Frame::new().fill(panel_bg()))
                .show(ui, |ui| self.sidebar(ui));
            let right = egui::Panel::right("right")
                .resizable(true)
                .default_size(s.right_width as f32)
                .size_range(240.0..=max_right)
                .frame(egui::Frame::new().fill(panel_bg()))
                .show(ui, |ui| inspector::inspector(ui, &mut self.app));
            let (lw, rw) = (left.response.rect.width() as f64, right.response.rect.width() as f64);
            if (lw - s.left_width).abs() > 1.0 || (rw - s.right_width).abs() > 1.0 {
                settings::update(|s| {
                    s.left_width = lw;
                    s.right_width = rw;
                });
            }
            egui::Panel::bottom("infobar").exact_size(28.0).frame(egui::Frame::new().fill(panel_bg())).show_separator_line(true).show(ui, |ui| self.info_bar(ui));
            if settings::get().show_filmstrip && !self.app.docs.is_empty() {
                egui::Panel::bottom("filmstrip").exact_size(84.0).frame(egui::Frame::new().fill(panel_bg())).show_separator_line(false).show(ui, |ui| filmstrip::filmstrip(ui, &mut self.app));
            }
        }
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let rect = ui.available_rect_before_wrap();
            canvas_view::canvas(ui, &mut self.app, &texs);
            // effect browser floats over the left side of the canvas
            if self.app.show_effect_browser && !presenting {
                let left = rect.left();
                let backdrop = Rect::from_min_max(pos2(left, rect.top()), rect.max);
                let resp = ui.interact(backdrop, Id::new("browser-backdrop"), Sense::click());
                ui.painter().rect_filled(backdrop, CornerRadius::ZERO, Color32::from_black_alpha(46));
                if resp.clicked() {
                    self.app.show_effect_browser = false;
                    self.app.set_preview_layers(None);
                }
                let panel = Rect::from_min_size(pos2(left + 8.0, rect.top() + 8.0), vec2(440.0f32.min(rect.width() - 16.0), rect.height() - 16.0));
                egui::Area::new(Id::new("effect-browser-area")).order(egui::Order::Middle).fixed_pos(panel.min).show(&ctx, |ui| {
                    egui::Frame::new()
                        .fill(panel_bg())
                        .stroke(egui::Stroke::new(1.0, separator()))
                        .corner_radius(CornerRadius::same(14))
                        .shadow(egui::Shadow { offset: [0, 10], blur: 28, spread: 0, color: Color32::from_black_alpha(115) })
                        .show(ui, |ui| {
                            ui.set_width(panel.width());
                            ui.set_height(panel.height());
                            browsers::effect_browser(ui, &mut self.app);
                        });
                });
            }
        });
        if self.app.show_command_palette {
            browsers::command_palette(&ctx, &mut self.app);
        }
        if self.app.show_canvas_popover && self.app.has_image() {
            dialogs::canvas_popover(&ctx, &mut self.app, self.canvas_btn_rect);
        }
        if self.app.show_export {
            dialogs::export_sheet(&ctx, &mut self.app);
        }
        if self.app.show_settings {
            dialogs::settings_window(&ctx, &mut self.app);
        }
        dialogs::dialogs(&ctx, &mut self.app);
        if self.confirm_quit {
            self.quit_dialog(&ctx);
        }
        self.handle_drops(&ctx);
        self.screenshot_mode(&ctx);
        // repaint while files hover (drop highlight)
        if ctx.input(|i| !i.raw.hovered_files.is_empty()) {
            ctx.request_repaint();
        }
    }

    fn on_exit(&mut self) {
        if settings::get().reopen_last_session {
            self.app.save_session(true);
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        let c = panel_bg();
        let [r, g, b, _] = c.to_normalized_gamma_f32();
        [r, g, b, 1.0]
    }
}

impl EffectorUi {
    /// Dev/test helper: runs a tiny script of UI actions, then saves a screenshot and quits.
    /// Script: comma-separated steps like `effect:film`, `preset:Neon Noir`, `browser`, `presets`, `select:0`, `wait`.
    fn screenshot_mode(&mut self, ctx: &egui::Context) {
        let Some((path, script)) = self.shot.clone() else { return };
        let start = *self.shot_start.get_or_insert_with(std::time::Instant::now);
        ctx.request_repaint();
        let steps: Vec<&str> = script.split(',').filter(|s| !s.is_empty()).collect();
        let t = start.elapsed().as_secs_f32();
        // one step per 1.2 s after a 3 s warm-up
        let due = if t < 3.0 { 0 } else { ((t - 3.0) / 1.2) as usize + 1 };
        while self.shot_step < due.min(steps.len()) {
            let st = steps[self.shot_step];
            let (cmd, arg) = st.split_once(':').unwrap_or((st, ""));
            match cmd {
                "effect" => self.app.add_effect(arg),
                "preset" => {
                    if let Some(p) = crate::presets::all().into_iter().find(|p| p.name == arg) {
                        self.app.apply_preset(&p);
                    }
                }
                "browser" => self.app.show_effect_browser = true,
                "presets" => self.app.set_show_presets(true),
                "palette" => self.app.show_command_palette = true,
                "export" => self.app.show_export = true,
                "settings" => self.app.show_settings = true,
                "crop" => self.app.begin_crop(),
                "split" => self.app.compare_split = true,
                "canvas" => self.app.show_canvas_popover = true,
                "light" => settings::update(|s| s.theme = Theme::Light),
                "pt" => settings::update(|s| s.language = settings::Language::Pt),
                "select" => {
                    let i: usize = arg.parse().unwrap_or(0);
                    let id = self.app.layers().flattened().get(i).map(|r| r.0.id);
                    self.app.set_selected(id);
                }
                _ => {}
            }
            self.shot_step += 1;
        }
        let done_at = 3.0 + steps.len() as f32 * 1.2 + 2.5;
        if t > done_at && self.shot_step != usize::MAX {
            self.shot_step = usize::MAX;
            let c2 = ctx.clone();
            ctx.request_screenshot(move |img| {
                let [w, h] = img.size;
                let rgba: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
                if let Ok(b) = crate::imageio::encode(&rgba, w, h, crate::imageio::Format::Png, 1.0, false) {
                    let _ = std::fs::write(&path, b);
                }
                c2.data_mut(|d| d.insert_temp(Id::new("shot-done"), true));
                c2.request_repaint();
            });
        }
        if ctx.data(|d| d.get_temp::<bool>(Id::new("shot-done")).unwrap_or(false)) {
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// Layers on top; when Presets is open it takes the lower part (split adjustable by dragging).
    fn sidebar(&mut self, ui: &mut Ui) {
        let docked = self.app.show_preset_browser && self.app.has_image();
        let rect = ui.available_rect_before_wrap();
        if !docked {
            self.presets_rect = Rect::NOTHING;
            layers::layers_panel(ui, &mut self.app);
            return;
        }
        let h = rect.height();
        let frac = settings::get().presets_split as f32;
        let presets_h = (h * frac).max(180.0).min((h - 160.0).max(180.0));
        let split_y = rect.bottom() - presets_h;
        let top = Rect::from_min_max(rect.min, pos2(rect.right(), split_y - 0.5));
        let bottom = Rect::from_min_max(pos2(rect.left(), split_y + 0.5), rect.max);
        ui.scope_builder(egui::UiBuilder::new().max_rect(top), |ui| {
            ui.set_clip_rect(top);
            layers::layers_panel(ui, &mut self.app);
        });
        // split handle
        let hr = Rect::from_center_size(pos2(rect.center().x, split_y), vec2(rect.width(), 9.0));
        let hresp = ui.interact(hr, Id::new("presets-split"), Sense::click_and_drag());
        let active = hresp.hovered() || hresp.dragged();
        ui.painter().rect_filled(Rect::from_center_size(pos2(rect.center().x, split_y), vec2(rect.width(), 1.0)), CornerRadius::ZERO, if active { with_alpha(ACCENT, 0.6) } else { separator() });
        if active {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
        }
        if hresp.dragged() {
            if let Some(p) = hresp.interact_pointer_pos() {
                let f = ((rect.bottom() - p.y) / h.max(1.0)).clamp(0.2, 0.85) as f64;
                settings::update(|s| s.presets_split = f);
            }
        }
        if hresp.double_clicked() {
            settings::update(|s| s.presets_split = 0.5);
        }
        let hresp = hresp.on_hover_text(tr("Drag to resize · double-click to reset"));
        let _ = hresp;
        self.presets_rect = bottom;
        ui.scope_builder(egui::UiBuilder::new().max_rect(bottom), |ui| {
            ui.set_clip_rect(bottom);
            browsers::presets_panel(ui, &mut self.app);
        });
    }
}

/// Starts the windowed app.
pub fn run(open: Vec<PathBuf>) -> eframe::Result {
    let icon = crate::appicon::icon_rgba(256);
    let viewport = egui::ViewportBuilder::default()
        .with_title("EFFECTOR")
        .with_inner_size([1440.0, 900.0])
        .with_min_inner_size([1000.0, 640.0])
        .with_icon(egui::IconData { rgba: icon, width: 256, height: 256 })
        .with_drag_and_drop(true);
    let mut wgpu_options = egui_wgpu::WgpuConfiguration::default();
    if let egui_wgpu::WgpuSetup::CreateNew(ref mut c) = wgpu_options.wgpu_setup {
        c.power_preference = eframe::wgpu::PowerPreference::HighPerformance;
        c.device_descriptor = Arc::new(|adapter: &eframe::wgpu::Adapter| eframe::wgpu::DeviceDescriptor {
            label: Some("effector"),
            required_features: crate::gpu::wanted_features(adapter),
            required_limits: crate::gpu::wanted_limits(adapter),
            ..Default::default()
        });
    }
    let options = eframe::NativeOptions { viewport, renderer: eframe::Renderer::Wgpu, wgpu_options, centered: true, ..Default::default() };
    eframe::run_native("EFFECTOR", options, Box::new(move |cc| Ok(Box::new(EffectorUi::new(cc, open)))))
}

pub fn version_line() -> String { format!("EFFECTOR {VERSION}") }

