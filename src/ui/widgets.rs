//! Reusable controls styled like the macOS app: sections, icon/pill buttons, value sliders,
//! switches, segmented rows, colour rows and the browsable picker field with favourites.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Response, Sense, Stroke, StrokeKind, Ui};

use super::icons;
use super::theme::{self, *};
use crate::l10n::tr;
use crate::model::Rgba;

pub fn icon_text(name: &str) -> String { icons::sf(name).to_string() }

/// Paints an SF-style icon glyph centred in `rect`.
pub fn paint_icon(ui: &Ui, rect: Rect, name: &str, size: f32, color: Color32) {
    let f = if icons::is_filled(name) { icon_fill_font(size) } else { icon_font(size) };
    ui.painter().text(rect.center(), Align2::CENTER_CENTER, icons::sf(name), f, color);
}

/// Small icon button (26×24) with hover background.
pub fn icon_button(ui: &mut Ui, symbol: &str, help: &str, active: bool, size: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(26.0, 24.0), Sense::click());
    let enabled = ui.is_enabled();
    let hover = resp.hovered() && enabled;
    if hover || active {
        ui.painter().rect_filled(rect, CornerRadius::same(6), row_hover());
    }
    let c = if !enabled { with_alpha(text_dim(), 0.4) } else if active { accent_text() } else if hover { text() } else { text_dim() };
    paint_icon(ui, rect, symbol, size + 2.0, c);
    if !help.is_empty() { resp.on_hover_text(tr(help)) } else { resp }
}

fn is_icon_char(c: char) -> bool { ('\u{E000}'..='\u{F8FF}').contains(&c) }

/// Text where icon glyphs (private-use code points) are drawn with the icon font.
pub fn rich_job(s: &str, font: egui::FontId, color: Color32, wrap: Option<f32>) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let icon = egui::FontId::new(font.size + 1.0, egui::FontFamily::Name("icons".into()));
    let mut run = String::new();
    let mut run_icon = false;
    let flush = |job: &mut egui::text::LayoutJob, run: &mut String, is_icon: bool| {
        if !run.is_empty() {
            let f = if is_icon { icon.clone() } else { font.clone() };
            job.append(run, 0.0, egui::TextFormat { font_id: f, color, valign: egui::Align::Center, ..Default::default() });
            run.clear();
        }
    };
    for c in s.chars() {
        let ic = is_icon_char(c);
        if ic != run_icon {
            flush(&mut job, &mut run, run_icon);
            run_icon = ic;
        }
        run.push(c);
    }
    flush(&mut job, &mut run, run_icon);
    if let Some(w) = wrap {
        job.wrap = egui::text::TextWrapping::truncate_at_width(w);
    }
    job
}

/// Rounded "pill" button; `prominent` = yellow.
pub fn pill_button(ui: &mut Ui, label: &str, prominent: bool) -> Response {
    let font = medium(12.0);
    let galley = ui.painter().layout_job(rich_job(label, font, if prominent { Color32::BLACK } else { text() }, None));
    let size = vec2(galley.size().x + 24.0, galley.size().y + 12.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let enabled = ui.is_enabled();
    let mut bg = if prominent { ACCENT } else { field_strong() };
    if resp.is_pointer_button_down_on() {
        bg = with_alpha(bg, 0.75);
    }
    if !enabled {
        bg = with_alpha(bg, 0.4);
    }
    ui.painter().rect_filled(rect, CornerRadius::same(7), bg);
    ui.painter().galley(rect.center() - galley.size() / 2.0, galley, text());
    resp
}

/// Pill button with a leading icon.
pub fn pill_icon_button(ui: &mut Ui, symbol: &str, label: &str, prominent: bool) -> Response {
    pill_button(ui, &format!("{}  {}", icons::sf(symbol), label), prominent)
}

pub fn separator_line(ui: &mut Ui) {
    let r = ui.available_rect_before_wrap();
    let (rect, _) = ui.allocate_exact_size(vec2(r.width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::ZERO, separator());
}

/// Panel header (40 px) with optional icon.
pub fn header(ui: &mut Ui, title: &str, symbol: Option<&str>) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 40.0), Sense::hover());
    let mut x = rect.left() + 14.0;
    if let Some(s) = symbol {
        let r = Rect::from_min_size(pos2(x, rect.center().y - 8.0), vec2(16.0, 16.0));
        paint_icon(ui, r, s, 14.0, accent_text());
        x += 22.0;
    }
    ui.painter().text(pos2(x, rect.center().y), Align2::LEFT_CENTER, title, semibold(13.0), text());
    ui.painter().rect_filled(Rect::from_min_size(pos2(rect.left(), rect.bottom() - 1.0), vec2(w, 1.0)), CornerRadius::ZERO, separator());
}

/// Collapsible section with uppercase title, optional colour dot and a trailing switch.
/// Returns (section open, switch change).
pub fn panel_section(
    ui: &mut Ui, id: Id, title: &str, dot: Option<Color32>, switch: Option<bool>, add: impl FnOnce(&mut Ui),
) -> Option<bool> {
    let open_id = id.with("open");
    let mut open: bool = ui.data(|d| d.get_temp(open_id).unwrap_or(true));
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 32.0), Sense::click());
    let mut switch_change = None;
    // header
    let cy = rect.center().y;
    let chev = Rect::from_center_size(pos2(rect.left() + 18.0, cy), vec2(10.0, 10.0));
    paint_icon(ui, chev, if open { "chevron.down" } else { "chevron.right" }, 9.0, text_faint());
    let mut x = rect.left() + 28.0;
    if let Some(c) = dot {
        ui.painter().circle_filled(pos2(x + 4.5, cy), 4.5, c);
        ui.painter().circle_stroke(pos2(x + 4.5, cy), 4.5, Stroke::new(0.5, Color32::from_black_alpha(64)));
        x += 15.0;
    }
    let t = spaced_upper(&tr(title));
    ui.painter().text(pos2(x, cy), Align2::LEFT_CENTER, t, section_font(), text_dim());
    if let Some(on) = switch {
        let sr = Rect::from_center_size(pos2(rect.right() - 30.0, cy), vec2(26.0, 15.0));
        let sresp = ui.interact(sr, id.with("switch"), Sense::click());
        paint_switch(ui, sr, on, sresp.hovered());
        if sresp.clicked() {
            switch_change = Some(!on);
        }
        sresp.on_hover_text(if on { tr("Turn this part off") } else { tr("Turn this part on") });
    }
    if resp.clicked() && switch_change.is_none() {
        open = !open;
        ui.data_mut(|d| d.insert_temp(open_id, open));
    }
    resp.on_hover_text(if open { tr("Collapse section") } else { tr("Expand section") });
    if open {
        let enabled = switch.unwrap_or(true);
        egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 14, top: 0, bottom: 14 }).show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 10.0;
            ui.set_width(w - 28.0);
            if enabled {
                add(ui);
            } else {
                ui.add_enabled_ui(false, |ui| {
                    ui.set_opacity(0.38);
                    add(ui);
                });
            }
        });
    }
    let (sep, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    ui.painter().rect_filled(sep, CornerRadius::ZERO, separator());
    switch_change
}

/// Upper-case text with a little tracking (letter spacing) like the macOS section titles.
pub fn spaced_upper(s: &str) -> String { s.to_uppercase() }

pub fn paint_switch(ui: &Ui, r: Rect, on: bool, hover: bool) {
    let bg = if on { ACCENT } else if hover { field_strong() } else { with_alpha(text_faint(), 0.45) };
    ui.painter().rect_filled(r, CornerRadius::same((r.height() / 2.0) as u8), bg);
    let k = r.height() - 4.0;
    let cx = if on { r.right() - 2.0 - k / 2.0 } else { r.left() + 2.0 + k / 2.0 };
    ui.painter().circle_filled(pos2(cx, r.center().y), k / 2.0, Color32::WHITE);
}

/// Label on the left, content on the right.
pub fn labeled_row(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tr(label)).font(label_font()).color(text_dim()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), add);
    });
}

pub fn toggle_row(ui: &mut Ui, label: &str, on: bool) -> Option<bool> {
    let mut out = None;
    labeled_row(ui, label, |ui| {
        let (r, resp) = ui.allocate_exact_size(vec2(26.0, 15.0), Sense::click());
        paint_switch(ui, r, on, resp.hovered());
        if resp.clicked() {
            out = Some(!on);
        }
    });
    out
}

/// Segmented control (full width).
pub fn segmented_row(ui: &mut Ui, label: &str, options: &[(String, String)], selected: &str) -> Option<String> {
    let mut out = None;
    if !label.is_empty() {
        ui.label(egui::RichText::new(tr(label)).font(label_font()).color(text_dim()));
    }
    let w = ui.available_width();
    let base = ui.next_auto_id();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(7), field());
    let n = options.len().max(1) as f32;
    let seg_w = (w - 4.0) / n;
    for (i, (id, lab)) in options.iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + 2.0 + i as f32 * seg_w, rect.top() + 2.0), vec2(seg_w - 2.0, 22.0));
        let resp = ui.interact(r, base.with(("seg", i)), Sense::click());
        let sel = id == selected;
        if sel {
            ui.painter().rect_filled(r, CornerRadius::same(5), field_strong());
        }
        let font = if sel { semibold(11.0) } else { regular(11.0) };
        let galley = ui.painter().layout(tr(lab), font, if sel { text() } else { text_dim() }, r.width() - 4.0);
        ui.painter().galley(r.center() - galley.size() / 2.0, galley, text());
        if resp.clicked() && !sel {
            out = Some(id.clone());
        }
    }
    out
}

/// Colour well + hex. Opens a colour picker popup.
pub fn color_row(ui: &mut Ui, label: &str, color: Rgba) -> Option<Rgba> {
    let mut out = None;
    labeled_row(ui, label, |ui| {
        if let Some(c) = color_well(ui, color, ui.id().with(("color", label))) {
            out = Some(c);
        }
        ui.label(egui::RichText::new(color.to_hex()).font(value_font()).color(text_faint()));
    });
    out
}

/// A colour swatch that opens a picker (with hex field) when clicked.
pub fn color_well(ui: &mut Ui, color: Rgba, id: Id) -> Option<Rgba> {
    let mut c32 = rgba_color(&color);
    let before = c32;
    let resp = egui::widgets::color_picker::color_edit_button_srgba(ui, &mut c32, egui::widgets::color_picker::Alpha::Opaque);
    let resp = resp.on_hover_text(tr("Choose a color"));
    let _ = id;
    if c32 != before || resp.changed() {
        let [r, g, b, _] = c32.to_srgba_unmultiplied();
        let n = Rgba::new(r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0);
        if n != color {
            return Some(n);
        }
    }
    None
}

// MARK: - Value slider

pub struct SliderSpec<'a> {
    pub label: &'a str,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub step: f64,
    pub unit: &'a str,
    /// Position → value exponent (> 1 = finer control near the minimum).
    pub curve: f64,
    pub track: Option<&'a [Rgba]>,
}

impl<'a> SliderSpec<'a> {
    pub fn new(label: &'a str, value: f64, min: f64, max: f64, default: f64) -> Self {
        SliderSpec { label, value, min, max, default, step: 0.0, unit: "", curve: 1.0, track: None }
    }
    pub fn step(mut self, s: f64) -> Self { self.step = s; self }
    pub fn unit(mut self, u: &'a str) -> Self { self.unit = u; self }
    pub fn curve(mut self, c: f64) -> Self { self.curve = c; self }
    pub fn track(mut self, t: Option<&'a [Rgba]>) -> Self { self.track = t; self }

    fn span(&self) -> f64 { self.max - self.min }
    fn position(&self, v: f64) -> f64 {
        let lin = ((v - self.min) / self.span()).clamp(0.0, 1.0);
        if self.curve == 1.0 { lin } else { lin.powf(1.0 / self.curve) }
    }
    fn value_at(&self, pos: f64) -> f64 {
        let t = pos.clamp(0.0, 1.0);
        self.min + self.span() * if self.curve == 1.0 { t } else { t.powf(self.curve) }
    }
    fn clamp_value(&self, v: f64) -> f64 {
        let mut v = v.clamp(self.min, self.max);
        if self.step > 0.0 {
            v = (v / self.step).round() * self.step;
        }
        v.clamp(self.min, self.max)
    }
    fn formatted(&self) -> String {
        let span = self.span();
        let v = self.value;
        let decimals = if self.step >= 1.0 {
            0
        } else if self.step > 0.0 {
            if self.step >= 0.1 { 1 } else { 2 }
        } else if span <= 4.0 || (self.curve > 1.0 && v < 1.0 && span < 30.0) {
            2
        } else if span <= 20.0 || (self.curve > 1.0 && v < 10.0) {
            1
        } else {
            0
        };
        format!("{:.*}{}", decimals, v, self.unit)
    }
}

#[derive(Clone, Default)]
struct SliderMem {
    drag_start: Option<f64>,
    resetting: bool,
    editing: bool,
    text: String,
}

/// Compact slider: drag anywhere on the track, Alt for fine control, double-click to reset,
/// click the value to type a number. Returns the new value when it changes.
pub fn value_slider(ui: &mut Ui, id: Id, spec: SliderSpec) -> Option<f64> {
    let mut out: Option<f64> = None;
    let mut mem: SliderMem = ui.data(|d| d.get_temp::<SliderMem>(id).unwrap_or_default());
    let w = ui.available_width();
    // top row: label + value
    ui.horizontal(|ui| {
        let lr = ui.add(egui::Label::new(egui::RichText::new(tr(spec.label)).font(label_font()).color(text_dim())).sense(Sense::click()));
        if lr.double_clicked() {
            out = Some(spec.default);
        }
        lr.on_hover_text(tr("Double-click to reset"));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if mem.editing {
                let te = ui.add(egui::TextEdit::singleline(&mut mem.text).font(value_font()).desired_width(60.0).horizontal_align(egui::Align::RIGHT));
                if !te.has_focus() && !te.gained_focus() {
                    te.request_focus();
                }
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if te.lost_focus() || enter {
                    mem.editing = false;
                    let cleaned: String = mem.text.replace(',', ".").chars().filter(|c| "-0123456789.".contains(*c)).collect();
                    if let Ok(v) = cleaned.parse::<f64>() {
                        let c = spec.clamp_value(v);
                        if (c - spec.value).abs() > 1e-12 {
                            out = Some(c);
                        }
                    }
                }
            } else {
                let changed = (spec.value - spec.default).abs() > 1e-9;
                let vr = ui.add(
                    egui::Label::new(egui::RichText::new(spec.formatted()).font(value_font()).color(if changed { text() } else { text_faint() }))
                        .sense(Sense::click()),
                );
                if vr.clicked() {
                    mem.text = if spec.step >= 1.0 { format!("{:.0}", spec.value) } else { format!("{:.2}", spec.value) };
                    mem.editing = true;
                }
                vr.on_hover_text(tr("Click to type a value"));
            }
        });
    });
    // track
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 14.0), Sense::click_and_drag());
    let active = resp.hovered() || resp.dragged();
    let p = ui.painter();
    let cy = rect.center().y;
    let frac = spec.position(spec.value);
    if let Some(track) = spec.track {
        let tr_rect = Rect::from_min_max(pos2(rect.left(), cy - 2.5), pos2(rect.right(), cy + 2.5));
        paint_gradient(ui, tr_rect, track, 2.5);
        p.rect_stroke(tr_rect, CornerRadius::same(3), Stroke::new(0.5, separator()), StrokeKind::Inside);
    } else {
        let tr_rect = Rect::from_min_max(pos2(rect.left(), cy - 2.0), pos2(rect.right(), cy + 2.0));
        p.rect_filled(tr_rect, CornerRadius::same(2), track());
        let origin = spec.position(0f64.clamp(spec.min, spec.max));
        let (a, b) = (frac.min(origin), frac.max(origin));
        let fr = Rect::from_min_max(pos2(rect.left() + a as f32 * w, cy - 2.0), pos2(rect.left() + b as f32 * w, cy + 2.0));
        p.rect_filled(fr, CornerRadius::same(2), track_fill());
    }
    let kr = if active { 6.0 } else { 5.0 };
    let kx = rect.left() + frac as f32 * w;
    p.circle_filled(pos2(kx, cy + 0.5), kr + 0.8, Color32::from_black_alpha(70));
    p.circle_filled(pos2(kx, cy), kr, Color32::WHITE);
    // interaction
    if resp.double_clicked() {
        mem.resetting = true;
        mem.drag_start = None;
        out = Some(spec.default);
    } else if resp.drag_started() || (resp.is_pointer_button_down_on() && mem.drag_start.is_none() && !mem.resetting) {
        let fine = ui.input(|i| i.modifiers.alt);
        if let Some(pos) = resp.interact_pointer_pos() {
            if fine {
                mem.drag_start = Some(frac);
            } else {
                let at = ((pos.x - rect.left()) / w) as f64;
                let v = spec.clamp_value(spec.value_at(at));
                mem.drag_start = Some(spec.position(v));
                if (v - spec.value).abs() > 1e-12 {
                    out = Some(v);
                }
            }
            ui.data_mut(|d| d.insert_temp(id.with("press_x"), pos.x));
        }
    }
    if resp.dragged() && !mem.resetting {
        if let (Some(start), Some(pos)) = (mem.drag_start, resp.interact_pointer_pos()) {
            let fine = ui.input(|i| i.modifiers.alt);
            let px: f32 = ui.data(|d| d.get_temp(id.with("press_x")).unwrap_or(pos.x));
            let dx = ((pos.x - px) / w) as f64 * if fine { 0.1 } else { 1.0 };
            let v = spec.clamp_value(spec.value_at(start + dx));
            if (v - spec.value).abs() > 1e-12 {
                out = Some(v);
            }
        }
    }
    if !resp.is_pointer_button_down_on() && !resp.dragged() {
        mem.drag_start = None;
        mem.resetting = false;
    }
    resp.on_hover_text(format!("{} · {} · {}", tr("Drag to change"), alt_hint(), tr("double-click to reset")));
    ui.data_mut(|d| d.insert_temp(id, mem));
    out
}

fn alt_hint() -> String {
    if crate::shortcuts::IS_MAC { tr("⌥ for fine control") } else { tr("⌥ for fine control").replace('⌥', "Alt") }
}

/// Horizontal multi-stop gradient painted as a mesh.
pub fn paint_gradient(ui: &Ui, rect: Rect, colors: &[Rgba], _radius: f32) {
    if colors.is_empty() {
        return;
    }
    let stops: Vec<(f32, Color32)> = if colors.len() == 1 {
        vec![(0.0, rgba_color(&colors[0])), (1.0, rgba_color(&colors[0]))]
    } else {
        colors.iter().enumerate().map(|(i, c)| (i as f32 / (colors.len() - 1) as f32, rgba_color(c))).collect()
    };
    paint_gradient_stops(ui, rect, &stops);
}

pub fn paint_gradient_stops(ui: &Ui, rect: Rect, stops: &[(f32, Color32)]) {
    let mut mesh = egui::Mesh::default();
    for (t, c) in stops {
        let x = rect.left() + rect.width() * t.clamp(0.0, 1.0);
        let i = mesh.vertices.len() as u32;
        mesh.colored_vertex(pos2(x, rect.top()), *c);
        mesh.colored_vertex(pos2(x, rect.bottom()), *c);
        if i >= 2 {
            mesh.add_triangle(i - 2, i - 1, i);
            mesh.add_triangle(i - 1, i, i + 1);
        }
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

pub fn swatch_strip(ui: &Ui, rect: Rect, colors: &[Rgba]) {
    let n = colors.len().min(16).max(1);
    let w = rect.width() / n as f32;
    for (i, c) in colors.iter().take(16).enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + i as f32 * w, rect.top()), vec2(w + 0.5, rect.height()));
        ui.painter().rect_filled(r, CornerRadius::ZERO, rgba_color(c));
    }
    ui.painter().rect_stroke(rect, CornerRadius::same(3), Stroke::new(0.5, separator()), StrokeKind::Inside);
}

pub fn gradient_strip(ui: &Ui, rect: Rect, colors: &[Rgba]) {
    paint_gradient(ui, rect, colors, 3.0);
    ui.painter().rect_stroke(rect, CornerRadius::same(3), Stroke::new(0.5, separator()), StrokeKind::Inside);
}

// MARK: - Picker field

#[derive(Clone, Default)]
pub struct PickerItem {
    pub id: String,
    pub label: String,
    pub group: Option<String>,
    pub swatches: Option<Vec<Rgba>>,
    pub gradient: Option<Vec<Rgba>>,
}

impl PickerItem {
    pub fn new(id: &str, label: &str, group: Option<String>) -> PickerItem {
        PickerItem { id: id.into(), label: label.into(), group, swatches: None, gradient: None }
    }
}

pub struct PickerAction {
    pub label: String,
    pub symbol: &'static str,
    pub destructive: bool,
}

#[derive(Default)]
pub struct PickerOut {
    pub selected: Option<String>,
    pub opened: bool,
    pub closed: bool,
    pub action: Option<usize>,
}

#[derive(Clone, Default)]
struct PickerMem {
    open: bool,
    initial: String,
    current: String,
    highlight: String,
    typed: String,
    typed_at: f64,
    scroll_to: bool,
}

fn fav_list(scope: &str) -> Vec<String> {
    let pre = format!("{scope}\u{1F}");
    crate::settings::get().favorites.iter().filter_map(|f| f.strip_prefix(&pre).map(|s| s.to_string())).collect()
}
fn fav_toggle(scope: &str, id: &str) {
    let key = format!("{scope}\u{1F}{id}");
    crate::settings::update(|s| {
        if let Some(i) = s.favorites.iter().position(|f| *f == key) {
            s.favorites.remove(i);
        } else {
            s.favorites.push(key);
        }
    });
}

/// Dropdown replacement: click to open a list, browse with ↑/↓ or W/S (applies live),
/// Enter keeps, Esc reverts. While the field has focus, ↑/↓ also step through the options.
/// Items can be starred to appear first.
#[allow(clippy::too_many_arguments)]
pub fn picker_field(
    ui: &mut Ui, scope: &str, items: &[PickerItem], selected: &str, title: Option<&str>, actions: &[PickerAction], expand: bool,
) -> PickerOut {
    let id = ui.id().with(("picker", scope));
    let mut out = PickerOut::default();
    let mut mem: PickerMem = ui.data(|d| d.get_temp::<PickerMem>(id).unwrap_or_default());
    let current = items.iter().find(|i| i.id == selected);
    let label = title.map(|t| t.to_string()).or_else(|| current.map(|c| c.label.clone())).unwrap_or_else(|| selected.to_string());
    // field
    let font = label_font();
    let max_w = if expand { ui.available_width() } else { 190f32.min(ui.available_width()) };
    let preview_w = if current.map(|c| c.gradient.is_some() || c.swatches.is_some()).unwrap_or(false) { 31.0 } else { 0.0 };
    let galley = ui.painter().layout(label.clone(), font.clone(), text(), (max_w - 30.0 - preview_w).max(20.0));
    let galley = if galley.rows.len() > 1 {
        let mut job = egui::text::LayoutJob::simple_singleline(label.clone(), font.clone(), text());
        job.wrap = egui::text::TextWrapping::truncate_at_width((max_w - 30.0 - preview_w).max(20.0));
        ui.painter().layout_job(job)
    } else {
        galley
    };
    let w = if expand { max_w } else { (galley.size().x + 28.0 + preview_w).min(max_w) };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 22.0), Sense::click());
    let resp = resp.on_hover_text(tr("Click to choose · ↑↓ or W/S to browse"));
    let bg = if mem.open || resp.hovered() { field_strong() } else { field() };
    ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
    if resp.has_focus() && !mem.open {
        ui.painter().rect_stroke(rect, CornerRadius::same(6), Stroke::new(1.0, with_alpha(ACCENT, 0.7)), StrokeKind::Inside);
    }
    let mut x = rect.left() + 7.0;
    if let Some(c) = current {
        let pr = Rect::from_min_size(pos2(x, rect.center().y - 5.0), vec2(26.0, 10.0));
        if let Some(g) = &c.gradient {
            gradient_strip(ui, pr, g);
            x += 31.0;
        } else if let Some(s) = &c.swatches {
            swatch_strip(ui, pr, s);
            x += 31.0;
        }
    }
    ui.painter().galley(pos2(x, rect.center().y - galley.size().y / 2.0), galley, text());
    paint_icon(ui, Rect::from_center_size(pos2(rect.right() - 10.0, rect.center().y), vec2(10.0, 10.0)), "chevron.up.chevron.down", 9.0, text_dim());

    // navigation order: favourites first
    let favs: Vec<String> = fav_list(scope).into_iter().filter(|f| items.iter().any(|i| &i.id == f)).collect();
    let mut order: Vec<String> = favs.clone();
    order.extend(items.iter().map(|i| i.id.clone()).filter(|i| !favs.contains(i)));

    if resp.clicked() {
        resp.request_focus();
        if mem.open {
            mem.open = false;
            out.closed = true;
        } else {
            mem.open = true;
            mem.initial = selected.to_string();
            mem.current = selected.to_string();
            mem.highlight = selected.to_string();
            mem.scroll_to = true;
            out.opened = true;
        }
    }
    // stepping while focused and closed
    if !mem.open && resp.has_focus() {
        let (up, down, open_key) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp) || i.key_pressed(egui::Key::W),
                i.key_pressed(egui::Key::ArrowDown) || i.key_pressed(egui::Key::S),
                i.key_pressed(egui::Key::Space) || i.key_pressed(egui::Key::Enter),
            )
        });
        if up || down {
            let i = order.iter().position(|o| o == selected).map(|i| i as i64).unwrap_or(-1);
            let n = (i + if up { -1 } else { 1 }).clamp(0, order.len() as i64 - 1) as usize;
            if order.get(n).map(|o| o != selected).unwrap_or(false) {
                out.selected = Some(order[n].clone());
            }
        } else if open_key {
            mem.open = true;
            mem.initial = selected.to_string();
            mem.current = selected.to_string();
            mem.highlight = selected.to_string();
            mem.scroll_to = true;
            out.opened = true;
        }
    }

    if mem.open {
        let fnr = ui.ctx().cumulative_frame_nr();
        ui.ctx().data_mut(|d| d.insert_temp(Id::new("picker-open-frame"), fnr));
        // rows: (row key, item, header)
        let mut rows: Vec<(String, &PickerItem, Option<String>)> = vec![];
        for (i, f) in favs.iter().enumerate() {
            if let Some(it) = items.iter().find(|x| &x.id == f) {
                rows.push((format!("★{}", it.id), it, if i == 0 { Some(format!("★ {}", tr("Favorites"))) } else { None }));
            }
        }
        let mut last: Option<Option<String>> = None;
        for (i, it) in items.iter().enumerate() {
            let mut header = None;
            if last.as_ref() != Some(&it.group) {
                header = it.group.clone().or_else(|| if i == 0 && !favs.is_empty() { Some(tr("All")) } else { None });
                last = Some(it.group.clone());
            } else if i == 0 && !favs.is_empty() {
                header = Some(tr("All"));
            }
            rows.push((it.id.clone(), it, header));
        }
        // keyboard
        let (up, down, enter, esc, typed) = ui.input(|i| {
            let txt: String = i
                .events
                .iter()
                .filter_map(|e| if let egui::Event::Text(t) = e { Some(t.clone()) } else { None })
                .collect::<String>();
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
                txt,
            )
        });
        let mut choose = |mem: &mut PickerMem, key: &str, item_id: &str, out: &mut PickerOut| {
            mem.highlight = key.to_string();
            mem.scroll_to = true;
            if item_id != mem.current {
                mem.current = item_id.to_string();
                out.selected = Some(item_id.to_string());
            }
        };
        let mv = |mem: &mut PickerMem, d: i64, out: &mut PickerOut, choose: &mut dyn FnMut(&mut PickerMem, &str, &str, &mut PickerOut)| {
            if rows.is_empty() {
                return;
            }
            let i = rows.iter().position(|r| r.0 == mem.highlight).map(|i| i as i64).unwrap_or(-1);
            let n = (i + d).clamp(0, rows.len() as i64 - 1) as usize;
            let (k, it) = (rows[n].0.clone(), rows[n].1.id.clone());
            choose(mem, &k, &it, out);
        };
        let mut close = false;
        let lower = typed.to_lowercase();
        if up || lower == "w" {
            mv(&mut mem, -1, &mut out, &mut choose);
        } else if down || lower == "s" {
            mv(&mut mem, 1, &mut out, &mut choose);
        } else if enter {
            close = true;
        } else if esc {
            if mem.current != mem.initial {
                out.selected = Some(mem.initial.clone());
            }
            close = true;
        } else if lower.chars().count() == 1 && lower.chars().all(|c| c.is_alphanumeric()) {
            let t = ui.input(|i| i.time);
            if t - mem.typed_at > 0.9 {
                mem.typed.clear();
            }
            mem.typed += &lower;
            mem.typed_at = t;
            if let Some(r) = rows.iter().find(|r| !r.0.starts_with('★') && r.1.label.to_lowercase().starts_with(&mem.typed)) {
                let (k, it) = (r.0.clone(), r.1.id.clone());
                choose(&mut mem, &k, &it, &mut out);
            }
        }
        // popup
        let area = egui::Area::new(id.with("popup"))
            .order(egui::Order::Foreground)
            .fixed_pos(pos2(rect.left().min(ui.ctx().content_rect().right() - 276.0), rect.bottom() + 4.0))
            .constrain(true);
        let inner = area.show(ui.ctx(), |ui| {
            egui::Frame::new()
                .fill(panel_bg())
                .stroke(Stroke::new(1.0, separator()))
                .corner_radius(CornerRadius::same(10))
                .shadow(egui::Shadow { offset: [0, 8], blur: 24, spread: 0, color: Color32::from_black_alpha(110) })
                .show(ui, |ui| {
                    ui.set_width(270.0);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    egui::ScrollArea::vertical().max_height(440.0).auto_shrink([false, true]).show(ui, |ui| {
                        ui.add_space(5.0);
                        for (key, it, header) in &rows {
                            if let Some(h) = header {
                                ui.add_space(5.0);
                                ui.horizontal(|ui| {
                                    ui.add_space(10.0);
                                    ui.label(egui::RichText::new(h.to_uppercase()).font(semibold(9.5)).color(text_faint()));
                                });
                                ui.add_space(3.0);
                            }
                            let (r, rr) = ui.allocate_exact_size(vec2(270.0, 24.0), Sense::click());
                            let hl = *key == mem.highlight;
                            if hl || rr.hovered() {
                                let br = r.shrink2(vec2(4.0, 0.0));
                                ui.painter().rect_filled(br, CornerRadius::same(5), if hl { row_selected() } else { row_hover() });
                            }
                            if hl && mem.scroll_to {
                                ui.scroll_to_rect(r, None);
                            }
                            let mut x = r.left() + 8.0;
                            if it.id == mem.current {
                                paint_icon(ui, Rect::from_center_size(pos2(x + 5.0, r.center().y), vec2(10.0, 10.0)), "checkmark", 10.0, accent_text());
                            }
                            x += 17.0;
                            let pr = Rect::from_min_size(pos2(x, r.center().y - 6.0), vec2(46.0, 12.0));
                            if let Some(g) = &it.gradient {
                                gradient_strip(ui, pr, g);
                                x += 53.0;
                            } else if let Some(s) = &it.swatches {
                                swatch_strip(ui, pr, s);
                                x += 53.0;
                            }
                            let mut job = egui::text::LayoutJob::simple_singleline(it.label.clone(), regular(12.0), text());
                            job.wrap = egui::text::TextWrapping::truncate_at_width(r.right() - 28.0 - x);
                            let gal = ui.painter().layout_job(job);
                            ui.painter().galley(pos2(x, r.center().y - gal.size().y / 2.0), gal, text());
                            let star_r = Rect::from_center_size(pos2(r.right() - 16.0, r.center().y), vec2(18.0, 18.0));
                            let is_fav = favs.contains(&it.id);
                            let star_resp = ui.interact(star_r, id.with(("star", key)), Sense::click());
                            let sc = if is_fav { ACCENT } else { with_alpha(text_faint(), if rr.hovered() || star_resp.hovered() { 1.0 } else { 0.25 }) };
                            paint_icon(ui, star_r, if is_fav { "star.fill" } else { "star" }, 11.0, sc);
                            if star_resp.clicked() {
                                fav_toggle(scope, &it.id);
                            } else if rr.clicked() {
                                let (k, iid) = (key.clone(), it.id.clone());
                                choose(&mut mem, &k, &iid, &mut out);
                                close = true;
                            }
                        }
                        ui.add_space(5.0);
                    });
                    mem.scroll_to = false;
                    if !actions.is_empty() {
                        separator_line(ui);
                        ui.add_space(4.0);
                        for (i, a) in actions.iter().enumerate() {
                            let (r, rr) = ui.allocate_exact_size(vec2(270.0, 24.0), Sense::click());
                            if rr.hovered() {
                                ui.painter().rect_filled(r.shrink2(vec2(4.0, 0.0)), CornerRadius::same(5), row_hover());
                            }
                            let c = if a.destructive { Color32::from_rgb(235, 80, 70) } else { text() };
                            paint_icon(ui, Rect::from_center_size(pos2(r.left() + 17.0, r.center().y), vec2(12.0, 12.0)), a.symbol, 11.0, c);
                            ui.painter().text(pos2(r.left() + 30.0, r.center().y), Align2::LEFT_CENTER, &a.label, regular(12.0), c);
                            if rr.clicked() {
                                out.action = Some(i);
                                close = true;
                            }
                        }
                        ui.add_space(4.0);
                    }
                    separator_line(ui);
                    ui.add_space(5.0);
                    let hint = if crate::shortcuts::IS_MAC {
                        tr("↑↓ / W S browse · ↩ keep · ⎋ revert · ★ favorite")
                    } else {
                        tr("↑↓ / W S browse · ↩ keep · ⎋ revert · ★ favorite").replace('↩', "Enter").replace('⎋', "Esc")
                    };
                    ui.vertical_centered(|ui| ui.label(egui::RichText::new(hint).font(regular(9.5)).color(text_faint())));
                    ui.add_space(5.0);
                });
        });
        // click outside closes (keeps the current choice)
        let popup_rect = inner.response.rect;
        let clicked_outside = ui.input(|i| {
            i.pointer.any_pressed() && i.pointer.interact_pos().map(|p| !popup_rect.contains(p) && !rect.contains(p)).unwrap_or(false)
        });
        if clicked_outside {
            close = true;
        }
        if close {
            mem.open = false;
            out.closed = true;
            resp.request_focus();
        }
    }
    ui.data_mut(|d| d.insert_temp(id, mem));
    out
}

/// Whether any picker popup is open (so global single-key shortcuts don't fire).
pub fn search_field(ui: &mut Ui, id: Id, placeholder: &str, text: &mut String, focus: bool) -> Response {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(8), field());
    paint_icon(ui, Rect::from_center_size(pos2(rect.left() + 16.0, rect.center().y), vec2(14.0, 14.0)), "magnifyingglass", 13.0, text_faint());
    let te_rect = Rect::from_min_max(pos2(rect.left() + 30.0, rect.top() + 7.0), pos2(rect.right() - 28.0, rect.bottom() - 7.0));
    let resp = ui
        .scope_builder(egui::UiBuilder::new().max_rect(te_rect), |ui| {
            ui.add(egui::TextEdit::singleline(text).id(id).frame(egui::Frame::NONE).hint_text(placeholder).font(regular(13.0)).desired_width(te_rect.width()))
        })
        .inner;
    if focus {
        resp.request_focus();
    }
    if !text.is_empty() {
        let xr = Rect::from_center_size(pos2(rect.right() - 15.0, rect.center().y), vec2(16.0, 16.0));
        let xresp = ui.interact(xr, id.with("clear"), Sense::click());
        paint_icon(ui, xr, "xmark.circle.fill", 13.0, text_faint());
        if xresp.clicked() {
            text.clear();
        }
        xresp.on_hover_text(tr("Clear search"));
    }
    resp
}

/// Grid / list toggle used by both browser panels.
pub fn view_mode_toggle(ui: &mut Ui, id: Id, list_mode: bool) -> Option<bool> {
    let (rect, _) = ui.allocate_exact_size(vec2(62.0, 30.0), Sense::hover());
    ui.painter().rect_filled(rect, CornerRadius::same(7), field());
    let mut out = None;
    for (i, (sym, list, help)) in [("square.grid.2x2", false, "Grid"), ("list.bullet", true, "List")].iter().enumerate() {
        let r = Rect::from_min_size(pos2(rect.left() + 2.0 + i as f32 * 30.0, rect.top() + 2.0), vec2(28.0, 26.0));
        let resp = ui.interact(r, id.with(i), Sense::click());
        let sel = list_mode == *list;
        if sel {
            ui.painter().rect_filled(r, CornerRadius::same(5), field_strong());
        }
        paint_icon(ui, r, sym, 12.0, if sel { text() } else { text_dim() });
        if resp.clicked() && !sel {
            out = Some(*list);
        }
        resp.on_hover_text(tr(help));
    }
    out
}

/// Thin footer line with a hint.
pub fn browser_footer(ui: &mut Ui, s: &str) {
    separator_line(ui);
    ui.add_space(5.0);
    ui.vertical_centered(|ui| ui.label(egui::RichText::new(s).font(regular(10.0)).color(text_faint())));
    ui.add_space(5.0);
}

/// Mac symbols in hints become words on Windows.
/// "Label (shortcut)" using the action's current (possibly customised) shortcut.
pub fn action_hint(label: &str, a: crate::shortcuts::Action) -> String {
    let k = crate::shortcuts::display(a);
    if k.is_empty() { crate::l10n::tr(label) } else { format!("{} ({k})", crate::l10n::tr(label)) }
}

pub fn keys_hint(s: &str) -> String {
    if crate::shortcuts::IS_MAC {
        s.to_string()
    } else {
        s.replace('⌘', "Ctrl+").replace('⇧', "Shift+").replace('⌥', "Alt+").replace('↩', "Enter").replace('⎋', "Esc").replace('⌫', "Backspace")
    }
}

pub fn theme_dark() -> bool { theme::is_dark() }
