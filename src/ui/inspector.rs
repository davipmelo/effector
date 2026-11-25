//! Inspector (right panel): layer / group / multi-selection / document (canvas & borders)
//! settings, with controls generated from each effect's parameter definitions.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use uuid::Uuid;

use super::theme::*;
use super::widgets::*;
use crate::app::{AppState, Dialog, PromptKind};
use crate::effects::{self, Effect};
use crate::l10n::{tr, trf};
use crate::model::*;
use crate::swatches;

pub fn inspector(ui: &mut Ui, app: &mut AppState) {
    if app.is_multi_selection() {
        multi_inspector(ui, app);
    } else if let Some(layer) = app.selected_layer() {
        layer_inspector(ui, app, &layer);
    } else {
        document_inspector(ui, app);
    }
}

fn blend_items() -> Vec<PickerItem> {
    BlendMode::MENU_ORDER.iter().map(|b| PickerItem::new(&b.id(), &tr(b.label()), Some(tr(b.group())))).collect()
}

/// Blend picker row; handles browse sessions.
fn blend_row(ui: &mut Ui, app: &mut AppState, selected: &str) -> Option<BlendMode> {
    let mut out = None;
    labeled_row(ui, "Blend", |ui| {
        let r = picker_field(ui, "blend", &blend_items(), selected, if selected.is_empty() { Some("—") } else { None }, &[], false);
        if r.opened {
            app.begin_browse();
        }
        if let Some(s) = r.selected {
            out = BlendMode::from_id(&s);
        }
        if r.closed {
            app.end_browse();
        }
    });
    out
}

fn multi_inspector(ui: &mut Ui, app: &mut AppState) {
    let nodes: Vec<LayerNode> = app.selection_roots().iter().filter_map(|id| app.layers().find(*id).cloned()).collect();
    let opacity = nodes.iter().map(|n| n.opacity).sum::<f64>() / nodes.len().max(1) as f64;
    let blends: std::collections::HashSet<BlendMode> = nodes.iter().map(|n| n.blend).collect();
    let blend = if blends.len() == 1 { nodes[0].blend.id() } else { String::new() };
    header(ui, &trf("%d layers selected", &[&app.selected_ids.len()]), Some("square.stack.3d.up"));
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        panel_section(ui, Id::new("multi-layers"), "Layers", None, None, |ui| {
            if let Some(b) = blend_row(ui, app, &blend) {
                app.update_selected(None, |n| n.blend = b);
            }
            if let Some(v) = value_slider(ui, Id::new("multi-opacity"), SliderSpec::new("Opacity", opacity * 100.0, 0.0, 100.0, 100.0).step(1.0).unit("%")) {
                app.update_selected(Some("multi-opacity".into()), |n| n.opacity = v / 100.0);
            }
        });
        egui::Frame::new().inner_margin(14).show(ui, |ui| {
            ui.horizontal(|ui| {
                if pill_icon_button(ui, "folder.badge.plus", &tr("Group"), false).on_hover_text(action_hint("Group selected layers", crate::shortcuts::Action::Group)).clicked() {
                    app.group_selected();
                }
                if pill_icon_button(ui, "plus.square.on.square", &tr("Duplicate"), false).on_hover_text(action_hint("Duplicate", crate::shortcuts::Action::Duplicate)).clicked() {
                    app.duplicate_selected();
                }
            });
            ui.horizontal(|ui| {
                if pill_icon_button(ui, "eye", &tr("Show / Hide"), false).clicked() {
                    app.toggle_selected_visibility();
                }
                if pill_icon_button(ui, "trash", &tr("Delete"), false).clicked() {
                    app.delete_selected();
                }
            });
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(keys_hint(&tr("⇧-click selects a range, ⌘-click adds or removes one layer. Drag any selected layer to move them all.")))
                    .font(regular(11.0))
                    .color(text_faint()),
            );
        });
    });
}

fn info_row(ui: &mut Ui, k: &str, v: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tr(k)).font(label_font()).color(text_dim()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add(egui::Label::new(egui::RichText::new(v).font(value_font()).color(text())).truncate());
        });
    });
}

fn document_inspector(ui: &mut Ui, app: &mut AppState) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        header(ui, &tr("Image"), None);
        if let Some(src) = app.source() {
            let st = app.state();
            let (w, h) = src.cropped_size(&st.crop);
            egui::Frame::new().inner_margin(14).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                info_row(ui, "File", &src.file_name);
                info_row(ui, "Size", &format!("{w} × {h} px"));
                info_row(ui, "Layers", &format!("{}", st.layers.flattened().len()));
                if !st.crop.is_full() {
                    info_row(ui, "Original", &format!("{} × {} px", src.width, src.height));
                }
            });
            separator_line(ui);
            panel_section(ui, Id::new("doc-canvas"), "Canvas & Borders", None, None, |ui| canvas_controls(ui, app));
            egui::Frame::new().inner_margin(14).show(ui, |ui| {
                ui.label(
                    egui::RichText::new(tr("Select a layer to edit its settings, or add an effect with the + button."))
                        .font(regular(11.5))
                        .color(text_faint()),
                );
            });
        } else {
            egui::Frame::new().inner_margin(14).show(ui, |ui| {
                ui.label(egui::RichText::new(tr("No image open")).font(regular(12.0)).color(text_faint()));
            });
        }
    });
}

/// Controls for the aspect-ratio canvas (borders around the image, e.g. for Instagram).
pub fn canvas_controls(ui: &mut Ui, app: &mut AppState) {
    let c = app.state().canvas.unwrap_or(CanvasSpec { enabled: false, ..Default::default() });
    let set = |app: &mut AppState, key: Option<&str>, f: &dyn Fn(&mut CanvasSpec)| {
        app.edit(key.map(|k| format!("canvas-{k}")), |s| {
            let mut cs = s.canvas.clone().unwrap_or(CanvasSpec { enabled: false, ..Default::default() });
            f(&mut cs);
            s.canvas = Some(cs);
        });
    };
    if let Some(v) = toggle_row(ui, "Fit in Canvas", c.enabled) {
        set(app, None, &|s| s.enabled = v);
    }
    let enabled = c.enabled;
    ui.add_enabled_ui(enabled, |ui| {
        if !enabled {
            ui.set_opacity(0.4);
        }
        labeled_row(ui, "Aspect Ratio", |ui| {
            let items: Vec<PickerItem> = CANVAS_RATIOS.iter().map(|r| PickerItem::new(r.id, &tr(r.label), Some(tr(r.group)))).collect();
            let r = picker_field(ui, "canvas.ratio", &items, &c.ratio, None, &[], false);
            if r.opened {
                app.begin_browse();
            }
            if let Some(v) = r.selected {
                set(app, None, &|s| {
                    s.ratio = v.clone();
                    s.enabled = true;
                });
            }
            if r.closed {
                app.end_browse();
            }
        });
        if c.ratio == "custom" {
            if let Some(v) = value_slider(ui, Id::new("cv-w"), SliderSpec::new("Width", c.custom_w, 1.0, 30.0, 4.0).step(0.1)) {
                set(app, Some("w"), &|s| s.custom_w = v);
            }
            if let Some(v) = value_slider(ui, Id::new("cv-h"), SliderSpec::new("Height", c.custom_h, 1.0, 30.0, 5.0).step(0.1)) {
                set(app, Some("h"), &|s| s.custom_h = v);
            }
        }
        if let Some(v) = value_slider(ui, Id::new("cv-pad"), SliderSpec::new("Border", c.padding, 0.0, 40.0, 4.0).step(0.5).unit("%")) {
            set(app, Some("pad"), &|s| s.padding = v);
        }
        let fills = [("color".to_string(), "Color".to_string()), ("blur".into(), "Blurred Photo".into()), ("gradient".into(), "Gradient".into())];
        if let Some(v) = segmented_row(ui, "Fill", &fills, &c.fill) {
            set(app, None, &|s| s.fill = v.clone());
        }
        if c.fill != "blur" {
            if let Some(v) = color_row(ui, if c.fill == "gradient" { "Top Color" } else { "Color" }, c.color) {
                set(app, Some("color"), &|s| s.color = v);
            }
        }
        if c.fill == "gradient" {
            if let Some(v) = color_row(ui, "Bottom Color", c.color2) {
                set(app, Some("color2"), &|s| s.color2 = v);
            }
        }
        if c.fill == "blur" {
            if let Some(v) = value_slider(ui, Id::new("cv-blur"), SliderSpec::new("Background Blur", c.blur, 0.0, 100.0, 60.0).step(1.0).unit("%")) {
                set(app, Some("blur"), &|s| s.blur = v);
            }
        }
        ui.horizontal(|ui| {
            for (hx, name) in [("#FFFFFF", "White"), ("#0A0A0A", "Black"), ("#F3EFE6", "Cream"), ("#FFBB00", "Yellow")] {
                let (r, resp) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
                ui.painter().circle_filled(r.center(), 9.0, rgba_color(&Rgba::hex(hx)));
                ui.painter().circle_stroke(r.center(), 9.0, Stroke::new(1.0, separator()));
                if resp.clicked() {
                    let col = Rgba::hex(hx);
                    set(app, None, &|s| {
                        s.fill = "color".into();
                        s.color = col;
                    });
                }
                resp.on_hover_text(tr(name));
            }
        });
        if let Some(v) = value_slider(ui, Id::new("cv-x"), SliderSpec::new("Position X", c.offset_x, -100.0, 100.0, 0.0).step(1.0).unit("%")) {
            set(app, Some("x"), &|s| s.offset_x = v);
        }
        if let Some(v) = value_slider(ui, Id::new("cv-y"), SliderSpec::new("Position Y", c.offset_y, -100.0, 100.0, 0.0).step(1.0).unit("%")) {
            set(app, Some("y"), &|s| s.offset_y = v);
        }
        if let Some(v) = value_slider(ui, Id::new("cv-r"), SliderSpec::new("Corner Radius", c.radius, 0.0, 50.0, 0.0).step(0.5).unit("%")) {
            set(app, Some("r"), &|s| s.radius = v);
        }
        if let Some(v) = value_slider(ui, Id::new("cv-s"), SliderSpec::new("Shadow", c.shadow, 0.0, 100.0, 0.0).step(1.0).unit("%")) {
            set(app, Some("s"), &|s| s.shadow = v);
        }
    });
    if let (Some(src), true) = (app.source(), c.enabled) {
        let (w, h) = app.output_size();
        let (iw, ih) = src.cropped_size(&app.state().crop);
        ui.label(
            egui::RichText::new(trf("Canvas %@ px · image %@ px", &[&format!("{w} × {h}"), &format!("{iw} × {ih}")])).font(regular(10.5)).color(text_faint()),
        );
    }
}

fn layer_inspector(ui: &mut Ui, app: &mut AppState, layer: &LayerNode) {
    let effect = effects::effect(&layer.effect_id);
    // header with editable name
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    ui.painter().rect_filled(Rect::from_min_size(pos2(rect.left(), rect.bottom() - 1.0), vec2(w, 1.0)), CornerRadius::ZERO, separator());
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink2(vec2(14.0, 6.0))), |ui| {
        ui.horizontal_centered(|ui| {
            let sym = if layer.is_group { "folder" } else { effect.map(|e| e.symbol).unwrap_or("square") };
            ui.label(egui::RichText::new(icon_text(sym)).font(icon_font(14.0)).color(accent_text()));
            let name_id = Id::new(("lname", layer.id));
            let mut name: String = ui.data(|d| d.get_temp(name_id)).unwrap_or_else(|| layer.name.clone());
            let btns = if layer.is_group { 30.0 } else { 86.0 };
            let te = ui.add(egui::TextEdit::singleline(&mut name).font(semibold(13.0)).frame(egui::Frame::NONE).desired_width(ui.available_width() - btns));
            if te.has_focus() {
                ui.data_mut(|d| d.insert_temp(name_id, name.clone()));
            }
            if te.lost_focus() {
                let n = name.trim().to_string();
                if !n.is_empty() && n != layer.name {
                    app.update_layer(layer.id, None, |l| l.name = n.clone());
                }
                ui.data_mut(|d| d.remove::<String>(name_id));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                if icon_button(ui, if layer.enabled { "eye" } else { "eye.slash" }, "Show / Hide", false, 13.0).clicked() {
                    app.toggle_visibility(layer.id);
                }
                if !layer.is_group {
                    if icon_button(ui, "arrow.counterclockwise", "Reset to defaults", false, 13.0).clicked() {
                        app.reset_params(layer.id);
                    }
                    if icon_button(ui, "dice", &format!("{} ({})", tr("Randomize settings"), crate::shortcuts::display(crate::shortcuts::Action::Randomize)), false, 13.0).clicked() {
                        app.randomize_params(layer.id);
                    }
                }
            });
        });
    });
    egui::ScrollArea::vertical().id_salt(("insp", layer.id)).auto_shrink([false, false]).show(ui, |ui| {
        panel_section(ui, Id::new(("sec-layer", layer.id)), if layer.is_group { "Group" } else { "Layer" }, None, None, |ui| {
            if let Some(b) = blend_row(ui, app, &layer.blend.id()) {
                app.update_layer(layer.id, None, |n| n.blend = b);
            }
            if let Some(v) = value_slider(ui, Id::new(("opacity", layer.id)), SliderSpec::new("Opacity", layer.opacity * 100.0, 0.0, 100.0, 100.0).step(1.0).unit("%")) {
                app.update_layer(layer.id, Some(format!("{}-opacity", layer.id)), |n| n.opacity = v / 100.0);
            }
        });
        depth_mask_section(ui, app, layer);
        if let Some(effect) = effect {
            if !effect.looks.is_empty() {
                look_row(ui, app, layer, effect);
            }
            param_sections(ui, app, layer, effect);
        } else if layer.is_group {
            egui::Frame::new().inner_margin(14).show(ui, |ui| {
                ui.label(egui::RichText::new(trf("%d layers in this group", &[&layer.children.len()])).font(regular(11.5)).color(text_faint()));
            });
        }
    });
}

/// Depth Mask: limit the layer to a range of distances (far … near), using the depth map.
fn depth_mask_section(ui: &mut Ui, app: &mut AppState, layer: &LayerNode) {
    let m = layer.depth_mask.clone().unwrap_or(DepthMask { enabled: false, ..Default::default() });
    let lid = layer.id;
    let id = Id::new(("sec-depth", lid));
    // collapsed by default while off, so it doesn't push the effect settings down
    let open_id = id.with("open");
    if !m.enabled && ui.data(|d| d.get_temp::<bool>(open_id).is_none()) {
        ui.data_mut(|d| d.insert_temp(open_id, false));
    }
    let track: Vec<Rgba> = (0..8).map(|i| { let v = 0.08 + 0.84 * i as f64 / 7.0; Rgba::new(v, v, v) }).collect();
    let change = panel_section(ui, id, "Depth Mask", None, Some(m.enabled), |ui| {
        if app.depth_busy() {
            ui.horizontal(|ui| {
                ui.add(egui::Spinner::new().size(11.0));
                ui.label(egui::RichText::new(tr("Calculating depth…")).font(regular(11.0)).color(text_dim()));
            });
        } else if !crate::depth::model_available() {
            ui.label(egui::RichText::new(tr("Depth model not found: reinstall EFFECTOR to use depth")).font(regular(11.0)).color(text_faint()));
        }
        let presets = [("Background", 0.0, 0.42), ("Middle", 0.3, 0.7), ("Subject", 0.55, 1.0)];
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for (name, a, b) in presets {
                let on = m.enabled && (m.from - a).abs() < 1e-3 && (m.to - b).abs() < 1e-3;
                if ui.add(egui::Button::selectable(on, tr(name))).clicked() {
                    app.update_layer(lid, None, |n| {
                        let mut dm = n.depth_mask.clone().unwrap_or_default();
                        dm.enabled = true;
                        dm.from = a;
                        dm.to = b;
                        n.depth_mask = Some(dm);
                    });
                }
            }
        });
        ui.add_space(4.0);
        let set = |app: &mut AppState, key: &str, f: &dyn Fn(&mut DepthMask)| {
            app.update_layer(lid, Some(format!("{lid}-dm-{key}")), |n| {
                let mut dm = n.depth_mask.clone().unwrap_or_default();
                dm.enabled = true;
                f(&mut dm);
                n.depth_mask = Some(dm);
            });
        };
        if let Some(v) = value_slider(ui, id.with("from"), SliderSpec::new("Far edge", m.from * 100.0, 0.0, 100.0, 0.0).step(1.0).unit("%").track(Some(&track))) {
            set(app, "from", &|d| d.from = v / 100.0);
        }
        if let Some(v) = value_slider(ui, id.with("to"), SliderSpec::new("Near edge", m.to * 100.0, 0.0, 100.0, 45.0).step(1.0).unit("%").track(Some(&track))) {
            set(app, "to", &|d| d.to = v / 100.0);
        }
        if let Some(v) = value_slider(ui, id.with("feather"), SliderSpec::new("Softness", m.feather * 100.0, 0.0, 50.0, 12.0).step(1.0).unit("%")) {
            set(app, "feather", &|d| d.feather = v / 100.0);
        }
        if let Some(v) = toggle_row(ui, "Invert", m.invert) {
            set(app, "invert", &|d| d.invert = v);
        }
        ui.add_space(2.0);
        let dv = app.show_depth;
        if ui.add(egui::Button::selectable(dv, format!("{}  {}", tr("View depth map"), crate::shortcuts::display(crate::shortcuts::Action::ShowDepth)))).clicked() {
            app.show_depth = !dv;
            app.request_render();
        }
        ui.label(egui::RichText::new(tr("0% = farthest, 100% = nearest. The depth map is estimated by AI on this computer.")).font(regular(10.5)).color(text_faint()));
    });
    if let Some(on) = change {
        app.update_layer(lid, None, |n| {
            let mut dm = n.depth_mask.clone().unwrap_or_default();
            dm.enabled = on;
            n.depth_mask = Some(dm);
        });
        if on {
            ui.data_mut(|d| d.insert_temp(open_id, true));
        }
    }
}

fn look_row(ui: &mut Ui, app: &mut AppState, layer: &LayerNode, effect: &Effect) {
    egui::Frame::new().inner_margin(egui::Margin::symmetric(14, 10)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(icon_text("sparkles")).font(icon_font(12.0)).color(accent_text()));
            ui.label(egui::RichText::new(tr("Look")).font(label_font()).color(text_dim()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let items: Vec<PickerItem> = effect.looks.iter().map(|l| PickerItem::new(&l.id, &tr(&l.name), l.group.as_ref().map(|g| tr(g)))).collect();
                let cur = layer.params.text("_look");
                let title = effect.looks.iter().find(|l| l.id == cur).map(|l| tr(&l.name)).unwrap_or_else(|| tr("Custom"));
                let r = picker_field(ui, &format!("look.{}", effect.id), &items, &cur, Some(&title), &[], false);
                if r.opened {
                    app.begin_browse();
                }
                if let Some(id) = r.selected {
                    if let Some(look) = effect.looks.iter().find(|l| l.id == id) {
                        app.apply_look(layer.id, look);
                    }
                }
                if r.closed {
                    app.end_browse();
                }
            });
        });
    });
    separator_line(ui);
}

fn param_sections(ui: &mut Ui, app: &mut AppState, layer: &LayerNode, effect: &Effect) {
    let visible: Vec<&ParamDef> = effect.params.iter().filter(|d| d.is_visible(&layer.params)).collect();
    let mut order: Vec<String> = vec![];
    for d in &visible {
        let s = d.section.clone().unwrap_or_else(|| "Settings".into());
        if !order.contains(&s) {
            order.push(s);
        }
    }
    for sec in order {
        let items: Vec<&ParamDef> = visible.iter().copied().filter(|d| d.section.clone().unwrap_or_else(|| "Settings".into()) == sec).collect();
        let sw = items.iter().find(|d| d.is_section_switch);
        let on = sw.map(|d| if let ParamKind::Toggle(def) = d.kind { layer.params.bool_or(&d.key, def) } else { true });
        let dot = effect.section_colors.iter().find(|(k, _)| *k == sec).map(|(_, c)| rgba_color(c));
        let change = panel_section(ui, Id::new(("sec", layer.id, &sec)), &sec, dot, on, |ui| {
            for d in items.iter().filter(|d| !d.is_section_switch) {
                param_control(ui, app, layer, d);
            }
        });
        if let (Some(v), Some(d)) = (change, sw) {
            app.set_param(layer.id, &d.key, ParamValue::Bool(v), false);
        }
    }
}

fn param_control(ui: &mut Ui, app: &mut AppState, layer: &LayerNode, def: &ParamDef) {
    let p = &layer.params;
    let lid = layer.id;
    let id = Id::new(("param", lid, &def.key));
    match &def.kind {
        ParamKind::Slider { min, max, def: d, step, unit, curve } => {
            let spec = SliderSpec::new(&def.label, p.num(&def.key, *d), *min, *max, *d).step(*step).unit(unit).curve(*curve).track(def.track.as_deref());
            if let Some(v) = value_slider(ui, id, spec) {
                app.set_param(lid, &def.key, ParamValue::Number(v), true);
            }
        }
        ParamKind::Toggle(d) => {
            if let Some(v) = toggle_row(ui, &def.label, p.bool_or(&def.key, *d)) {
                app.set_param(lid, &def.key, ParamValue::Bool(v), false);
            }
        }
        ParamKind::Choice(o, d) => {
            labeled_row(ui, &def.label, |ui| {
                let items: Vec<PickerItem> = o.iter().map(|c| PickerItem::new(&c.id, &tr(&c.label), c.group.as_ref().map(|g| tr(g)))).collect();
                let r = picker_field(ui, &format!("{}.{}", layer.effect_id, def.key), &items, &p.choice_or(&def.key, d), None, &[], false);
                if r.opened {
                    app.begin_browse();
                }
                if let Some(v) = r.selected {
                    app.set_param(lid, &def.key, ParamValue::Choice(v), false);
                }
                if r.closed {
                    app.end_browse();
                }
            });
        }
        ParamKind::Segmented(o, d) => {
            let opts: Vec<(String, String)> = o.iter().map(|c| (c.id.clone(), c.label.clone())).collect();
            if let Some(v) = segmented_row(ui, &def.label, &opts, &p.choice_or(&def.key, d)) {
                app.set_param(lid, &def.key, ParamValue::Choice(v), false);
            }
        }
        ParamKind::Color(d) => {
            if let Some(v) = color_row(ui, &def.label, p.color_or(&def.key, *d)) {
                app.set_param(lid, &def.key, ParamValue::Color(v), true);
            }
        }
        ParamKind::Curves => {
            if let Some(c) = curve_editor(ui, id, &p.curves(&def.key)) {
                app.set_param(lid, &def.key, ParamValue::Curves(c), true);
            }
        }
        ParamKind::Gradient(d) => {
            let g = p.gradient(&def.key).unwrap_or_else(|| d.clone());
            gradient_editor(ui, app, id, lid, &def.key, &g);
        }
        ParamKind::Palette(d) => {
            let pal = p.palette(&def.key).unwrap_or_else(|| d.clone());
            palette_editor(ui, app, id, lid, &def.key, &pal);
        }
        ParamKind::Text(d) => {
            ui.label(egui::RichText::new(tr(&def.label)).font(label_font()).color(text_dim()));
            let mut t = p.text_or(&def.key, d);
            let multiline = t.contains('\n') || def.key == "osdBottom";
            let te = if multiline {
                ui.add(egui::TextEdit::multiline(&mut t).font(regular(12.0)).desired_rows(2).desired_width(f32::INFINITY))
            } else {
                ui.add(egui::TextEdit::singleline(&mut t).font(regular(12.0)).desired_width(f32::INFINITY))
            };
            if te.changed() {
                app.set_param(lid, &def.key, ParamValue::Text(t), true);
            }
        }
        ParamKind::Lut => lut_control(ui, app, lid, &def.key, p.lut(&def.key)),
        ParamKind::Wheel(prefix) => {
            let size = ui.available_width().min(190.0);
            ui.vertical_centered(|ui| {
                wheel_param(ui, app, layer, prefix, size);
            });
        }
        ParamKind::WheelRow(prefixes) => {
            let n = prefixes.len().max(1) as f32;
            let w = ui.available_width();
            let gap = 10.0;
            let col = ((w - gap * (n - 1.0)) / n).max(40.0);
            let size = if prefixes.len() == 1 { col.min(104.0) } else { col.min(118.0) };
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for pf in prefixes {
                    ui.allocate_ui_with_layout(vec2(col, size + 64.0), egui::Layout::top_down(egui::Align::Center), |ui| {
                        ui.set_width(col);
                        let name = match pf.as_str() { "sh" => "Shadows", "mid" => "Midtones", "hi" => "Highlights", _ => "Global" };
                        ui.label(egui::RichText::new(tr(name)).font(label_font()).color(text_dim()));
                        wheel_param(ui, app, layer, pf, size);
                        let key = format!("{pf}Lum");
                        let lid2 = Id::new(("param", lid, &key));
                        if let Some(v) = value_slider(ui, lid2, SliderSpec::new("Lum", p.num(&key, 0.0), -100.0, 100.0, 0.0).step(1.0)) {
                            app.set_param(lid, &key, ParamValue::Number(v), true);
                        }
                    });
                }
            });
        }
        ParamKind::Seed => {
            let v = p.num(&def.key, 1.0);
            labeled_row(ui, "Seed", |ui| {
                if pill_button(ui, &icon_text("dice"), false).on_hover_text(tr("New random seed")).clicked() {
                    let r = (std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(7) % 9999 + 1) as f64;
                    app.set_param(lid, &def.key, ParamValue::Number(r), false);
                }
                ui.label(egui::RichText::new(format!("{}", v as i64)).font(value_font()).color(text_dim()));
            });
        }
    }
}

// MARK: - Color wheels

/// A hue / saturation wheel bound to `{prefix}Hue` and `{prefix}Sat`.
fn wheel_param(ui: &mut Ui, app: &mut AppState, layer: &LayerNode, prefix: &str, size: f32) {
    let (hk, sk) = (format!("{prefix}Hue"), format!("{prefix}Sat"));
    let (hue, sat) = (layer.params.num(&hk, 0.0), layer.params.num(&sk, 0.0));
    if let Some((h, s)) = color_wheel(ui, Id::new(("wheel", layer.id, prefix)), hue, sat, size) {
        app.update_layer(layer.id, Some(format!("{}-wheel-{prefix}", layer.id)), |n| {
            n.params.set(&hk, ParamValue::Number(h));
            n.params.set(&sk, ParamValue::Number(s));
        });
    }
    ui.label(egui::RichText::new(if sat < 0.5 { "—".to_string() } else { format!("{}°  ·  {}", hue.round() as i64, sat.round() as i64) }).font(value_font()).color(text_faint()));
}

/// Draws a colour wheel (angle = hue, distance = saturation). Drag anywhere; double-click resets;
/// hold Shift for fine control.
pub fn color_wheel(ui: &mut Ui, id: Id, hue: f64, sat: f64, size: f32) -> Option<(f64, f64)> {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click_and_drag());
    let resp = resp.on_hover_text(tr("Drag to pick hue and strength · Shift for fine control · double-click to reset"));
    let c = rect.center();
    let r = size / 2.0 - 3.0;
    let painter = ui.painter_at(rect.expand(2.0));
    // colour field
    let mut mesh = egui::Mesh::default();
    let (rings, segs) = (7usize, 72usize);
    for i in 0..=rings {
        let t = i as f32 / rings as f32;
        for j in 0..=segs {
            let a = j as f32 / segs as f32 * std::f32::consts::TAU;
            let col = crate::effects::color::oklch_rgb(0.7, 0.14 * t as f64, a.to_degrees() as f64);
            mesh.colored_vertex(c + vec2(a.cos(), -a.sin()) * r * t, rgba_color(&col));
        }
    }
    for i in 0..rings {
        for j in 0..segs {
            let a = (i * (segs + 1) + j) as u32;
            let b = a + (segs + 1) as u32;
            mesh.add_triangle(a, a + 1, b);
            mesh.add_triangle(a + 1, b + 1, b);
        }
    }
    painter.add(egui::Shape::mesh(mesh));
    painter.circle_stroke(c, r, egui::Stroke::new(1.0, separator()));
    painter.circle_stroke(c, r * 0.5, egui::Stroke::new(0.5, with_alpha(Color32::WHITE, 0.12)));
    painter.line_segment([c - vec2(5.0, 0.0), c + vec2(5.0, 0.0)], egui::Stroke::new(1.0, with_alpha(Color32::WHITE, 0.35)));
    painter.line_segment([c - vec2(0.0, 5.0), c + vec2(0.0, 5.0)], egui::Stroke::new(1.0, with_alpha(Color32::WHITE, 0.35)));
    // puck
    let a = (hue as f32).to_radians();
    let pr = (sat as f32 / 100.0).clamp(0.0, 1.0) * r;
    let pp = c + vec2(a.cos(), -a.sin()) * pr;
    let pc = rgba_color(&crate::effects::color::oklch_rgb(0.7, 0.14 * sat / 100.0, hue));
    painter.circle_filled(pp, 6.5, Color32::from_black_alpha(90));
    painter.circle_filled(pp, 5.5, pc);
    painter.circle_stroke(pp, 5.5, egui::Stroke::new(2.0, Color32::WHITE));
    if resp.double_clicked() {
        return Some((hue, 0.0));
    }
    let drag_id = id.with("start");
    if resp.drag_started() {
        ui.data_mut(|d| d.insert_temp(drag_id, (pp, resp.interact_pointer_pos().unwrap_or(pp))));
    }
    if resp.dragged() || resp.clicked() {
        let pos = resp.interact_pointer_pos()?;
        let fine = ui.input(|i| i.modifiers.shift);
        let target = match (resp.dragged(), fine, ui.data(|d| d.get_temp::<(egui::Pos2, egui::Pos2)>(drag_id))) {
            (true, true, Some((puck0, ptr0))) => puck0 + (pos - ptr0) * 0.25,
            (true, false, Some((puck0, ptr0))) => puck0 + (pos - ptr0),
            _ => pos,
        };
        let d = target - c;
        let mut h = (-d.y).atan2(d.x).to_degrees() as f64;
        if h < 0.0 {
            h += 360.0;
        }
        let s = ((d.length() / r) as f64 * 100.0).clamp(0.0, 100.0);
        return Some((h.round(), s.round()));
    }
    None
}

// MARK: - Curves

pub fn curve_editor(ui: &mut Ui, id: Id, curves: &CurveSet) -> Option<CurveSet> {
    let mut out = None;
    let ch_id = id.with("channel");
    let mut channel: usize = ui.data(|d| d.get_temp(ch_id).unwrap_or(0));
    let opts: Vec<(String, String)> = [("0", "RGB"), ("1", "Red"), ("2", "Green"), ("3", "Blue")].iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    if let Some(v) = segmented_row(ui, "", &opts, &channel.to_string()) {
        channel = v.parse().unwrap_or(0);
        ui.data_mut(|d| d.insert_temp(ch_id, channel));
    }
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, w), Sense::click_and_drag());
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(6), field());
    for i in 1..4 {
        let f = i as f32 / 4.0;
        p.line_segment([pos2(rect.left() + f * w, rect.top()), pos2(rect.left() + f * w, rect.bottom())], Stroke::new(1.0, separator()));
        p.line_segment([pos2(rect.left(), rect.top() + f * w), pos2(rect.right(), rect.top() + f * w)], Stroke::new(1.0, separator()));
    }
    // dashed diagonal
    let n = 24;
    for i in 0..n {
        if i % 2 == 0 {
            let a = i as f32 / n as f32;
            let b = (i + 1) as f32 / n as f32;
            p.line_segment([pos2(rect.left() + a * w, rect.bottom() - a * w), pos2(rect.left() + b * w, rect.bottom() - b * w)], Stroke::new(1.0, with_alpha(text_faint(), 0.4)));
        }
    }
    let colors = [Color32::WHITE, Color32::RED, Color32::GREEN, Color32::from_rgb(77, 140, 255)];
    let curve_pts = |pts: &[CurvePoint]| -> Vec<egui::Pos2> {
        (0..=96)
            .map(|i| {
                let x = i as f64 / 96.0;
                let y = crate::effects::adjust::curve_eval(pts, x);
                pos2(rect.left() + x as f32 * w, rect.top() + (1.0 - y as f32) * w)
            })
            .collect()
    };
    for c in 0..4 {
        if c != channel && *curves.channel(c) != CurveSet::identity() {
            p.add(egui::Shape::line(curve_pts(curves.channel(c)), Stroke::new(1.0, with_alpha(colors[c], 0.3))));
        }
    }
    p.add(egui::Shape::line(curve_pts(curves.channel(channel)), Stroke::new(1.6, if channel == 0 { text() } else { colors[channel] })));
    let mut pts = curves.channel(channel).clone();
    pts.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    let drag_id = id.with("drag");
    let mut drag_index: Option<usize> = ui.data(|d| d.get_temp(drag_id));
    for (i, pt) in pts.iter().enumerate() {
        let c = pos2(rect.left() + pt.x as f32 * w, rect.top() + (1.0 - pt.y as f32) * w);
        p.circle_filled(c, 4.5, if drag_index == Some(i) { ACCENT } else { Color32::WHITE });
        p.circle_stroke(c, 4.5, Stroke::new(1.0, Color32::from_black_alpha(100)));
    }
    let to_norm = |pos: egui::Pos2| ((pos.x - rect.left()) as f64 / w as f64, 1.0 - (pos.y - rect.top()) as f64 / w as f64);
    let nearest = |pts: &[CurvePoint], pos: egui::Pos2| -> Option<usize> {
        let mut best = None;
        let mut bd = 12.0f32;
        for (i, q) in pts.iter().enumerate() {
            let d = (pos2(rect.left() + q.x as f32 * w, rect.top() + (1.0 - q.y as f32) * w) - pos).length();
            if d < bd {
                bd = d;
                best = Some(i);
            }
        }
        best
    };
    if resp.double_clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            if let Some(i) = nearest(&pts, pos) {
                if i > 0 && i < pts.len() - 1 {
                    pts.remove(i);
                    let mut c = curves.clone();
                    *c.channel_mut(channel) = pts.clone();
                    out = Some(c);
                }
            }
        }
    } else if resp.drag_started() || (resp.is_pointer_button_down_on() && drag_index.is_none()) {
        if let Some(pos) = resp.interact_pointer_pos() {
            if let Some(i) = nearest(&pts, pos) {
                drag_index = Some(i);
            } else {
                let x = to_norm(pos).0.clamp(0.0, 1.0);
                let np = CurvePoint { x, y: crate::effects::adjust::curve_eval(&pts, x) };
                pts.push(np);
                pts.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
                drag_index = pts.iter().position(|q| *q == np);
                let mut c = curves.clone();
                *c.channel_mut(channel) = pts.clone();
                out = Some(c);
            }
        }
    }
    if resp.dragged() {
        if let (Some(i), Some(pos)) = (drag_index, resp.interact_pointer_pos()) {
            if i < pts.len() {
                let (mut x, y) = to_norm(pos);
                x = x.clamp(0.0, 1.0);
                let y = y.clamp(0.0, 1.0);
                let lo = if i > 0 { pts[i - 1].x + 0.01 } else { 0.0 };
                let hi = if i < pts.len() - 1 { pts[i + 1].x - 0.01 } else { 1.0 };
                x = if i == 0 { x.min(hi) } else if i == pts.len() - 1 { x.max(lo) } else { x.clamp(lo, hi) };
                pts[i] = CurvePoint { x, y };
                let mut c = curves.clone();
                *c.channel_mut(channel) = pts.clone();
                out = Some(c);
            }
        }
    }
    if resp.drag_stopped() {
        // dragged far outside → remove point
        if let (Some(i), Some(pos)) = (drag_index, resp.interact_pointer_pos()) {
            let outside = pos.y < rect.top() - 20.0 || pos.y > rect.bottom() + 20.0;
            if outside && i > 0 && i < pts.len() - 1 {
                pts.remove(i);
                let mut c = curves.clone();
                *c.channel_mut(channel) = pts.clone();
                out = Some(c);
            }
        }
        drag_index = None;
    }
    if !resp.is_pointer_button_down_on() && !resp.dragged() {
        drag_index = None;
    }
    ui.data_mut(|d| match drag_index {
        Some(i) => {
            d.insert_temp(drag_id, i);
        }
        None => d.remove::<usize>(drag_id),
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tr("Click to add · drag out to remove")).font(regular(10.0)).color(text_faint()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.add(egui::Label::new(egui::RichText::new(tr("Reset")).font(regular(11.0)).color(accent_text())).sense(Sense::click())).clicked() {
                let mut c = curves.clone();
                *c.channel_mut(channel) = CurveSet::identity();
                out = Some(c);
            }
        });
    });
    out
}

// MARK: - Gradient editor

fn preview_colors(g: &GradientSpec) -> Vec<Rgba> { (0..12).map(|i| g.sample(i as f64 / 11.0)).collect() }

fn gradient_editor(ui: &mut Ui, app: &mut AppState, id: Id, lid: Uuid, key: &str, g: &GradientSpec) {
    let lib = swatches::library().lock().unwrap().clone();
    let mut items: Vec<PickerItem> = lib
        .gradients
        .iter()
        .map(|x| PickerItem { id: format!("s:{}", x.name), label: x.name.clone(), group: Some(tr("Saved")), swatches: None, gradient: Some(preview_colors(x)) })
        .collect();
    for (grp, list) in swatches::gradient_groups() {
        for x in list {
            items.push(PickerItem { id: format!("b:{}", x.name), label: x.name.clone(), group: Some(tr(grp)), swatches: None, gradient: Some(preview_colors(x)) });
        }
    }
    let selected = if lib.gradients.iter().any(|x| x.name == g.name) { format!("s:{}", g.name) } else { format!("b:{}", g.name) };
    let saved = lib.gradients.contains(g);
    let mut actions = vec![
        PickerAction { label: tr("Save Gradient…"), symbol: "square.and.arrow.down", destructive: false },
        PickerAction { label: tr("Import Gradients…"), symbol: "square.and.arrow.down.on.square", destructive: false },
        PickerAction { label: tr("Export Gradient…"), symbol: "square.and.arrow.up", destructive: false },
    ];
    if saved {
        actions.push(PickerAction { label: tr("Delete Saved Gradient"), symbol: "trash", destructive: true });
    }
    let set = |app: &mut AppState, g: GradientSpec, coalesce: bool| app.set_param(lid, key, ParamValue::Gradient(g), coalesce);
    ui.horizontal(|ui| {
        let r = picker_field(ui, "gradient", &items, &selected, Some(&g.name), &actions, false);
        if r.opened {
            app.begin_browse();
        }
        if let Some(s) = r.selected {
            let name = &s[2..];
            let found = if s.starts_with("s:") { lib.gradients.iter().find(|x| x.name == name).cloned() } else { swatches::all_gradients().into_iter().find(|x| x.name == name) };
            if let Some(f) = found {
                set(app, f, false);
            }
        }
        if r.closed {
            app.end_browse();
        }
        match r.action {
            Some(0) => {
                app.dialog = Some(Dialog::Prompt { title: tr("Save Gradient"), label: tr("Name"), text: g.name.clone(), kind: PromptKind::SaveGradient { layer: lid, key: key.into(), gradient: g.clone() } })
            }
            Some(1) => {
                if let Some(files) = rfd::FileDialog::new().add_filter("JSON", &["json"]).pick_files() {
                    for f in files {
                        if let Ok(d) = std::fs::read(f) {
                            for x in swatches::parse_gradient_file(&d) {
                                swatches::save_gradient(x);
                            }
                        }
                    }
                }
            }
            Some(2) => {
                if let Some(path) = rfd::FileDialog::new().set_file_name(format!("{}.effectorgradient.json", g.name)).save_file() {
                    let _ = std::fs::write(path, serde_json::to_vec_pretty(&vec![g.clone()]).unwrap_or_default());
                }
            }
            Some(3) => swatches::delete_gradient(g),
            _ => {}
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if icon_button(ui, "arrow.left.arrow.right", "Reverse", false, 12.0).clicked() {
                let mut ng = g.clone();
                for s in ng.stops.iter_mut() {
                    s.position = 1.0 - s.position;
                }
                set(app, ng, false);
            }
        });
    });
    // bar + stops
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 42.0), Sense::hover());
    let bar = Rect::from_min_size(rect.min, vec2(w, 22.0));
    let mut sorted = g.stops.clone();
    sorted.sort_by(|a, b| a.position.partial_cmp(&b.position).unwrap());
    let stops: Vec<(f32, Color32)> = {
        let mut v: Vec<(f32, Color32)> = sorted.iter().map(|s| (s.position as f32, rgba_color(&s.color))).collect();
        if let Some(f) = v.first().copied() {
            if f.0 > 0.0 {
                v.insert(0, (0.0, f.1));
            }
        }
        if let Some(l) = v.last().copied() {
            if l.0 < 1.0 {
                v.push((1.0, l.1));
            }
        }
        v
    };
    paint_gradient_stops(ui, bar, &stops);
    ui.painter().rect_stroke(bar, CornerRadius::same(5), Stroke::new(1.0, separator()), StrokeKind::Inside);
    let bar_resp = ui.interact(bar, id.with("bar"), Sense::click());
    let sel_id = id.with("sel");
    let mut selected_stop: Option<Uuid> = ui.data(|d| d.get_temp(sel_id));
    if bar_resp.clicked() {
        if let Some(pos) = bar_resp.interact_pointer_pos() {
            let t = ((pos.x - bar.left()) / w).clamp(0.0, 1.0) as f64;
            let mut ng = g.clone();
            let s = GradientStop { id: Uuid::new_v4(), position: t, color: ng.sample(t) };
            selected_stop = Some(s.id);
            ng.stops.push(s);
            ng.name = tr("Custom");
            set(app, ng, false);
        }
    }
    for s in &g.stops {
        let cx = bar.left() + s.position as f32 * w;
        let hr = Rect::from_center_size(pos2(cx, rect.top() + 32.0), vec2(14.0, 20.0));
        let hresp = ui.interact(hr, id.with(("stop", s.id)), Sense::click_and_drag());
        let sel = selected_stop == Some(s.id);
        let tri = vec![pos2(cx, rect.top() + 23.0), pos2(cx + 4.0, rect.top() + 28.0), pos2(cx - 4.0, rect.top() + 28.0)];
        ui.painter().add(egui::Shape::convex_polygon(tri, if sel { ACCENT } else { text_dim() }, Stroke::NONE));
        let box_r = Rect::from_center_size(pos2(cx, rect.top() + 34.0), vec2(12.0, 12.0));
        ui.painter().rect_filled(box_r, CornerRadius::same(3), rgba_color(&s.color));
        ui.painter().rect_stroke(box_r, CornerRadius::same(3), Stroke::new(1.5, if sel { ACCENT } else { with_alpha(Color32::WHITE, 0.7) }), StrokeKind::Outside);
        if hresp.clicked() || hresp.drag_started() {
            selected_stop = Some(s.id);
        }
        if hresp.dragged() {
            if let Some(pos) = hresp.interact_pointer_pos() {
                let t = ((pos.x - bar.left()) / w).clamp(0.0, 1.0) as f64;
                let removing = pos.y > rect.top() + 60.0;
                let mut ng = g.clone();
                if removing && ng.stops.len() > 2 {
                    ng.stops.retain(|x| x.id != s.id);
                    selected_stop = None;
                } else if let Some(x) = ng.stops.iter_mut().find(|x| x.id == s.id) {
                    x.position = t;
                }
                set(app, ng, true);
            }
        }
    }
    if let Some(sid) = selected_stop.filter(|sid| g.stops.iter().any(|x| x.id == *sid)) {
        let stop = g.stops.iter().find(|x| x.id == sid).unwrap().clone();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(tr("Stop")).font(label_font()).color(text_dim()));
            ui.label(egui::RichText::new(format!("{}%", (stop.position * 100.0) as i64)).font(value_font()).color(text_faint()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled_ui(g.stops.len() > 2, |ui| icon_button(ui, "trash", "", false, 12.0)).inner.clicked() {
                    let mut ng = g.clone();
                    ng.stops.retain(|x| x.id != sid);
                    selected_stop = None;
                    set(app, ng, false);
                }
                if let Some(c) = color_well(ui, stop.color, id.with(("stopcol", sid))) {
                    let mut ng = g.clone();
                    if let Some(x) = ng.stops.iter_mut().find(|x| x.id == sid) {
                        x.color = c;
                    }
                    set(app, ng, true);
                }
            });
        });
    } else {
        ui.label(egui::RichText::new(tr("Click the bar to add a stop")).font(regular(10.0)).color(text_faint()));
    }
    ui.data_mut(|d| match selected_stop {
        Some(s) => {
            d.insert_temp(sel_id, s);
        }
        None => d.remove::<Uuid>(sel_id),
    });
}

// MARK: - Palette editor

fn palette_editor(ui: &mut Ui, app: &mut AppState, id: Id, lid: Uuid, key: &str, pal: &PaletteSpec) {
    let lib = swatches::library().lock().unwrap().clone();
    let mut items: Vec<PickerItem> = lib
        .palettes
        .iter()
        .map(|x| PickerItem { id: format!("s:{}", x.name), label: x.name.clone(), group: Some(tr("Saved")), swatches: Some(x.colors.clone()), gradient: None })
        .collect();
    for (grp, list) in swatches::palette_groups() {
        for x in list {
            items.push(PickerItem { id: format!("b:{}", x.name), label: x.name.clone(), group: Some(tr(grp)), swatches: Some(x.colors.clone()), gradient: None });
        }
    }
    let selected = if lib.palettes.iter().any(|x| x.name == pal.name) { format!("s:{}", pal.name) } else { format!("b:{}", pal.name) };
    let mut actions = vec![
        PickerAction { label: tr("Save Palette…"), symbol: "square.and.arrow.down", destructive: false },
        PickerAction { label: tr("Import Palettes…"), symbol: "square.and.arrow.down.on.square", destructive: false },
        PickerAction { label: tr("Export Palette…"), symbol: "square.and.arrow.up", destructive: false },
    ];
    if lib.palettes.contains(pal) {
        actions.push(PickerAction { label: tr("Delete Saved Palette"), symbol: "trash", destructive: true });
    }
    let set = |app: &mut AppState, p: PaletteSpec| app.set_param(lid, key, ParamValue::Palette(p), false);
    ui.horizontal(|ui| {
        let r = picker_field(ui, "palette", &items, &selected, Some(&pal.name), &actions, false);
        if r.opened {
            app.begin_browse();
        }
        if let Some(s) = r.selected {
            let name = &s[2..];
            let found = if s.starts_with("s:") { lib.palettes.iter().find(|x| x.name == name).cloned() } else { swatches::all_palettes().into_iter().find(|x| x.name == name) };
            if let Some(f) = found {
                set(app, f);
            }
        }
        if r.closed {
            app.end_browse();
        }
        match r.action {
            Some(0) => {
                app.dialog = Some(Dialog::Prompt { title: tr("Save Palette"), label: tr("Name"), text: pal.name.clone(), kind: PromptKind::SavePalette { layer: lid, key: key.into(), palette: pal.clone() } })
            }
            Some(1) => {
                if let Some(files) = rfd::FileDialog::new().add_filter(&tr("Palettes"), &["hex", "gpl", "txt", "json"]).pick_files() {
                    let mut last = None;
                    for f in files {
                        let name = f.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        let ext = f.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        if let Ok(t) = std::fs::read_to_string(&f) {
                            if let Some(p) = swatches::parse_palette_file(&name, &ext, &t) {
                                swatches::save_palette(p.clone());
                                last = Some(p);
                            }
                        }
                    }
                    if let Some(p) = last {
                        set(app, p);
                    }
                }
            }
            Some(2) => {
                if let Some(path) = rfd::FileDialog::new().set_file_name(format!("{}.hex", pal.name)).add_filter("HEX", &["hex"]).add_filter("GIMP", &["gpl"]).save_file() {
                    let gpl = path.extension().map(|e| e.eq_ignore_ascii_case("gpl")).unwrap_or(false);
                    let _ = std::fs::write(path, swatches::palette_file_text(pal, gpl));
                }
            }
            Some(3) => swatches::delete_palette(pal),
            _ => {}
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(egui::RichText::new(format!("{}", pal.colors.len())).font(value_font()).color(text_faint()));
        });
    });
    // grid of colours (8 per row)
    let sel_id = id.with("sel");
    let mut selected_i: Option<usize> = ui.data(|d| d.get_temp(sel_id));
    let w = ui.available_width();
    let cell = (w - 7.0 * 4.0) / 8.0;
    let n = pal.colors.len() + if pal.colors.len() < 64 { 1 } else { 0 };
    let rows = n.div_ceil(8);
    let (rect, _) = ui.allocate_exact_size(vec2(w, rows as f32 * (cell + 4.0)), Sense::hover());
    for i in 0..n {
        let (cx, cy) = (i % 8, i / 8);
        let r = Rect::from_min_size(pos2(rect.left() + cx as f32 * (cell + 4.0), rect.top() + cy as f32 * (cell + 4.0)), vec2(cell, cell));
        let resp = ui.interact(r, id.with(("cell", i)), Sense::click());
        if i < pal.colors.len() {
            ui.painter().rect_filled(r, CornerRadius::same(4), rgba_color(&pal.colors[i]));
            let sel = selected_i == Some(i);
            ui.painter().rect_stroke(r, CornerRadius::same(4), Stroke::new(if sel { 2.0 } else { 1.0 }, if sel { ACCENT } else { separator() }), StrokeKind::Inside);
            if resp.clicked() {
                selected_i = if sel { None } else { Some(i) };
            }
            resp.on_hover_text(format!("{} · {}", pal.colors[i].to_hex(), tr("click to edit")));
        } else {
            ui.painter().rect_stroke(r, CornerRadius::same(4), Stroke::new(1.0, text_faint()), StrokeKind::Inside);
            paint_icon(ui, r, "plus", 10.0, text_dim());
            if resp.clicked() {
                let mut p = pal.clone();
                p.colors.push(p.colors.last().copied().unwrap_or(Rgba::WHITE));
                p.name = tr("Custom");
                selected_i = Some(p.colors.len() - 1);
                set(app, p);
            }
            resp.on_hover_text(tr("Add a color"));
        }
    }
    if let Some(i) = selected_i.filter(|i| *i < pal.colors.len()) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(pal.colors[i].to_hex()).font(value_font()).color(text_dim()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add_enabled_ui(pal.colors.len() > 2, |ui| icon_button(ui, "trash", "", false, 12.0)).inner.clicked() {
                    let mut p = pal.clone();
                    p.colors.remove(i);
                    p.name = tr("Custom");
                    selected_i = None;
                    set(app, p);
                }
                if let Some(c) = color_well(ui, pal.colors[i], id.with(("pc", i))) {
                    let mut p = pal.clone();
                    p.colors[i] = c;
                    p.name = tr("Custom");
                    app.set_param(lid, key, ParamValue::Palette(p), true);
                }
            });
        });
    }
    ui.data_mut(|d| match selected_i {
        Some(s) => {
            d.insert_temp(sel_id, s);
        }
        None => d.remove::<usize>(sel_id),
    });
    // extract from image
    let cnt_id = id.with("extract");
    let mut count: usize = ui.data(|d| d.get_temp(cnt_id).unwrap_or(6));
    ui.horizontal(|ui| {
        if pill_icon_button(ui, "drop", &tr("Extract from image"), false).on_hover_text(tr("Build a palette from the colors of your image")).clicked() {
            if let Some(src) = app.source() {
                let (px, w, h) = src.rgba8_thumbnail(512);
                set(app, swatches::extract_palette(&px, w, h, count));
            }
        }
        if icon_button(ui, "minus", "", false, 11.0).clicked() {
            count = count.saturating_sub(1).max(2);
        }
        ui.label(egui::RichText::new(format!("{count}")).font(value_font()).color(text_dim()));
        if icon_button(ui, "plus", "", false, 11.0).clicked() {
            count = (count + 1).min(16);
        }
    });
    ui.data_mut(|d| d.insert_temp(cnt_id, count));
}

// MARK: - LUT

pub fn lut_dir() -> std::path::PathBuf {
    let d = crate::settings::support_dir().join("LUTs");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn lut_names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(lut_dir())
        .map(|rd| {
            rd.flatten()
                .filter(|e| e.path().extension().map(|x| x.eq_ignore_ascii_case("cube")).unwrap_or(false))
                .filter_map(|e| e.path().file_stem().map(|s| s.to_string_lossy().to_string()))
                .collect()
        })
        .unwrap_or_default();
    v.sort_by_key(|a| a.to_lowercase());
    v
}

pub fn load_lut(name: &str) -> Option<LutData> {
    let text = std::fs::read_to_string(lut_dir().join(format!("{name}.cube"))).ok()?;
    crate::effects::color::parse_cube(name, &text)
}

/// Copies .cube files into the library; returns the last one.
pub fn add_luts(paths: &[std::path::PathBuf]) -> Option<LutData> {
    let mut last = None;
    for p in paths {
        if !p.extension().map(|x| x.eq_ignore_ascii_case("cube")).unwrap_or(false) {
            continue;
        }
        let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let Ok(text) = std::fs::read_to_string(p) else { continue };
        let Some(l) = crate::effects::color::parse_cube(&name, &text) else { continue };
        let dest = lut_dir().join(p.file_name().unwrap());
        if dest != *p {
            let _ = std::fs::write(&dest, &text);
        }
        last = Some(l);
    }
    last
}

fn lut_control(ui: &mut Ui, app: &mut AppState, lid: Uuid, key: &str, lut: Option<LutData>) {
    let names = lut_names();
    let frame = egui::Frame::new().inner_margin(8).stroke(Stroke::new(1.0, separator())).corner_radius(CornerRadius::same(8));
    let hovering_files = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    let resp = frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(icon_text("cube")).font(icon_font(13.0)).color(if hovering_files { accent_text() } else { text_dim() }));
            if names.is_empty() && lut.is_none() {
                ui.label(egui::RichText::new(tr("No LUT loaded")).font(label_font()).color(text_faint()));
            } else {
                let items: Vec<PickerItem> = names.iter().map(|n| PickerItem::new(n, n, None)).collect();
                let mut actions = vec![
                    PickerAction { label: tr("Load .cube…"), symbol: "folder", destructive: false },
                    PickerAction { label: tr("Show LUT Folder"), symbol: "folder.badge.gearshape", destructive: false },
                ];
                let in_lib = lut.as_ref().map(|l| names.contains(&l.name)).unwrap_or(false);
                if lut.is_some() && !in_lib {
                    actions.push(PickerAction { label: tr("Save to Library"), symbol: "square.and.arrow.down", destructive: false });
                }
                if in_lib {
                    actions.push(PickerAction { label: tr("Remove from Library"), symbol: "trash", destructive: true });
                }
                let title = lut.as_ref().map(|l| l.name.clone()).unwrap_or_else(|| tr("Choose a LUT"));
                let r = picker_field(ui, "lut", &items, &lut.as_ref().map(|l| l.name.clone()).unwrap_or_default(), Some(&title), &actions, false);
                if r.opened {
                    app.begin_browse();
                }
                if let Some(n) = r.selected {
                    if let Some(l) = load_lut(&n) {
                        app.set_param(lid, key, ParamValue::Lut(l), false);
                    }
                }
                if r.closed {
                    app.end_browse();
                }
                match r.action.map(|i| actions[i].label.clone()) {
                    Some(a) if a == tr("Load .cube…") => load_cube_dialog(app, lid, key),
                    Some(a) if a == tr("Show LUT Folder") => {
                        let _ = open::that(lut_dir());
                    }
                    Some(a) if a == tr("Save to Library") => {
                        if let Some(l) = &lut {
                            let _ = std::fs::write(lut_dir().join(format!("{}.cube", l.name)), crate::effects::color::cube_text(l));
                        }
                    }
                    Some(a) if a == tr("Remove from Library") => {
                        if let Some(l) = &lut {
                            let _ = std::fs::remove_file(lut_dir().join(format!("{}.cube", l.name)));
                        }
                    }
                    _ => {}
                }
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if pill_button(ui, &tr("Load .cube…"), false).on_hover_text(tr("Load .cube files (they are saved to your LUT library)")).clicked() {
                    load_cube_dialog(app, lid, key);
                }
            });
        });
        ui.label(
            egui::RichText::new(if hovering_files { tr("Drop to load and save the LUT") } else { tr("Drop .cube files here · loaded LUTs are saved to your library") })
                .font(regular(10.5))
                .color(if hovering_files { accent_text() } else { text_faint() }),
        );
    });
    // files dropped on this control
    let rect = resp.response.rect;
    let dropped: Vec<std::path::PathBuf> = ui.ctx().input(|i| {
        let over = i.pointer.hover_pos().map(|p| rect.contains(p)).unwrap_or(false);
        if over { i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect() } else { vec![] }
    });
    if !dropped.is_empty() {
        match add_luts(&dropped) {
            Some(l) => app.set_param(lid, key, ParamValue::Lut(l), false),
            None => app.flash(&tr("Invalid .cube file")),
        }
        ui.ctx().input_mut(|i| i.raw.dropped_files.clear());
    }
}

fn load_cube_dialog(app: &mut AppState, lid: Uuid, key: &str) {
    if let Some(files) = rfd::FileDialog::new().add_filter("LUT", &["cube"]).pick_files() {
        match add_luts(&files) {
            Some(l) => app.set_param(lid, key, ParamValue::Lut(l), false),
            None => app.alert(&tr("Could not read LUT"), &tr("Invalid .cube file")),
        }
    }
}

pub fn draw_text_centered(ui: &Ui, rect: Rect, s: &str, font: egui::FontId, c: Color32) { ui.painter().text(rect.center(), Align2::CENTER_CENTER, s, font, c); }
