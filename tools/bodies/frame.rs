//== header
use crate::overlay::{OverlayCanvas, Weight};

pub const FRAME_RATIOS: [(&str, &str, f64); 9] = [
    ("1.33", "1.33:1 (Academy 4:3)", 4.0 / 3.0),
    ("1.66", "1.66:1 (European)", 1.66),
    ("1.78", "1.78:1 (16:9)", 16.0 / 9.0),
    ("1.85", "1.85:1 (Flat)", 1.85),
    ("2.00", "2.00:1 (Univisium)", 2.0),
    ("2.20", "2.20:1 (70 mm)", 2.2),
    ("2.39", "2.39:1 (Scope)", 2.39),
    ("2.76", "2.76:1 (Ultra Panavision)", 2.76),
    ("3.00", "3.00:1 (Ultra-wide)", 3.0),
];

/// CGRect.integral: smallest integer rect containing (x, y, w, h).
fn integral(x: f64, y: f64, w: f64, h: f64) -> (f64, f64, f64, f64) {
    let (x0, y0, x1, y1) = (x.floor(), y.floor(), (x + w).ceil(), (y + h).ceil());
    (x0, y0, x1 - x0, y1 - y0)
}
//== frame
        let (w, h) = (input.width() as f64, input.height() as f64);
        let m = w.min(h);
        let k = p.num("size", 100.0) / 100.0;
        let style = p.choice_or("style", "letterbox");
        let mut frame_color = p.color_or("color", hex("#F5F4F0"));
        let rect: (f64, f64, f64, f64);
        let mut radius = ctx.px(p.num("radius", 0.0));
        let mut texture = p.fd("texture", 50.0) / 100.0;
        let mut shadow = p.fd("shadow", 35.0) / 100.0;
        let mut src = (0.0, 0.0, 1.0, 1.0);
        match style.as_str() {
            "letterbox" => {
                frame_color = p.color_or("barColor", Rgba::BLACK);
                texture = 0.0;
                shadow = 0.0;
                radius = 0.0;
                let rid = p.choice_or("ratio", "2.39");
                let a = FRAME_RATIOS.iter().find(|r| r.0 == rid).map(|r| r.2).unwrap_or(2.39);
                let canvas = w / h;
                if a >= canvas {
                    let hh = (w / a).round();
                    rect = (0.0, ((h - hh) / 2.0).round(), w, hh);
                } else {
                    let ww = (h * a).round();
                    rect = (((w - ww) / 2.0).round(), 0.0, ww, h);
                }
                // the bars cover the image (same scale); Reframe slides the visible part
                let (slack_y, slack_x) = (1.0 - rect.3 / h, 1.0 - rect.2 / w);
                let o = p.n("offset") / 100.0;
                src = (slack_x / 2.0 * (1.0 + o), slack_y / 2.0 * (1.0 + o), rect.2 / w, rect.3 / h);
            }
            "border" | "rounded" => {
                let s = m * 0.05 * k;
                rect = integral(s, s, w - 2.0 * s, h - 2.0 * s);
                if style == "rounded" && radius == 0.0 {
                    radius = m * 0.02;
                }
            }
            _ => {
                // polaroid
                let (s, b) = (m * 0.055 * k, m * 0.22 * k);
                rect = integral(s, s, w - 2.0 * s, h - s - b);
                radius = m * 0.002;
            }
        }
        if style != "letterbox" {
            // cover-fit the image into the window
            let (ra, ia) = (rect.2 / rect.3.max(1.0), w / h);
            if ia > ra {
                let ww = ra / ia;
                src = ((1.0 - ww) / 2.0, 0.0, ww, 1.0);
            } else {
                let hh = ia / ra;
                src = (0.0, (1.0 - hh) / 2.0, 1.0, hh);
            }
        }
        let mut u = vec![rect.0 as f32, rect.1 as f32, rect.2 as f32, rect.3 as f32, radius as f32];
        u.extend(frame_color.floats());
        u.extend([texture, shadow, p.fd("seed", 1.0) * 3.3, ctx.scale as f32, src.0 as f32, src.1 as f32, src.2 as f32, src.3 as f32]);
        let mut out = ctx.apply("k_frame", &[input], &u);
        let caption = p.text("caption");
        if p.bool_or("textOn", false) && !caption.is_empty() {
            let mut ov = OverlayCanvas::new(input.width(), input.height());
            let fs = ctx.px(p.num("textSize", 26.0));
            let color = p.color_or("textColor", hex("#F2E36B"));
            let max_y = rect.1 + rect.3;
            match style.as_str() {
                "letterbox" => {
                    let in_bar = p.choice_or("textPlace", "bar") == "bar" && rect.1 > fs * 1.6;
                    let y = if in_bar { max_y + (h - max_y - fs) / 2.0 } else { max_y - fs * 2.2 };
                    let est = caption.chars().count() as f64 * fs * 0.5;
                    ov.text(&caption, (w - est) / 2.0, y, fs, color, Weight::Medium, false, !in_bar, None, 0.0, None, 1.0, None, 0.0);
                }
                "polaroid" => {
                    let bottom = h - max_y;
                    ov.text(&caption, rect.0 + rect.2 * 0.06, max_y + (bottom - fs) * 0.45, fs, color, Weight::Regular, false, false, None,
                        0.0, Some("Noteworthy-Bold"), 1.0, None, 0.0);
                }
                _ => {
                    ov.text(&caption, rect.0, max_y + ((h - max_y - fs) / 2.0).max(2.0), fs, color, Weight::Medium, false, false, None, 0.0,
                        None, 1.0, None, 0.0);
                }
            }
            let t = ov.texture();
            out = ctx.apply("k_over_pm", &[&out, &t], &[]);
        }
        out
