use super::*;
use crate::overlay::{OverlayCanvas, Weight};

#[derive(Clone, Copy, Debug)]
pub struct Blob {
    /// normalized 0..1, top-left origin (x, y, w, h)
    pub rect: (f64, f64, f64, f64),
    /// normalized
    pub area: f64,
}

impl Blob {
    fn center(&self) -> (f64, f64) { (self.rect.0 + self.rect.2 / 2.0, self.rect.1 + self.rect.3 / 2.0) }
}

/// Connected components (4-neighbour flood fill) on a w×h mask given by `test`.
pub fn components(w: usize, h: usize, test: &dyn Fn(usize) -> bool, min_area: f64, max_area: f64) -> Vec<Blob> {
    let mut label = vec![0i32; w * h];
    let mut blobs = vec![];
    let mut stack: Vec<usize> = vec![];
    let mut next = 1i32;
    for start in 0..w * h {
        if label[start] != 0 || !test(start) {
            continue;
        }
        let (mut min_x, mut min_y, mut max_x, mut max_y, mut count) = (w, h, 0usize, 0usize, 0usize);
        stack.push(start);
        label[start] = next;
        while let Some(i) = stack.pop() {
            let (x, y) = (i % w, i / w);
            count += 1;
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
            for (dx, dy) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                if nx < 0 || ny < 0 || nx >= w as i64 || ny >= h as i64 {
                    continue;
                }
                let j = ny as usize * w + nx as usize;
                if label[j] == 0 && test(j) {
                    label[j] = next;
                    stack.push(j);
                }
            }
        }
        next += 1;
        let area = count as f64 / (w * h) as f64;
        if area >= min_area && area <= max_area {
            blobs.push(Blob {
                rect: (
                    min_x as f64 / w as f64,
                    min_y as f64 / h as f64,
                    (max_x - min_x + 1) as f64 / w as f64,
                    (max_y - min_y + 1) as f64 / h as f64,
                ),
                area,
            });
        }
    }
    blobs
}

/// Box-area filter shared by the "Vision" style detectors.
fn keep(out: &mut Vec<Blob>, r: (f64, f64, f64, f64), min_area: f64, max_area: f64) {
    let a = r.2 * r.3;
    if a >= min_area && a <= max_area {
        out.push(Blob { rect: r, area: a });
    }
}

fn luma_of(px: &[f32], i: usize) -> f32 { 0.2126 * px[i * 4] + 0.7152 * px[i * 4 + 1] + 0.0722 * px[i * 4 + 2] }

/// Otsu threshold of values in 0…1.
fn otsu(v: &[f32]) -> f32 {
    let mut hist = [0f64; 256];
    for x in v {
        hist[(x.clamp(0.0, 1.0) * 255.0) as usize] += 1.0;
    }
    let total = v.len() as f64;
    let sum: f64 = (0..256).map(|i| i as f64 * hist[i]).sum();
    let (mut sb, mut wb, mut best, mut th) = (0.0, 0.0, 0.0, 127usize);
    for (t, hv) in hist.iter().enumerate() {
        wb += hv;
        if wb == 0.0 {
            continue;
        }
        let wf = total - wb;
        if wf == 0.0 {
            break;
        }
        sb += t as f64 * hv;
        let (mb, mf) = (sb / wb, (sum - sb) / wf);
        let between = wb * wf * (mb - mf) * (mb - mf);
        if between > best {
            best = between;
            th = t;
        }
    }
    th as f32 / 255.0
}

/// Contour detection (replaces Vision's VNDetectContoursRequest): contrast-boosted luminance,
/// Otsu split, bounding boxes of the dark-on-light shapes (plus light shapes when few are found).
pub fn contours(px: &[f32], w: usize, h: usize, min_area: f64, max_area: f64) -> Vec<Blob> {
    let lum: Vec<f32> = (0..w * h).map(|i| ((luma_of(px, i) - 0.5) * 1.5 + 0.5).clamp(0.0, 1.0)).collect();
    let t = otsu(&lum);
    let mut out = vec![];
    for b in components(w, h, &|i| lum[i] < t, 0.0, 1.0) {
        keep(&mut out, b.rect, min_area, max_area);
    }
    if out.len() < 6 {
        for b in components(w, h, &|i| lum[i] >= t, 0.0, 1.0) {
            keep(&mut out, b.rect, min_area, max_area);
        }
    }
    out
}

fn fft(re: &mut [f64], im: &mut [f64], inverse: bool) {
    let n = re.len();
    let mut j = 0;
    for i in 1..n {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j ^= bit;
            bit >>= 1;
        }
        j |= bit;
        if i < j {
            re.swap(i, j);
            im.swap(i, j);
        }
    }
    let mut len = 2;
    while len <= n {
        let ang = 2.0 * std::f64::consts::PI / len as f64 * if inverse { 1.0 } else { -1.0 };
        let (wr, wi) = (ang.cos(), ang.sin());
        for i in (0..n).step_by(len) {
            let (mut cr, mut ci) = (1.0, 0.0);
            for k in 0..len / 2 {
                let (ar, ai) = (re[i + k], im[i + k]);
                let (br, bi) = (re[i + k + len / 2] * cr - im[i + k + len / 2] * ci, re[i + k + len / 2] * ci + im[i + k + len / 2] * cr);
                re[i + k] = ar + br;
                im[i + k] = ai + bi;
                re[i + k + len / 2] = ar - br;
                im[i + k + len / 2] = ai - bi;
                let ncr = cr * wr - ci * wi;
                ci = cr * wi + ci * wr;
                cr = ncr;
            }
        }
        len <<= 1;
    }
    if inverse {
        for v in re.iter_mut() {
            *v /= n as f64;
        }
        for v in im.iter_mut() {
            *v /= n as f64;
        }
    }
}

fn fft2(re: &mut [f64], im: &mut [f64], n: usize, inverse: bool) {
    let (mut rr, mut ri) = (vec![0.0; n], vec![0.0; n]);
    for y in 0..n {
        rr.copy_from_slice(&re[y * n..y * n + n]);
        ri.copy_from_slice(&im[y * n..y * n + n]);
        fft(&mut rr, &mut ri, inverse);
        re[y * n..y * n + n].copy_from_slice(&rr);
        im[y * n..y * n + n].copy_from_slice(&ri);
    }
    for x in 0..n {
        for y in 0..n {
            rr[y] = re[y * n + x];
            ri[y] = im[y * n + x];
        }
        fft(&mut rr, &mut ri, inverse);
        for y in 0..n {
            re[y * n + x] = rr[y];
            im[y * n + x] = ri[y];
        }
    }
}

/// Saliency (replaces Vision's objectness/attention saliency): spectral-residual saliency map
/// (Hou & Zhang 2007) on a 64×64 copy; salient regions become boxes, plus one box around all of them.
pub fn saliency(px: &[f32], w: usize, h: usize, min_area: f64, max_area: f64) -> Vec<Blob> {
    const N: usize = 64;
    let mut re = vec![0.0f64; N * N];
    let mut im = vec![0.0f64; N * N];
    for y in 0..N {
        for x in 0..N {
            let (sx, sy) = ((x * w / N).min(w - 1), (y * h / N).min(h - 1));
            re[y * N + x] = luma_of(px, sy * w + sx) as f64;
        }
    }
    fft2(&mut re, &mut im, N, false);
    let mut amp: Vec<f64> = (0..N * N).map(|i| (re[i].hypot(im[i])).max(1e-9).ln()).collect();
    let phase: Vec<f64> = (0..N * N).map(|i| im[i].atan2(re[i])).collect();
    // spectral residual = log amplitude minus its 3×3 local average
    let avg: Vec<f64> = (0..N * N)
        .map(|i| {
            let (x, y) = ((i % N) as i64, (i / N) as i64);
            let mut s = 0.0;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (xx, yy) = ((x + dx).rem_euclid(N as i64), (y + dy).rem_euclid(N as i64));
                    s += amp[(yy * N as i64 + xx) as usize];
                }
            }
            s / 9.0
        })
        .collect();
    for i in 0..N * N {
        amp[i] -= avg[i];
        let m = amp[i].exp();
        re[i] = m * phase[i].cos();
        im[i] = m * phase[i].sin();
    }
    fft2(&mut re, &mut im, N, true);
    let mut sal: Vec<f64> = (0..N * N).map(|i| re[i] * re[i] + im[i] * im[i]).collect();
    // smooth (3 box passes ≈ gaussian)
    for _ in 0..3 {
        let src = sal.clone();
        for y in 0..N {
            for x in 0..N {
                let mut s = 0.0;
                let mut c = 0.0;
                for dy in -1i64..=1 {
                    for dx in -1i64..=1 {
                        let (xx, yy) = (x as i64 + dx, y as i64 + dy);
                        if xx >= 0 && yy >= 0 && xx < N as i64 && yy < N as i64 {
                            s += src[yy as usize * N + xx as usize];
                            c += 1.0;
                        }
                    }
                }
                sal[y * N + x] = s / c;
            }
        }
    }
    let mean = sal.iter().sum::<f64>() / (N * N) as f64;
    let mask: Vec<bool> = sal.iter().map(|v| *v > mean * 3.0).collect();
    let mut out = vec![];
    let comps = components(N, N, &|i| mask[i], 0.002, 1.0);
    for b in &comps {
        keep(&mut out, b.rect, min_area, max_area);
    }
    if !comps.is_empty() {
        let x0 = comps.iter().map(|b| b.rect.0).fold(1.0, f64::min);
        let y0 = comps.iter().map(|b| b.rect.1).fold(1.0, f64::min);
        let x1 = comps.iter().map(|b| b.rect.0 + b.rect.2).fold(0.0, f64::max);
        let y1 = comps.iter().map(|b| b.rect.1 + b.rect.3).fold(0.0, f64::max);
        keep(&mut out, (x0, y0, x1 - x0, y1 - y0), min_area, max_area);
    }
    out
}

pub fn blob_tracker() -> Effect {
    Effect::new("blob", "Blob Tracker", Category::Overlay, |input, p, ctx| {
        let mode = p.choice_or("detect", "brightness");
        let (min_a, max_a) = (p.num("minSize", 0.3) / 100.0, p.num("maxSize", 40.0) / 100.0);
        let mut blobs: Vec<Blob> = if mode == "contours" || mode == "saliency" {
            let f = (768.0 / input.width().max(input.height()) as f64).min(1.0);
            let (w, h) = (((input.width() as f64 * f) as usize).max(1), ((input.height() as f64 * f) as usize).max(1));
            let small = ctx.resize(input, w, h, false);
            let px = ctx.read_floats(&small);
            if mode == "saliency" { saliency(&px, w, h, min_a, 1.0) } else {
                // Vision worked at ≤ 512 px
                let f2 = (512.0 / w.max(h) as f64).min(1.0);
                let (w2, h2) = (((w as f64 * f2) as usize).max(1), ((h as f64 * f2) as usize).max(1));
                let s2 = ctx.resize(&small, w2, h2, false);
                let px2 = ctx.read_floats(&s2);
                contours(&px2, w2, h2, min_a, max_a)
            }
        } else {
            let detail = p.num("detail", 160.0);
            let sc = detail / input.width().max(input.height()) as f64;
            let (w, h) = (((input.width() as f64 * sc) as usize).max(8), ((input.height() as f64 * sc) as usize).max(8));
            let small = ctx.resize(input, w, h, false);
            let px = ctx.read_floats(&small);
            let th = (p.num("threshold", 65.0) / 100.0) as f32;
            let target = p.color_or("target", hex("#E03020"));
            let tol = (p.num("tolerance", 25.0) / 100.0) as f32;
            let (tr, tg, tb) = (target.r as f32, target.g as f32, target.b as f32);
            match mode.as_str() {
                "darkness" => components(w, h, &|i| luma_of(&px, i) < 1.0 - th, min_a, max_a),
                "color" => components(w, h, &|i| {
                    let (dr, dg, db) = (px[i * 4] - tr, px[i * 4 + 1] - tg, px[i * 4 + 2] - tb);
                    (dr * dr + dg * dg + db * db).sqrt() < tol * 1.2
                }, min_a, max_a),
                _ => components(w, h, &|i| luma_of(&px, i) > th, min_a, max_a),
            }
        };
        blobs.sort_by(|a, b| b.area.partial_cmp(&a.area).unwrap());
        blobs.truncate(p.num("maxBlobs", 14.0).max(0.0) as usize);
        // stable reading order for IDs / sequence: left-to-right, top-to-bottom
        blobs.sort_by(|a, b| {
            let (ac, bc) = (a.center(), b.center());
            if (ac.1 * 4.0).round() == (bc.1 * 4.0).round() { ac.0.partial_cmp(&bc.0).unwrap() } else { ac.1.partial_cmp(&bc.1).unwrap() }
        });

        let (ww, hh) = (input.width() as f64, input.height() as f64);
        let mut ov = OverlayCanvas::new(input.width(), input.height());
        let mut mask = OverlayCanvas::new(input.width(), input.height());
        let color = p.color_or("color", Rgba::WHITE);
        let lw = ctx.px(p.num("lineWidth", 1.6)).max(0.5);
        let pad = ctx.px(p.num("padding", 4.0));
        let fixed = p.choice_or("boxMode", "fit") == "fixed";
        let bs = p.num("boxScale", 100.0) / 100.0;
        let fs = ctx.px(p.num("boxSize", 30.0));
        let rects: Vec<(f64, f64, f64, f64)> = blobs
            .iter()
            .map(|b| {
                let (cx, cy) = ((b.rect.0 + b.rect.2 / 2.0) * ww, (b.rect.1 + b.rect.3 / 2.0) * hh);
                if fixed {
                    return (cx - fs / 2.0, cy - fs / 2.0, fs, fs);
                }
                let w = (b.rect.2 * ww * bs + pad * 2.0).max(2.0);
                let h = (b.rect.3 * hh * bs + pad * 2.0).max(2.0);
                (cx - w / 2.0, cy - h / 2.0, w, h)
            })
            .collect();
        let mid = |r: &(f64, f64, f64, f64)| (r.0 + r.2 / 2.0, r.1 + r.3 / 2.0);
        // connections (under markers); "none" is kept for older projects
        let conn = if p.bool_or("connectOn", true) { p.choice_or("connect", "nearest") } else { "none".into() };
        if conn != "none" && rects.len() > 1 {
            ov.set_stroke(color, p.num("connectOpacity", 70.0) / 100.0);
            ov.set_line_width(ctx.px(p.num("connectWidth", 1.0)).max(0.5));
            if p.bool("dashed") {
                ov.set_dash(&[lw * 4.0, lw * 3.0]);
            }
            let cs: Vec<(f64, f64)> = rects.iter().map(mid).collect();
            match conn.as_str() {
                "sequence" => ov.add_lines(&cs),
                "center" => {
                    for pt in &cs {
                        ov.move_to(ww / 2.0, hh / 2.0);
                        ov.line_to(pt.0, pt.1);
                    }
                }
                "all" => {
                    for i in 0..cs.len() {
                        for j in i + 1..cs.len() {
                            ov.move_to(cs[i].0, cs[i].1);
                            ov.line_to(cs[j].0, cs[j].1);
                        }
                    }
                }
                _ => {
                    for (i, a) in cs.iter().enumerate() {
                        let mut best: Option<usize> = None;
                        let mut bd = f64::INFINITY;
                        for (j, b) in cs.iter().enumerate() {
                            if j == i {
                                continue;
                            }
                            let d = (a.0 - b.0).hypot(a.1 - b.1);
                            if d < bd {
                                bd = d;
                                best = Some(j);
                            }
                        }
                        if let Some(b) = best {
                            ov.move_to(a.0, a.1);
                            ov.line_to(cs[b].0, cs[b].1);
                        }
                    }
                }
            }
            ov.stroke_path();
            ov.set_dash(&[]);
        }
        // markers
        ov.set_stroke(color, 1.0);
        let fill_a = p.n("fill") / 100.0;
        ov.set_fill(color, fill_a);
        ov.set_line_width(lw);
        let shape = p.choice_or("shape", "box");
        let markers_on = p.bool_or("markersOn", true);
        for r in &rects {
            mask.set_fill(Rgba::WHITE, 1.0);
            mask.fill_rect(r.0, r.1, r.2, r.3);
            if !markers_on {
                continue;
            }
            if p.n("fill") > 0.0 {
                ov.fill_rect(r.0, r.1, r.2, r.3);
            }
            let (min_x, min_y, max_x, max_y) = (r.0, r.1, r.0 + r.2, r.1 + r.3);
            let (mx, my) = mid(r);
            match shape.as_str() {
                "corners" | "brackets" => {
                    let l = r.2.min(r.3) * if shape == "corners" { 0.25 } else { 0.15 };
                    if shape == "brackets" {
                        ov.add_lines(&[(min_x + l, min_y), (min_x, min_y), (min_x, max_y), (min_x + l, max_y)]);
                        ov.add_lines(&[(max_x - l, min_y), (max_x, min_y), (max_x, max_y), (max_x - l, max_y)]);
                    } else {
                        ov.add_lines(&[(min_x, min_y + l), (min_x, min_y), (min_x + l, min_y)]);
                        ov.add_lines(&[(max_x - l, min_y), (max_x, min_y), (max_x, min_y + l)]);
                        ov.add_lines(&[(max_x, max_y - l), (max_x, max_y), (max_x - l, max_y)]);
                        ov.add_lines(&[(min_x + l, max_y), (min_x, max_y), (min_x, max_y - l)]);
                    }
                    ov.stroke_path();
                }
                "circle" => {
                    let d = r.2.max(r.3);
                    ov.stroke_ellipse(mx - d / 2.0, my - d / 2.0, d, d);
                }
                "cross" => {
                    let l = r.2.max(r.3) * 0.6;
                    ov.move_to(mx - l, my);
                    ov.line_to(mx + l, my);
                    ov.move_to(mx, my - l);
                    ov.line_to(mx, my + l);
                    ov.stroke_path();
                    ov.stroke_rect(r.0 + r.2 * 0.25, r.1 + r.3 * 0.25, r.2 * 0.5, r.3 * 0.5);
                }
                _ => ov.stroke_rect(r.0, r.1, r.2, r.3),
            }
            if p.bool_or("centerDot", true) {
                let d = lw * 2.5;
                ov.set_fill(color, 1.0);
                ov.fill_ellipse(mx - d / 2.0, my - d / 2.0, d, d);
                ov.set_fill(color, fill_a);
            }
        }
        // labels
        let label_mode = if p.bool_or("labelsOn", true) { p.choice_or("label", "id") } else { "none".into() };
        if label_mode != "none" {
            let size = ctx.px(p.num("labelSize", 16.0));
            let inv = if color.luma() > 0.5 { Rgba::BLACK } else { Rgba::WHITE };
            for (i, r) in rects.iter().enumerate() {
                let (mx, my) = mid(r);
                let text = match label_mode.as_str() {
                    "coords" => format!("X{:04} Y{:04}", (mx / ww * 1000.0) as i64, (my / hh * 1000.0) as i64),
                    "size" => format!("{}×{}", (r.2 / ww * 1000.0) as i64, (r.3 / hh * 1000.0) as i64),
                    "hex" => format!("0x{:04X}", (i as i64 * 7919 + mx as i64 * 31 + my as i64) & 0xFFFF),
                    "custom" => {
                        let t = p.text_or("labelText", "TARGET");
                        if rects.len() > 1 { format!("{t} {}", i + 1) } else { t }
                    }
                    _ => format!("ID {:02}", i + 1),
                };
                let bg = p.bool_or("labelBG", true);
                let y = (r.1 - size * 1.35 - lw).max(0.0);
                ov.text(&text, r.0 + if bg { size * 0.25 } else { 0.0 }, y + size * 0.12, size, if bg { inv } else { color }, Weight::Semibold,
                    true, false, if bg { Some(color) } else { None }, if bg { size * 0.2 } else { 0.0 }, None, 1.0, None, 0.0);
            }
        }
        let (ot, mt) = (ov.texture(), mask.texture());
        let u = if p.bool_or("highlightOn", true) { [p.f("invertInside") / 100.0, p.f("dimOutside") / 100.0 * 0.85] } else { [0.0, 0.0] };
        ctx.apply("k_blob", &[input, &ot, &mt], &u)
    })
    .symbol("viewfinder")
    .summary("TouchDesigner-style tracking boxes, labels and connections.")
    .params(vec![
        ParamDef::choice("detect", "Detect By", &[("brightness", "Brightness"), ("darkness", "Darkness"), ("color", "Color"), ("contours", "Contours"), ("saliency", "Objects")], "brightness").sec("Detection"),
        s("threshold", "Threshold", 0.0, 100.0, 65.0).step(1.0).unit("%").sec("Detection").vis(when_any("detect", &["brightness", "darkness"])),
        ParamDef::color("target", "Target Color", hex("#E03020")).sec("Detection").vis(when("detect", "color")),
        s("tolerance", "Tolerance", 1.0, 100.0, 25.0).step(1.0).unit("%").sec("Detection").vis(when("detect", "color")),
        s("maxBlobs", "Max Blobs", 1.0, 60.0, 14.0).step(1.0).sec("Detection"),
        s("minSize", "Min Size", 0.0, 10.0, 0.3).step(0.05).unit("%").curve(2.5).sec("Detection"),
        s("maxSize", "Max Size", 1.0, 100.0, 40.0).step(1.0).unit("%").curve(2.0).sec("Detection"),
        s("detail", "Detail", 40.0, 400.0, 160.0).step(10.0).sec("Detection"),
        ParamDef::section_switch("markersOn", "Markers", true),
        ParamDef::choice("shape", "Marker", &[("box", "Box"), ("corners", "Corners"), ("circle", "Circle"), ("cross", "Crosshair"), ("brackets", "Brackets")], "box").sec("Markers"),
        ParamDef::segmented("boxMode", "Box Size", &[("fit", "Fit Blob"), ("fixed", "Fixed")], "fit").sec("Markers"),
        s("boxScale", "Box Scale", 10.0, 200.0, 100.0).step(1.0).unit("%").sec("Markers").vis(when_not("boxMode", "fixed")),
        s("boxSize", "Box Size", 4.0, 200.0, 30.0).step(1.0).curve(1.8).sec("Markers").vis(when("boxMode", "fixed")),
        s("padding", "Padding", -20.0, 40.0, 4.0).step(0.5).sec("Markers").vis(when_not("boxMode", "fixed")),
        ParamDef::color("color", "Color", hex("#FFFFFF")).sec("Markers"),
        s("lineWidth", "Line Width", 0.25, 8.0, 1.6).step(0.05).curve(1.8).sec("Markers"),
        ParamDef::toggle("centerDot", "Center Dot", true).sec("Markers"),
        s("fill", "Fill", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Markers"),
        ParamDef::section_switch("highlightOn", "Highlight", true),
        s("invertInside", "Invert Inside", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Highlight"),
        s("dimOutside", "Dim Outside", 0.0, 100.0, 0.0).step(1.0).unit("%").sec("Highlight"),
        ParamDef::section_switch("labelsOn", "Labels", true),
        ParamDef::choice("label", "Label", &[("id", "ID"), ("coords", "Coordinates"), ("size", "Size"), ("hex", "Hex ID"), ("custom", "Custom Text")], "id").sec("Labels"),
        ParamDef::text("labelText", "Text", "TARGET").sec("Labels").vis(when("label", "custom")),
        s("labelSize", "Label Size", 4.0, 60.0, 16.0).step(0.5).curve(1.5).sec("Labels"),
        ParamDef::toggle("labelBG", "Label Background", true).sec("Labels"),
        ParamDef::section_switch("connectOn", "Connections", true),
        ParamDef::choice("connect", "Style", &[("sequence", "Sequence"), ("nearest", "Nearest"), ("center", "To Center"), ("all", "All")], "nearest").sec("Connections"),
        s("connectWidth", "Line Width", 0.25, 6.0, 1.0).step(0.05).curve(1.8).sec("Connections"),
        ParamDef::toggle("dashed", "Dashed", false).sec("Connections"),
        s("connectOpacity", "Opacity", 0.0, 100.0, 70.0).step(1.0).unit("%").sec("Connections"),
    ])
}
