//! Colours, fonts and icons (same values as the macOS app's Theme).

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use egui::{Color32, FontData, FontDefinitions, FontFamily, FontId};

use crate::overlay;

static DARK: AtomicBool = AtomicBool::new(true);

pub fn set_dark(d: bool) { DARK.store(d, Ordering::Relaxed); }
pub fn is_dark() -> bool { DARK.load(Ordering::Relaxed) }

fn g(v: f32, a: f32) -> Color32 {
    let c = (v * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(c, c, c, (a * 255.0).round() as u8)
}
fn dyn_(dark: Color32, light: Color32) -> Color32 { if is_dark() { dark } else { light } }
fn rgb(r: f32, gg: f32, b: f32, a: f32) -> Color32 {
    Color32::from_rgba_unmultiplied((r * 255.0) as u8, (gg * 255.0) as u8, (b * 255.0) as u8, (a * 255.0) as u8)
}

/// #FFBB00
pub const ACCENT: Color32 = Color32::from_rgb(255, 187, 0);
pub fn canvas_bg() -> Color32 { dyn_(g(0.085, 1.0), g(0.86, 1.0)) }
pub fn panel_bg() -> Color32 { dyn_(g(0.135, 1.0), g(0.955, 1.0)) }
pub fn panel_bg2() -> Color32 { dyn_(g(0.165, 1.0), g(0.925, 1.0)) }
pub fn row_hover() -> Color32 { dyn_(g(1.0, 0.05), g(0.0, 0.045)) }
pub fn row_selected() -> Color32 { dyn_(rgb(1.0, 0.733, 0.0, 0.16), rgb(1.0, 0.62, 0.0, 0.18)) }
pub fn field() -> Color32 { dyn_(g(1.0, 0.06), g(0.0, 0.05)) }
pub fn field_strong() -> Color32 { dyn_(g(1.0, 0.1), g(0.0, 0.09)) }
pub fn separator() -> Color32 { dyn_(g(1.0, 0.08), g(0.0, 0.1)) }
pub fn text() -> Color32 { dyn_(g(0.92, 1.0), g(0.1, 1.0)) }
pub fn text_dim() -> Color32 { dyn_(g(0.6, 1.0), g(0.38, 1.0)) }
pub fn text_faint() -> Color32 { dyn_(g(0.42, 1.0), g(0.55, 1.0)) }
pub fn track() -> Color32 { dyn_(g(1.0, 0.09), g(0.0, 0.1)) }
pub fn track_fill() -> Color32 { dyn_(rgb(1.0, 0.733, 0.0, 0.55), rgb(0.95, 0.62, 0.0, 0.7)) }
pub fn accent_text() -> Color32 { dyn_(rgb(1.0, 0.76, 0.1, 1.0), rgb(0.75, 0.47, 0.0, 1.0)) }

pub fn with_alpha(c: Color32, a: f32) -> Color32 {
    let [r, g, b, aa] = c.to_srgba_unmultiplied();
    Color32::from_rgba_unmultiplied(r, g, b, (aa as f32 * a) as u8)
}

pub fn rgba_color(c: &crate::model::Rgba) -> Color32 {
    Color32::from_rgb((c.r.clamp(0.0, 1.0) * 255.0).round() as u8, (c.g.clamp(0.0, 1.0) * 255.0).round() as u8, (c.b.clamp(0.0, 1.0) * 255.0).round() as u8)
}

// MARK: - Fonts

pub fn font(size: f32, weight: overlay::Weight) -> FontId {
    let fam = match weight {
        overlay::Weight::Regular => FontFamily::Proportional,
        overlay::Weight::Medium => FontFamily::Name("medium".into()),
        overlay::Weight::Semibold => FontFamily::Name("semibold".into()),
        overlay::Weight::Bold => FontFamily::Name("bold".into()),
        overlay::Weight::Heavy => FontFamily::Name("heavy".into()),
    };
    FontId::new(size, fam)
}
pub fn regular(size: f32) -> FontId { FontId::proportional(size) }
pub fn medium(size: f32) -> FontId { font(size, overlay::Weight::Medium) }
pub fn semibold(size: f32) -> FontId { font(size, overlay::Weight::Semibold) }
pub fn bold(size: f32) -> FontId { font(size, overlay::Weight::Bold) }
pub fn mono(size: f32) -> FontId { FontId::monospace(size) }
/// Numbers in values (rounded design + monospaced digits in the macOS app).
pub fn value_font() -> FontId { FontId::new(11.5, FontFamily::Name("digits".into())) }
pub fn label_font() -> FontId { regular(11.5) }
pub fn section_font() -> FontId { semibold(10.5) }
pub fn icon_font(size: f32) -> FontId { FontId::new(size, FontFamily::Name("icons".into())) }
pub fn icon_fill_font(size: f32) -> FontId { FontId::new(size, FontFamily::Name("icons-fill".into())) }

pub fn install_fonts(ctx: &egui::Context) {
    let mut f = FontDefinitions::default();
    let add = |f: &mut FontDefinitions, name: &str, data: &'static [u8]| {
        f.font_data.insert(name.into(), Arc::new(FontData::from_static(data)));
    };
    let names = ["inter-400", "inter-500", "inter-600", "inter-700", "inter-800"];
    for (n, d) in names.iter().zip(overlay::INTER.iter()) {
        add(&mut f, n, d);
    }
    add(&mut f, "mono-400", overlay::MONO[0]);
    add(&mut f, "mono-500", overlay::MONO[1]);
    add(&mut f, "phosphor", include_bytes!("../../assets/fonts/Phosphor.ttf"));
    add(&mut f, "phosphor-fill", include_bytes!("../../assets/fonts/Phosphor-Fill.ttf"));
    let fallbacks = ["phosphor", "mono-400"];
    let fam = |main: &str| -> Vec<String> {
        let mut v = vec![main.to_string()];
        v.extend(fallbacks.iter().map(|s| s.to_string()));
        v
    };
    f.families.insert(FontFamily::Proportional, fam("inter-400"));
    f.families.insert(FontFamily::Monospace, vec!["mono-400".into(), "inter-400".into(), "phosphor".into()]);
    f.families.insert(FontFamily::Name("medium".into()), fam("inter-500"));
    f.families.insert(FontFamily::Name("semibold".into()), fam("inter-600"));
    f.families.insert(FontFamily::Name("bold".into()), fam("inter-700"));
    f.families.insert(FontFamily::Name("heavy".into()), fam("inter-800"));
    f.families.insert(FontFamily::Name("digits".into()), vec!["mono-500".into(), "inter-500".into(), "phosphor".into()]);
    f.families.insert(FontFamily::Name("icons".into()), vec!["phosphor".into(), "inter-400".into()]);
    f.families.insert(FontFamily::Name("icons-fill".into()), vec!["phosphor-fill".into(), "phosphor".into(), "inter-400".into()]);
    ctx.set_fonts(f);
}

/// egui visuals matching the app colours.
pub fn apply_style(ctx: &egui::Context) {
    let mut v = if is_dark() { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = panel_bg();
    v.window_fill = panel_bg();
    v.extreme_bg_color = field();
    v.faint_bg_color = panel_bg2();
    v.selection.bg_fill = with_alpha(ACCENT, 0.45);
    v.selection.stroke = egui::Stroke::new(1.0, ACCENT);
    v.hyperlink_color = accent_text();
    v.window_stroke = egui::Stroke::new(1.0, separator());
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, separator());
    v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, text());
    v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, text());
    v.widgets.inactive.weak_bg_fill = field();
    v.widgets.inactive.bg_fill = field();
    v.widgets.hovered.weak_bg_fill = field_strong();
    v.widgets.hovered.bg_fill = field_strong();
    v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, text());
    v.widgets.active.weak_bg_fill = field_strong();
    v.widgets.active.fg_stroke = egui::Stroke::new(1.0, text());
    v.text_cursor.stroke = egui::Stroke::new(1.5, ACCENT);
    v.window_corner_radius = egui::CornerRadius::same(12);
    v.menu_corner_radius = egui::CornerRadius::same(8);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 6.0);
        s.spacing.button_padding = egui::vec2(8.0, 4.0);
        s.interaction.tooltip_delay = 0.6;
        s.text_styles.insert(egui::TextStyle::Body, regular(12.0));
        s.text_styles.insert(egui::TextStyle::Button, regular(12.0));
        s.text_styles.insert(egui::TextStyle::Small, regular(10.5));
        s.text_styles.insert(egui::TextStyle::Monospace, mono(12.0));
        s.text_styles.insert(egui::TextStyle::Heading, semibold(16.0));
    });
}
