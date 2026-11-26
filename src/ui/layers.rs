//! Layers panel: Add Effect / Presets buttons, the layer tree with groups, multi-selection,
//! drag to reorder (above / below / inside groups), rename, visibility, mask and context menu.

use egui::{pos2, vec2, Align2, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use uuid::Uuid;

use super::theme::*;
use super::widgets::*;
use crate::app::{AppState, Dialog, DropPosition, PromptKind};
use crate::l10n::{tr, trf};
use crate::model::*;
use crate::shortcuts::{display, Action};

const ROW_H: f32 = 32.0;
const ROW_GAP: f32 = 1.0;

#[derive(Clone, Default)]
struct DragState {
    ids: Vec<Uuid>,
    target: Option<Uuid>,
    pos: Option<DropPosition>,
    to_bottom: bool,
}

pub fn layers_panel(ui: &mut Ui, app: &mut AppState) {
    let has = app.has_image();
    // header
    egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 10, top: 10, bottom: 6 }).show(ui, |ui| {
        ui.add_enabled_ui(has, |ui| {
            if !has {
                ui.set_opacity(0.4);
            }
            let w = ui.available_width();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let bw = (w - 6.0) / 2.0;
                let eon = app.show_effect_browser;
                let (r1, add) = ui.allocate_exact_size(vec2(bw, 28.0), Sense::click());
                ui.painter().rect_filled(r1, CornerRadius::same(7), if eon { with_alpha(ACCENT, 0.75) } else { ACCENT });
                if eon {
                    ui.painter().rect_stroke(r1, CornerRadius::same(7), egui::Stroke::new(1.5, text()), egui::StrokeKind::Inside);
                }
                let g = ui.painter().layout_job(rich_job(&format!("{}  {}", icon_text("plus"), tr("Add Effect")), semibold(12.0), egui::Color32::BLACK, None));
                ui.painter().galley(r1.center() - g.size() / 2.0, g, egui::Color32::BLACK);
                if add.clicked() {
                    app.perform(Action::AddEffect, &ui.ctx().clone());
                }
                add.on_hover_text(format!("{}  {}", tr("Add Effect…"), display(Action::AddEffect)));
                let on = app.show_preset_browser && !app.show_effect_browser;
                let (r2, pr) = ui.allocate_exact_size(vec2(bw, 28.0), Sense::click());
                ui.painter().rect_filled(r2, CornerRadius::same(7), if on { ACCENT } else { field_strong() });
                let g = ui.painter().layout_job(rich_job(&format!("{}  {}", icon_text("wand.and.stars"), tr("Presets")), semibold(12.0), if on { egui::Color32::BLACK } else { text() }, None));
                ui.painter().galley(r2.center() - g.size() / 2.0, g, text());
                if pr.clicked() {
                    app.set_show_presets(!on);
                }
                pr.on_hover_text(format!("{}  {}", tr("Browse presets"), display(Action::ShowPresets)));
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.label(egui::RichText::new(spaced_upper(&tr("Layers"))).font(section_font()).color(text_dim()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let empty = app.layers().is_empty();
                    if ui.add_enabled_ui(!empty, |ui| icon_button(ui, "square.and.arrow.down.on.square", "Save all layers as a preset", false, 13.0)).inner.clicked() {
                        app.save_stack_as_preset();
                    }
                    if icon_button(ui, "plus.square.on.square", &action_hint("Duplicate", crate::shortcuts::Action::Duplicate), false, 13.0).clicked() {
                        app.duplicate_selected();
                    }
                    if icon_button(ui, "folder.badge.plus", &action_hint("Group selected layers", crate::shortcuts::Action::Group), false, 13.0).clicked() {
                        app.group_selected();
                    }
                });
            });
        });
    });
    if has {
        egui::Frame::new().inner_margin(egui::Margin { left: 10, right: 10, top: 0, bottom: 8 }).show(ui, |ui| layer_props_bar(ui, app));
    }
    separator_line(ui);
    if app.is_multi_selection() {
        selection_bar(ui, app);
    }
    // list
    let rows = app.layers().flattened();
    let drag_id = Id::new("layer-drag");
    let mut drag: Option<DragState> = ui.data(|d| d.get_temp(drag_id));
    let mut drag_end = false;
    egui::ScrollArea::vertical().id_salt("layers-scroll").auto_shrink([false, false]).show(ui, |ui| {
        ui.add_space(6.0);
        let top = ui.cursor().top();
        ui.spacing_mut().item_spacing.y = ROW_GAP;
        for (node, depth) in &rows {
            let dim = drag.as_ref().map(|d| d.ids.contains(&node.id)).unwrap_or(false);
            let ind = drag.as_ref().filter(|d| !d.to_bottom && d.target == Some(node.id)).and_then(|d| d.pos);
            let resp = layer_row(ui, app, node, *depth, dim, ind);
            // drag to reorder
            if resp.drag_started() {
                if !app.selected_ids.contains(&node.id) {
                    app.set_selected(Some(node.id));
                }
                let roots = app.selection_roots();
                drag = Some(DragState { ids: if roots.is_empty() { vec![node.id] } else { roots }, ..Default::default() });
            }
            if resp.dragged() {
                if let (Some(d), Some(p)) = (drag.as_mut(), ui.ctx().pointer_interact_pos()) {
                    update_drag(d, p.y - top, &rows, app);
                }
            }
            if resp.drag_stopped() {
                drag_end = true;
            }
        }
        // empty space below the list: click to deselect
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 60f32.max(ui.available_height())), Sense::click());
        if resp.clicked() {
            app.set_selected(None);
        }
        if drag.as_ref().map(|d| d.to_bottom).unwrap_or(false) {
            let lr = Rect::from_min_size(pos2(r.left() + 10.0, r.top()), vec2(r.width() - 20.0, 2.0));
            ui.painter().rect_filled(lr, CornerRadius::same(1), ACCENT);
        }
        if rows.is_empty() {
            let c = r.center_top() + vec2(0.0, 40.0);
            paint_icon(ui, Rect::from_center_size(c, vec2(30.0, 30.0)), "square.3.layers.3d", 26.0, text_faint());
            let t = if app.has_image() { tr("No effects yet") } else { tr("Open an image to start") };
            ui.painter().text(c + vec2(0.0, 30.0), Align2::CENTER_CENTER, t, medium(12.0), text_dim());
            if app.has_image() {
                let g = ui.painter().layout(
                    tr("Add an effect or apply a preset.\nThe top layer is applied last."),
                    regular(11.0),
                    text_faint(),
                    r.width() - 32.0,
                );
                let gx = c.x - g.size().x / 2.0;
                let mut job_g = g;
                // centre each line
                let _ = &mut job_g;
                ui.painter().galley(pos2(gx, c.y + 46.0), job_g, text_faint());
            }
        }
    });
    if drag_end {
        if let Some(d) = drag.take() {
            if d.to_bottom {
                app.move_to_root_bottom(&d.ids);
            } else if let (Some(t), Some(p)) = (d.target, d.pos) {
                app.move_layers(&d.ids, t, p);
            }
        }
    }
    ui.data_mut(|m| match drag {
        Some(d) => {
            m.insert_temp(drag_id, d);
        }
        None => m.remove::<DragState>(drag_id),
    });
}

fn update_drag(d: &mut DragState, y: f32, rows: &[(LayerNode, usize)], app: &AppState) {
    let f = y / (ROW_H + ROW_GAP);
    let idx = f.floor() as i64;
    if idx >= rows.len() as i64 {
        d.to_bottom = true;
        d.target = None;
        return;
    }
    let i = idx.max(0) as usize;
    let node = &rows[i].0;
    let frac = if idx < 0 { 0.0 } else { f - idx as f32 };
    d.to_bottom = false;
    let layers = app.layers();
    let invalid = d.ids.contains(&node.id) || d.ids.iter().any(|x| layers.contains_inside(node.id, *x));
    if invalid {
        d.target = None;
        return;
    }
    d.target = Some(node.id);
    d.pos = Some(if node.is_group {
        if frac < 0.3 {
            DropPosition::Above
        } else if frac > 0.7 && !(node.expanded && !node.children.is_empty()) {
            DropPosition::Below
        } else {
            DropPosition::Inside
        }
    } else if frac < 0.5 {
        DropPosition::Above
    } else {
        DropPosition::Below
    });
}

fn selection_bar(ui: &mut Ui, app: &mut AppState) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::ZERO, with_alpha(row_selected(), 0.6));
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect.shrink2(vec2(10.0, 5.0))), |ui| {
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.label(egui::RichText::new(trf("%d selected", &[&app.selected_ids.len()])).font(semibold(11.5)).color(accent_text()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, "xmark", "Clear selection", false, 12.0).clicked() {
                    app.set_selected(None);
                }
                if icon_button(ui, "trash", &action_hint("Delete", crate::shortcuts::Action::Delete), false, 12.0).clicked() {
                    app.delete_selected();
                }
                if icon_button(ui, "eye.slash", "Show / Hide (H)", false, 12.0).clicked() {
                    app.toggle_selected_visibility();
                }
                if icon_button(ui, "plus.square.on.square", &action_hint("Duplicate", crate::shortcuts::Action::Duplicate), false, 12.0).clicked() {
                    app.duplicate_selected();
                }
                if icon_button(ui, "folder.badge.plus", &action_hint("Group selected layers", crate::shortcuts::Action::Group), false, 12.0).clicked() {
                    app.group_selected();
                }
            });
        });
    });
    separator_line(ui);
}

fn layer_row(ui: &mut Ui, app: &mut AppState, node: &LayerNode, depth: usize, dimmed: bool, ind: Option<DropPosition>) -> egui::Response {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, ROW_H), Sense::click_and_drag());
    let selected = app.selected_ids.contains(&node.id);
    let primary = app.selected_id == Some(node.id);
    let hover = resp.hovered();
    let rename_id = Id::new(("rename", node.id));
    let mut renaming: Option<String> = ui.data(|d| d.get_temp(rename_id));
    let bg = rect.shrink2(vec2(6.0, 0.0));
    if selected {
        ui.painter().rect_filled(bg, CornerRadius::same(7), if primary { row_selected() } else { with_alpha(row_selected(), 0.6) });
    } else if hover {
        ui.painter().rect_filled(bg, CornerRadius::same(7), row_hover());
    }
    let opacity = if dimmed { 0.45 } else { 1.0 };
    let cy = rect.center().y;
    let mut x = rect.left() + 10.0 + depth as f32 * 14.0;
    // chevron
    if node.is_group {
        let cr = Rect::from_center_size(pos2(x + 8.0, cy), vec2(16.0, 24.0));
        let cresp = ui.interact(cr, Id::new(("chev", node.id)), Sense::click());
        paint_icon(ui, cr, if node.expanded { "chevron.down" } else { "chevron.right" }, 10.0, with_alpha(text_dim(), opacity));
        if cresp.clicked() {
            let id = node.id;
            app.update_layer_silently(id, |n| n.expanded = !n.expanded);
        }
        cresp.on_hover_text(if node.expanded { tr("Collapse group") } else { tr("Expand group") });
    }
    x += 22.0;
    let sym = if node.is_group {
        if node.expanded { "folder" } else { "folder.fill" }
    } else {
        crate::effects::effect(&node.effect_id).map(|e| e.symbol).unwrap_or("square")
    };
    paint_icon(ui, Rect::from_center_size(pos2(x + 9.0, cy), vec2(18.0, 18.0)), sym, 13.0, with_alpha(if selected { accent_text() } else { text_dim() }, opacity));
    x += 24.0;
    // right side controls
    let mut right = rect.right() - 10.0;
    let eye_r = Rect::from_center_size(pos2(right - 10.0, cy), vec2(20.0, 24.0));
    right -= 20.0;
    let trash_r = Rect::from_center_size(pos2(right - 11.0, cy), vec2(18.0, 24.0));
    right -= 24.0;
    // mask slot: always there, next to the trash and the eye (Photoshop-style)
    let mask_r = Rect::from_center_size(pos2(right - 9.0, cy), vec2(18.0, 24.0));
    right -= 22.0;
    let eye = ui.interact(eye_r, Id::new(("eye", node.id)), Sense::click());
    paint_icon(
        ui,
        eye_r,
        if node.enabled { "eye" } else { "eye.slash" },
        13.0,
        with_alpha(if node.enabled { if hover { text() } else { text_dim() } } else { text_faint() }, opacity),
    );
    if eye.clicked() {
        if app.is_multi_selection() && app.selected_ids.contains(&node.id) {
            app.toggle_selected_visibility();
        } else {
            app.toggle_visibility(node.id);
        }
    }
    eye.on_hover_text(format!("{} ({})", tr("Show / Hide"), display(Action::ToggleVisibility)));
    let trash = ui.interact(trash_r, Id::new(("trash", node.id)), Sense::click());
    paint_icon(ui, trash_r, "trash", 12.0, with_alpha(if hover { text_dim() } else { with_alpha(text_faint(), 0.7) }, opacity));
    if trash.clicked() {
        if !app.selected_ids.contains(&node.id) {
            app.set_selected(Some(node.id));
        }
        app.delete_selected();
    }
    trash.on_hover_text(format!("{} ({})", tr(if app.selected_ids.contains(&node.id) && app.is_multi_selection() { "Delete selected layers" } else { "Delete" }), display(Action::Delete)));
    {
        let linked = node.mask.and_then(|r| app.state().mask(r.id).map(|m| (m.name.clone(), r.invert)));
        let mresp = ui.interact(mask_r, Id::new(("maskslot", node.id)), Sense::click());
        let col = match &linked {
            Some(_) => with_alpha(accent_text(), 0.95 * opacity),
            None if hover || mresp.hovered() => with_alpha(text_dim(), opacity),
            None => with_alpha(with_alpha(text_faint(), 0.7), opacity),
        };
        paint_icon(ui, mask_r, if linked.as_ref().map(|l| l.1).unwrap_or(false) { "circle.lefthalf.filled" } else { "circle.dashed" }, 12.0, col);
        let tip = match &linked {
            Some((n, true)) => crate::l10n::trf("Mask: %@ (inverted)", &[n]),
            Some((n, false)) => crate::l10n::trf("Mask: %@", &[n]),
            None => tr("Add a mask to this layer"),
        };
        let mresp = mresp.on_hover_text(tip);
        let lid = node.id;
        egui::Popup::menu(&mresp).id(Id::new(("maskmenu", lid))).show(|ui| layer_mask_menu(ui, app, lid));
    }
    // badges: opacity, blend
    if node.opacity < 0.999 {
        let t = format!("{}%", (node.opacity * 100.0).round() as i64);
        let g = ui.painter().layout_no_wrap(t, value_font_small(), text_faint());
        right -= g.size().x + 4.0;
        ui.painter().galley(pos2(right, cy - g.size().y / 2.0), g, text_faint());
    }
    if node.blend != BlendMode::Normal {
        let g = ui.painter().layout_no_wrap(tr(node.blend.label()), regular(9.5), text_faint());
        right -= g.size().x + 6.0;
        ui.painter().galley(pos2(right, cy - g.size().y / 2.0), g, text_faint());
    }
    // name / rename field
    let name_rect = Rect::from_min_max(pos2(x, rect.top() + 6.0), pos2(right - 4.0, rect.bottom() - 6.0));
    if let Some(mut text) = renaming.take() {
        let r = ui
            .scope_builder(egui::UiBuilder::new().max_rect(name_rect), |ui| {
                ui.add(egui::TextEdit::singleline(&mut text).id(rename_id.with("te")).font(regular(12.5)).desired_width(name_rect.width()).frame(egui::Frame::NONE))
            })
            .inner;
        if !r.has_focus() && !r.lost_focus() {
            r.request_focus();
        }
        let done = r.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Enter));
        if done {
            let n = text.trim().to_string();
            if !n.is_empty() && n != node.name {
                app.update_layer(node.id, None, |l| l.name = n.clone());
            }
            ui.data_mut(|d| d.remove::<String>(rename_id));
        } else {
            ui.data_mut(|d| d.insert_temp(rename_id, text));
        }
    } else {
        let mut job = egui::text::LayoutJob::simple_singleline(
            node.name.clone(),
            if selected { medium(12.5) } else { regular(12.5) },
            with_alpha(if node.enabled { text() } else { text_faint() }, opacity),
        );
        job.wrap = egui::text::TextWrapping::truncate_at_width(name_rect.width());
        let g = ui.painter().layout_job(job);
        ui.painter().galley(pos2(name_rect.left(), cy - g.size().y / 2.0), g, text());
    }
    // drop indicators
    let lx = rect.left() + depth as f32 * 14.0 + 10.0;
    match ind {
        Some(DropPosition::Above) => {
            ui.painter().rect_filled(Rect::from_min_max(pos2(lx, rect.top() - 1.0), pos2(rect.right() - 8.0, rect.top() + 1.0)), CornerRadius::same(1), ACCENT);
        }
        Some(DropPosition::Below) => {
            ui.painter().rect_filled(Rect::from_min_max(pos2(lx, rect.bottom() - 1.0), pos2(rect.right() - 8.0, rect.bottom() + 1.0)), CornerRadius::same(1), ACCENT);
        }
        Some(DropPosition::Inside) => {
            ui.painter().rect_stroke(bg, CornerRadius::same(7), Stroke::new(1.5, ACCENT), StrokeKind::Inside);
        }
        None => {}
    }
    // click / double-click
    if resp.double_clicked() {
        ui.data_mut(|d| d.insert_temp(rename_id, node.name.clone()));
    } else if resp.clicked() {
        let m = ui.input(|i| i.modifiers);
        app.click(node.id, m);
    }
    let resp = resp.on_hover_text(tr("Click to select · ⇧/⌘-click for several · drag to reorder · double-click to rename"));
    // context menu
    let nid = node.id;
    let is_group = node.is_group;
    let enabled = node.enabled;
    let name = node.name.clone();
    let children = node.children.clone();
    let node_clone = node.clone();
    resp.context_menu(|ui| {
        let select = |app: &mut AppState| {
            if !app.selected_ids.contains(&nid) {
                app.set_selected(Some(nid));
            }
        };
        if ui.button(tr("Rename")).clicked() {
            ui.data_mut(|d| d.insert_temp(rename_id, name.clone()));
            ui.close();
        }
        if ui.button(tr("Duplicate")).clicked() {
            select(app);
            app.duplicate_selected();
            ui.close();
        }
        if ui.button(if enabled { tr("Hide") } else { tr("Show") }).clicked() {
            select(app);
            app.toggle_selected_visibility();
            ui.close();
        }
        ui.separator();
        if ui.button(tr("Group Layer")).clicked() {
            select(app);
            app.group_selected();
            ui.close();
        }
        if is_group && ui.button(tr("Ungroup")).clicked() {
            app.set_selected(Some(nid));
            app.ungroup_selected();
            ui.close();
        }
        if ui.button(tr("Save as Preset…")).clicked() {
            let roots: Vec<LayerNode> = if app.selected_ids.contains(&nid) && app.is_multi_selection() {
                app.selection_roots().iter().filter_map(|id| app.layers().find(*id).cloned()).collect()
            } else if is_group {
                children.clone()
            } else {
                vec![node_clone.clone()]
            };
            app.dialog = Some(Dialog::Prompt { title: tr("Save as Preset"), label: tr("Name"), text: name.clone(), kind: PromptKind::SaveLayersPreset(roots) });
            ui.close();
        }
        ui.separator();
        if ui.button(egui::RichText::new(tr("Delete")).color(egui::Color32::from_rgb(235, 80, 70))).clicked() {
            select(app);
            app.delete_selected();
            ui.close();
        }
    });
    resp
}

/// Blend mode + opacity of the selected layer(s), Photoshop-style, on top of the list.
fn layer_props_bar(ui: &mut Ui, app: &mut AppState) {
    let nodes: Vec<LayerNode> = app.selection_roots().iter().filter_map(|id| app.layers().find(*id).cloned()).collect();
    let any = !nodes.is_empty();
    let blends: std::collections::HashSet<BlendMode> = nodes.iter().map(|n| n.blend).collect();
    let blend = if blends.len() == 1 { nodes[0].blend.id() } else { String::new() };
    let opacity = if any { nodes.iter().map(|n| n.opacity).sum::<f64>() / nodes.len() as f64 } else { 1.0 };
    let w = ui.available_width();
    ui.add_enabled_ui(any, |ui| {
        if !any {
            ui.set_opacity(0.45);
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            let bw = ((w - 6.0) * 0.52).floor();
            ui.allocate_ui_with_layout(vec2(bw, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                let items = super::inspector::blend_items();
                let title = if !any { Some(tr("Normal")) } else if blend.is_empty() { Some("—".to_string()) } else { None };
                let r = picker_field(ui, "layers-blend", &items, &blend, title.as_deref(), &[], true);
                if r.opened {
                    app.begin_browse();
                }
                if let Some(v) = r.selected {
                    if let Some(b) = BlendMode::from_id(&v) {
                        app.update_selected(None, |n| n.blend = b);
                    }
                }
                if r.closed {
                    app.end_browse();
                }
            })
            .response
            .on_hover_text(tr("Blend mode of the selected layer"));
            let ow = ui.available_width().min(w - bw - 6.0);
            if let Some(v) = opacity_field(ui, ow, opacity) {
                app.update_selected(Some("layers-bar-opacity".into()), |n| n.opacity = v);
            }
        });
    });
}

/// Compact opacity bar: click or drag to set, double-click for 100%, scroll to nudge.
fn opacity_field(ui: &mut Ui, w: f32, v: f64) -> Option<f64> {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 22.0), Sense::click_and_drag());
    let mut out = None;
    let p = ui.painter();
    p.rect_filled(rect, CornerRadius::same(6), if resp.hovered() || resp.dragged() { field_strong() } else { field() });
    let fill = Rect::from_min_size(rect.min, vec2(rect.width() * v.clamp(0.0, 1.0) as f32, rect.height()));
    p.rect_filled(fill, CornerRadius::same(6), with_alpha(ACCENT, if resp.dragged() { 0.38 } else { 0.24 }));
    p.text(pos2(rect.left() + 8.0, rect.center().y), Align2::LEFT_CENTER, tr("Opacity"), label_font(), text_dim());
    p.text(pos2(rect.right() - 8.0, rect.center().y), Align2::RIGHT_CENTER, format!("{}%", (v * 100.0).round() as i64), value_font_small(), text());
    if resp.double_clicked() {
        out = Some(1.0);
    } else if resp.dragged() || resp.drag_started() || resp.clicked() {
        if let Some(pp) = resp.interact_pointer_pos() {
            let nv = (((pp.x - rect.left()) / rect.width()).clamp(0.0, 1.0) as f64 * 100.0).round() / 100.0;
            if (nv - v).abs() > 1e-6 {
                out = Some(nv);
            }
        }
    }
    if resp.hovered() {
        let d = ui.input(|i| i.smooth_scroll_delta.y + i.smooth_scroll_delta.x);
        if d.abs() > 0.5 {
            ui.ctx().input_mut(|i| i.smooth_scroll_delta = egui::Vec2::ZERO);
            out = Some(((v * 100.0 + d.signum() as f64).round() / 100.0).clamp(0.0, 1.0));
        }
    }
    resp.on_hover_text(tr("Opacity · drag or click to set · double-click for 100%"));
    out
}

/// Menu of a layer's mask slot: pick a mask of the image, invert, create or edit.
fn layer_mask_menu(ui: &mut Ui, app: &mut AppState, lid: Uuid) {
    ui.set_min_width(200.0);
    let st = app.state();
    let cur = app.layers().find(lid).and_then(|n| n.mask).filter(|r| st.mask(r.id).is_some());
    if ui.add(egui::Button::selectable(cur.is_none(), tr("None (whole image)"))).clicked() {
        app.set_layer_mask(lid, None);
        ui.close();
    }
    if !st.masks.is_empty() {
        ui.separator();
        ui.label(egui::RichText::new(spaced_upper(&tr("Masks"))).font(section_font()).color(text_faint()));
        for m in &st.masks {
            let on = cur.map(|c| c.id == m.id).unwrap_or(false);
            if ui.add(egui::Button::selectable(on, &m.name)).clicked() {
                app.set_layer_mask(lid, Some(MaskRef { id: m.id, invert: cur.map(|c| c.invert).unwrap_or(false) }));
                ui.close();
            }
        }
    }
    ui.separator();
    if let Some(c) = cur {
        if ui.add(egui::Button::selectable(c.invert, tr("Invert Mask"))).clicked() {
            app.set_layer_mask(lid, Some(MaskRef { id: c.id, invert: !c.invert }));
            ui.close();
        }
    }
    if ui.button(tr("New Mask…")).clicked() {
        app.set_selected(Some(lid));
        app.mask_new_for = Some(lid);
        app.set_tool(crate::app::Tool::Masks);
        ui.close();
    }
    if ui.button(tr("Edit Masks…")).clicked() {
        app.selected_mask = cur.map(|c| c.id).or(app.selected_mask);
        app.set_tool(crate::app::Tool::Masks);
        ui.close();
    }
}

fn value_font_small() -> egui::FontId { egui::FontId::new(10.0, egui::FontFamily::Name("digits".into())) }
