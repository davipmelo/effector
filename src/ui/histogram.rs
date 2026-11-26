//! Histogram panel at the top of the right sidebar (Lightroom Classic style): RGB with the
//! overlaps in cyan / magenta / yellow / grey, luminance, single channels or everything at once;
//! clipping triangles in the top corners (click or J paints clipped areas on the image);
//! collapsible, so the effect controls get the room back.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};

use super::theme::*;
use super::widgets::*;
use crate::app::AppState;
use crate::histogram::Histogram;
use crate::l10n::tr;
use crate::settings;
use crate::shortcuts::Action;

const MODES: [(&str, &str, &str); 6] = [
    ("rgb", "RGB", "Red, green and blue on top of each other"),
    ("luma", "Lum", "Luminance (brightness)"),
    ("red", "R", "Red channel"),
    ("green", "G", "Green channel"),
    ("blue", "B", "Blue channel"),
    ("all", "All", "RGB with the luminance outline"),
];

const RED: Color32 = Color32::from_rgb(232, 60, 60);
const GREEN: Color32 = Color32::from_rgb(70, 200, 90);
const BLUE: Color32 = Color32::from_rgb(70, 120, 255);

pub fn panel(ui: &mut Ui, app: &mut AppState) {
    let st = settings::get();
    let open = st.histogram_open;
    let w = ui.available_width();
    // header
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
    let cy = rect.center().y;
    paint_icon(ui, Rect::from_center_size(pos2(rect.left() + 18.0, cy), vec2(10.0, 10.0)), if open { "chevron.down" } else { "chevron.right" }, 9.0, text_faint());
    ui.painter().text(pos2(rect.left() + 28.0, cy), Align2::LEFT_CENTER, spaced_upper(&tr("Histogram")), section_font(), text_dim());
    if app.show_clipping {
        ui.painter().text(pos2(rect.right() - 14.0, cy), Align2::RIGHT_CENTER, tr("Clipping"), semibold(9.5), accent_text());
    }
    if resp.clicked() {
        app.perform(Action::ToggleHistogram, ui.ctx());
    }
    resp.on_hover_text(action_hint(if open { "Hide histogram" } else { "Show histogram" }, Action::ToggleHistogram));
    if open {
        egui::Frame::new().inner_margin(egui::Margin { left: 14, right: 14, top: 0, bottom: 10 }).show(ui, |ui| {
            ui.set_width(w - 28.0);
            body(ui, app, &st.histogram_mode);
        });
    }
    let (sep, _) = ui.allocate_exact_size(vec2(w, 1.0), Sense::hover());
    ui.painter().rect_filled(sep, CornerRadius::ZERO, separator());
}

fn body(ui: &mut Ui, app: &mut AppState, mode: &str) {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 96.0), Sense::hover());
    let p = ui.painter().with_clip_rect(rect);
    p.rect_filled(rect, CornerRadius::same(6), dyn_bg());
    // quarter lines
    for i in 1..4 {
        let x = rect.left() + rect.width() * i as f32 / 4.0;
        p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(1.0, separator()));
    }
    let hist = if app.has_image() { app.histogram.clone() } else { None };
    if let Some(h) = &hist {
        draw(&p, rect.shrink2(vec2(1.0, 2.0)), h, mode);
        triangles(ui, app, rect, h);
        if let Some(pos) = resp.hover_pos() {
            let lvl = (((pos.x - rect.left()) / rect.width()) * 255.0).round().clamp(0.0, 255.0) as usize;
            let x = rect.left() + lvl as f32 / 255.0 * rect.width();
            p.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(1.0, with_alpha(text(), 0.5)));
            let t = h.total.max(1) as f32 / 100.0;
            let info = format!(
                "{lvl}   R {:.1}%  G {:.1}%  B {:.1}%  L {:.1}%",
                h.r[lvl] as f32 / t, h.g[lvl] as f32 / t, h.b[lvl] as f32 / t, h.l[lvl] as f32 / t
            );
            ui.ctx().request_repaint();
            footer(ui, &info);
        } else {
            let (lo, hi) = (h.clipped(false), h.clipped(true));
            let pct = |v: [f32; 4]| v[..3].iter().cloned().fold(0.0f32, f32::max) * 100.0;
            footer(ui, &format!("{} {:.1}%   ·   {} {:.1}%", tr("Shadows"), pct(lo), tr("Highlights"), pct(hi)));
        }
    } else {
        p.text(rect.center(), Align2::CENTER_CENTER, tr("No image"), regular(11.0), text_faint());
        footer(ui, "");
    }
    // modes
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        let n = MODES.len() as f32;
        let bw = ((ui.available_width() - 3.0 * (n - 1.0)) / n).max(18.0);
        for (id, label, help) in MODES {
            let on = mode == id;
            let (r, rr) = ui.allocate_exact_size(vec2(bw, 18.0), Sense::click());
            let bg = if on { field_strong() } else if rr.hovered() { field() } else { Color32::TRANSPARENT };
            ui.painter().rect_filled(r, CornerRadius::same(4), bg);
            let col = match id {
                "red" => RED,
                "green" => GREEN,
                "blue" => BLUE,
                _ => if on { text() } else { text_dim() },
            };
            ui.painter().text(r.center(), Align2::CENTER_CENTER, label, if on { semibold(10.0) } else { regular(10.0) }, col);
            if rr.on_hover_text(tr(help)).clicked() {
                settings::update(|s| s.histogram_mode = id.to_string());
            }
        }
    });
}

fn footer(ui: &mut Ui, s: &str) {
    let w = ui.available_width();
    let (r, _) = ui.allocate_exact_size(vec2(w, 16.0), Sense::hover());
    let mut job = egui::text::LayoutJob::simple_singleline(s.to_string(), egui::FontId::new(9.5, egui::FontFamily::Name("digits".into())), text_faint());
    job.wrap = egui::text::TextWrapping::truncate_at_width(w);
    let g = ui.painter().layout_job(job);
    ui.painter().galley(pos2(r.center().x - g.size().x / 2.0, r.center().y - g.size().y / 2.0), g, text_faint());
}

fn dyn_bg() -> Color32 { if theme_dark() { Color32::from_black_alpha(70) } else { Color32::from_black_alpha(14) } }

/// Bin heights 0..1 (lightly smoothed; scaled to the tallest bin away from the clipped ends, so a
/// spike at pure black / white does not flatten the rest — Lightroom does the same).
fn heights(bins: &[u32]) -> Vec<f32> {
    let s: Vec<f32> = (0..256)
        .map(|i| {
            let a = bins[i.max(1) - 1] as f32;
            let b = bins[i] as f32;
            let c = bins[(i + 1).min(255)] as f32;
            if i == 0 || i == 255 { b } else { (a + 2.0 * b + c) / 4.0 }
        })
        .collect();
    let top = s[2..254].iter().cloned().fold(1.0f32, f32::max);
    s.iter().map(|v| (v / top).min(1.0)).collect()
}

fn sample(h: &[f32], t: f32) -> f32 {
    let x = t.clamp(0.0, 1.0) * 255.0;
    let i = x.floor() as usize;
    let j = (i + 1).min(255);
    let f = x - i as f32;
    h[i] * (1.0 - f) + h[j] * f
}

fn draw(p: &egui::Painter, r: Rect, h: &Histogram, mode: &str) {
    let (hr, hg, hb, hl) = (heights(&h.r), heights(&h.g), heights(&h.b), heights(&h.l));
    let cols = r.width().max(1.0) as usize;
    let col_rect = |x: usize, a: f32, b: f32| {
        let x0 = r.left() + x as f32;
        Rect::from_min_max(pos2(x0, r.bottom() - b * r.height()), pos2(x0 + 1.0, r.bottom() - a * r.height()))
    };
    let single = |c: Color32, hh: &[f32]| {
        for x in 0..cols {
            let v = sample(hh, x as f32 / (cols - 1).max(1) as f32);
            p.rect_filled(col_rect(x, 0.0, v), CornerRadius::ZERO, with_alpha(c, 0.85));
        }
    };
    match mode {
        "luma" => single(Color32::from_gray(if theme_dark() { 205 } else { 110 }), &hl),
        "red" => single(RED, &hr),
        "green" => single(GREEN, &hg),
        "blue" => single(BLUE, &hb),
        _ => {
            // additive look: where all three overlap grey, two → cyan / magenta / yellow, one → its colour
            let grey = Color32::from_gray(if theme_dark() { 190 } else { 120 });
            for x in 0..cols {
                let t = x as f32 / (cols - 1).max(1) as f32;
                let mut v = [(sample(&hr, t), 0usize), (sample(&hg, t), 1), (sample(&hb, t), 2)];
                v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                let (lo, mid, hi) = (v[0], v[1], v[2]);
                p.rect_filled(col_rect(x, 0.0, lo.0), CornerRadius::ZERO, with_alpha(grey, 0.85));
                let pair = match (mid.1.min(hi.1), mid.1.max(hi.1)) {
                    (0, 1) => Color32::from_rgb(220, 210, 60),
                    (0, 2) => Color32::from_rgb(210, 80, 220),
                    _ => Color32::from_rgb(60, 200, 220),
                };
                p.rect_filled(col_rect(x, lo.0, mid.0), CornerRadius::ZERO, with_alpha(pair, 0.8));
                let one = [RED, GREEN, BLUE][hi.1];
                p.rect_filled(col_rect(x, mid.0, hi.0), CornerRadius::ZERO, with_alpha(one, 0.85));
            }
            if mode == "all" {
                let pts: Vec<egui::Pos2> = (0..cols)
                    .map(|x| {
                        let v = sample(&hl, x as f32 / (cols - 1).max(1) as f32);
                        pos2(r.left() + x as f32 + 0.5, r.bottom() - v * r.height())
                    })
                    .collect();
                p.add(egui::Shape::line(pts, Stroke::new(1.2, Color32::WHITE)));
            }
        }
    }
}

/// Clipping indicators: lit (in the colour of the clipped channels) when any pixels are clipped;
/// click to paint clipped areas on the image (J).
fn triangles(ui: &mut Ui, app: &mut AppState, rect: Rect, h: &Histogram) {
    for high in [false, true] {
        let c = h.clipped(high);
        let lit = |v: f32| if v > 0.0005 { 255u8 } else { 0 };
        let (r, g, b) = (lit(c[0]), lit(c[1]), lit(c[2]));
        let any = r > 0 || g > 0 || b > 0;
        let col = if any { Color32::from_rgb(r.max(40), g.max(40), b.max(40)) } else { with_alpha(text_faint(), 0.6) };
        let s = 9.0;
        let (a, bb, cc) = if high {
            let o = pos2(rect.right() - 5.0, rect.top() + 5.0);
            (o, pos2(o.x - s, o.y), pos2(o.x, o.y + s))
        } else {
            let o = pos2(rect.left() + 5.0, rect.top() + 5.0);
            (o, pos2(o.x + s, o.y), pos2(o.x, o.y + s))
        };
        let hit = Rect::from_two_pos(a, pos2(if high { a.x - s - 3.0 } else { a.x + s + 3.0 }, a.y + s + 3.0));
        let resp = ui.interact(hit, Id::new(("hist-tri", high)), Sense::click());
        let fill = if any || resp.hovered() { col } else { Color32::TRANSPARENT };
        ui.painter().add(egui::Shape::convex_polygon(vec![a, bb, cc], fill, Stroke::new(1.0, col)));
        if app.show_clipping {
            ui.painter().rect_stroke(hit.expand(1.0), CornerRadius::same(2), Stroke::new(1.0, ACCENT), StrokeKind::Outside);
        }
        let tip = format!(
            "{} — R {:.2}%  G {:.2}%  B {:.2}%\n{}",
            tr(if high { "Highlights clipped" } else { "Shadows clipped" }),
            c[0] * 100.0, c[1] * 100.0, c[2] * 100.0,
            action_hint("Show clipping on the image", Action::ShowClipping)
        );
        if resp.on_hover_text(tip).clicked() {
            app.perform(Action::ShowClipping, ui.ctx());
        }
    }
}
