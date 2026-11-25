//! Headless rendering used for automated checks:
//! effector --render-test <image> <outdir> [effect[:key=value,...][+effect...] | presets[=Category]] [--check-neutral] [--full]
//! effector --lint | --dump-params

use std::path::Path;

use crate::effects;
use crate::imageio::{self, Format, SourceImage};
use crate::model::*;
use crate::renderer::Renderer;

pub fn dump_params() {
    for e in effects::all() {
        println!("== {} [{:?}] {}", e.id, e.category, e.name);
        for d in &e.params {
            let k = match &d.kind {
                ParamKind::Slider { min, max, def, unit, .. } => format!("slider {min}…{max} def {def}{unit}"),
                ParamKind::Toggle(def) => format!("toggle def {def}{}", if d.is_section_switch { " (section switch)" } else { "" }),
                ParamKind::Choice(o, def) | ParamKind::Segmented(o, def) => {
                    format!("choice def {def} [{}]", o.iter().map(|x| x.id.as_str()).collect::<Vec<_>>().join(","))
                }
                ParamKind::Color(c) => format!("color def {}", c.to_hex()),
                ParamKind::Gradient(_) => "gradient".into(),
                ParamKind::Palette(_) => "palette".into(),
                ParamKind::Text(t) => format!("text def '{t}'"),
                ParamKind::Curves => "curves".into(),
                ParamKind::Lut => "lut".into(),
                ParamKind::Seed => "seed".into(),
                ParamKind::Wheel(p) => format!("wheel {p}"),
                ParamKind::WheelRow(v) => format!("wheels {}", v.join(",")),
            };
            println!("  {}: {k}{}", d.key, d.section.as_ref().map(|s| format!("  §{s}")).unwrap_or_default());
        }
        if !e.looks.is_empty() {
            println!("  looks: {}", e.looks.iter().map(|l| l.id.as_str()).collect::<Vec<_>>().join(", "));
        }
    }
}

pub fn lint() {
    let mut problems: Vec<String> = vec![];
    for e in effects::all() {
        let mut keys = std::collections::HashSet::new();
        for d in &e.params {
            if !keys.insert(d.key.clone()) {
                problems.push(format!("{}: duplicate key {}", e.id, d.key));
            }
            match &d.kind {
                ParamKind::Slider { min, max, def, .. } if def < min || def > max => {
                    problems.push(format!("{}.{}: default {def} outside {min}…{max}", e.id, d.key))
                }
                ParamKind::Choice(o, def) | ParamKind::Segmented(o, def) if !o.iter().any(|x| &x.id == def) => {
                    problems.push(format!("{}.{}: default {def} not an option", e.id, d.key))
                }
                _ => {}
            }
        }
        for l in &e.looks {
            for (k, _) in &l.values {
                if !keys.contains(k) && k != "_look" {
                    problems.push(format!("{} look {}: unknown key {k}", e.id, l.id));
                }
            }
        }
    }
    fn check(nodes: &[LayerNode], w: &str, problems: &mut Vec<String>) {
        for n in nodes {
            if n.is_group {
                check(&n.children, w, problems);
                continue;
            }
            let Some(e) = effects::effect(&n.effect_id) else {
                problems.push(format!("{w}: missing effect {}", n.effect_id));
                continue;
            };
            for k in n.params.values.keys() {
                if k != "_look" && !e.params.iter().any(|d| &d.key == k) {
                    problems.push(format!("{w} / {}: unknown key {k}", e.id));
                }
            }
        }
    }
    for p in crate::presets::factory() {
        check(&p.layers, &format!("preset {}", p.name), &mut problems);
    }
    // every kernel compiles
    for k in crate::gpu::kernel_names() {
        let src = crate::gpu::kernel_source(k).unwrap();
        let mut fe = naga::front::glsl::Frontend::default();
        if let Err(e) = fe.parse(&naga::front::glsl::Options::from(naga::ShaderStage::Compute), &src) {
            problems.push(format!("kernel {k}: {}", e.emit_to_string(&src)));
        }
    }
    println!("{}", if problems.is_empty() { "lint: OK".to_string() } else { problems.join("\n") });
    println!("effects: {}, presets: {}", effects::all().len(), crate::presets::factory().len());
}

fn parse_spec(spec: &str) -> Vec<LayerNode> {
    let mut layers = vec![];
    for part in spec.split('+') {
        let mut it = part.splitn(2, ':');
        let id = it.next().unwrap();
        let mut node = LayerNode::effect(id, None, None);
        if let Some(kvs) = it.next() {
            for kv in kvs.split(',') {
                let mut p = kv.splitn(2, '=');
                let (Some(k), Some(v)) = (p.next(), p.next()) else { continue };
                // look=<id>: start from one of the effect's Looks
                if k == "look" {
                    if let Some(l) = effects::effect(id).and_then(|e| e.looks.iter().find(|l| l.id == v)) {
                        for (lk, lv) in &l.values {
                            node.params.set(lk, lv.clone());
                        }
                    }
                    continue;
                }
                // depth mask: dm=from-to[-feather][-inv]
                if k == "dm" {
                    let parts: Vec<&str> = v.split('/').collect();
                    let n = |i: usize, d: f64| parts.get(i).and_then(|x| x.parse::<f64>().ok()).unwrap_or(d);
                    node.depth_mask = Some(DepthMask { enabled: true, from: n(0, 0.0), to: n(1, 0.45), feather: n(2, 0.12), invert: parts.contains(&"inv") });
                    continue;
                }
                let val = if let Ok(d) = v.parse::<f64>() {
                    ParamValue::Number(d)
                } else if v == "true" || v == "false" {
                    ParamValue::Bool(v == "true")
                } else {
                    ParamValue::Choice(v.into())
                };
                node.params.set(k, val);
            }
        }
        layers.insert(0, node);
    }
    layers
}

/// `--depth-view <image> <out.png>`: writes the depth map at render size (dev/test).
pub fn depth_view(args: &[String]) {
    let Some(i) = args.iter().position(|a| a == "--depth-view") else { return };
    crate::gpu::init_headless().expect("gpu");
    let src = SourceImage::load_path(Path::new(&args[i + 1])).expect("image");
    let t0 = std::time::Instant::now();
    let Some(map) = crate::depth::get_blocking(&src) else {
        println!("depth unavailable: model at {:?}", crate::depth::model_candidates());
        return;
    };
    println!("depth {}x{} in {:?}", map.w, map.h, t0.elapsed());
    let tex = src.make_texture(&CropRect::default(), (1200.0 / src.width.max(src.height) as f64).min(1.0));
    let mut ctx = crate::gpu::Ctx::new(1.0, tex.clone(), true);
    ctx.depth_src = Some(crate::depth::DepthSrc { map, crop: CropRect::default() });
    let d = ctx.depth().unwrap();
    let v = ctx.apply("k_depthview", &[&d], &[]);
    ctx.flush();
    let _ = imageio::write(&v, Path::new(&args[i + 2]), imageio::Format::Png, 1.0, false);
}

pub fn run(args: &[String]) {
    let Some(i) = args.iter().position(|a| a == "--render-test") else { return };
    if args.len() < i + 3 {
        println!("usage: --render-test <image> <outdir> [effect[:k=v,...]]");
        return;
    }
    if let Err(e) = crate::gpu::init_headless() {
        println!("error: {e}");
        return;
    }
    let input = Path::new(&args[i + 1]);
    let out_dir = Path::new(&args[i + 2]);
    let _ = std::fs::create_dir_all(out_dir);
    if args.iter().any(|a| a == "--project-roundtrip") {
        project_roundtrip(input, out_dir);
        return;
    }
    let specs: Vec<&String> = args[i + 3..].iter().filter(|a| !a.starts_with("--")).collect();
    let src = match SourceImage::load_path(input) {
        Ok(s) => s,
        Err(e) => {
            println!("error: {e}");
            return;
        }
    };
    let full = args.iter().any(|a| a == "--full");
    let scale = if args.iter().any(|a| a == "--full2x") {
        2.0
    } else if full {
        1.0
    } else {
        (1200.0 / src.width.max(src.height) as f64).min(1.0)
    };
    let tex = src.make_texture(&CropRect::default(), scale);
    let mut r = Renderer::new(false);
    let list: Vec<String> = if specs.is_empty() { effects::all().iter().map(|e| e.id.to_string()).collect() } else { specs.iter().map(|s| s.to_string()).collect() };
    let mut sheet: Vec<(String, Vec<u8>, usize, usize)> = vec![("original".into(), imageio::read_rgba8(&tex), tex.width(), tex.height())];
    for spec in &list {
        if spec == "presets" || spec.starts_with("presets=") {
            let cat = spec.strip_prefix("presets=").map(|c| c.replace('_', " "));
            for pr in crate::presets::factory() {
                if cat.as_ref().map(|c| pr.category.as_deref() == Some(c.as_str())).unwrap_or(true) {
                    let t0 = std::time::Instant::now();
                    let extras = crate::renderer::Extras::for_render(&src, &CropRect::default(), &pr.layers, true);
                    let out = r.render(&pr.layers, &tex, 0, false, None, extras);
                    println!("ok preset {} {}ms", pr.name, t0.elapsed().as_millis());
                    sheet.push((pr.name.clone(), imageio::read_rgba8(&out), out.width(), out.height()));
                }
            }
            continue;
        }
        let layers = parse_spec(spec);
        let t0 = std::time::Instant::now();
        let extras = crate::renderer::Extras::for_render(&src, &CropRect::default(), &layers, true);
        let out = r.render(&layers, &tex, 0, false, None, extras);
        let ms = t0.elapsed().as_millis();
        let rgba8 = imageio::read_rgba8(&out);
        if args.iter().any(|a| a == "--check-neutral") {
            let mut c = crate::gpu::Ctx::new(1.0, out.clone(), true);
            let f = c.read_floats(&out);
            let mut max_diff = 0f32;
            for px in f.chunks(4) {
                max_diff = max_diff.max((px[0] - px[1]).abs()).max((px[1] - px[2]).abs());
            }
            println!("neutral-check {spec}: max channel difference = {max_diff}");
        }
        let name: String = spec.replace(':', "_").replace(',', "_").replace('=', "-").replace('+', "__").chars().take(120).collect();
        let path = out_dir.join(format!("{name}.jpg"));
        if let Ok(b) = imageio::encode(&rgba8, out.width(), out.height(), Format::Jpg, 0.85, false) {
            let _ = std::fs::write(&path, b);
        }
        if !full {
            sheet.push((spec.clone(), rgba8, out.width(), out.height()));
        }
        println!("ok {spec} {ms}ms -> {}", path.display());
    }
    write_sheet(&sheet, &out_dir.join("_sheet.jpg"));
}

/// Contact sheet: grid of results with labels.
fn write_sheet(items: &[(String, Vec<u8>, usize, usize)], path: &Path) {
    let Some(first) = items.first() else { return };
    let cols = items.len().min(4);
    let rows = items.len().div_ceil(cols);
    let cw = 600usize;
    let ch = (cw as f64 * first.3 as f64 / first.2 as f64) as usize;
    let label_h = 26;
    let (w, h) = (cols * cw, rows * (ch + label_h));
    let mut ov = crate::overlay::OverlayCanvas::new(w, h);
    ov.set_fill(Rgba::new(0.12, 0.12, 0.12), 1.0);
    ov.fill_rect(0.0, 0.0, w as f64, h as f64);
    for (i, (name, px, iw, ih)) in items.iter().enumerate() {
        let (c, r) = (i % cols, i / cols);
        let img = image::RgbaImage::from_raw(*iw as u32, *ih as u32, px.clone()).unwrap();
        let small = image::imageops::resize(&img, cw as u32, ch as u32, image::imageops::FilterType::Triangle);
        let pm = tiny_skia::Pixmap::from_vec(
            small.into_raw().chunks(4).flat_map(|p| {
                let a = p[3] as u32;
                [(p[0] as u32 * a / 255) as u8, (p[1] as u32 * a / 255) as u8, (p[2] as u32 * a / 255) as u8, p[3]]
            }).collect(),
            tiny_skia::IntSize::from_wh(cw as u32, ch as u32).unwrap(),
        )
        .unwrap();
        ov.pixmap.draw_pixmap((c * cw) as i32, (r * (ch + label_h)) as i32, pm.as_ref(), &Default::default(), tiny_skia::Transform::identity(), None);
        ov.text(name, (c * cw + 8) as f64, (r * (ch + label_h) + ch + 4) as f64, 15.0, Rgba::WHITE, crate::overlay::Weight::Medium, false, false,
            None, 0.0, None, 1.0, None, 0.0);
    }
    let rgba: Vec<u8> = ov.data().to_vec();
    if let Ok(b) = imageio::encode(&rgba, w, h, Format::Jpg, 0.85, false) {
        let _ = std::fs::write(path, b);
    }
}

/// Saves a project with a preset stack and a crop, reopens it and compares the document state.
fn project_roundtrip(input: &Path, out_dir: &Path) {
    let ctx = egui::Context::default();
    let mut app = crate::app::AppState::new(ctx);
    app.headless = true;
    let img = match SourceImage::load_path(input) {
        Ok(i) => i,
        Err(e) => {
            println!("roundtrip error: {e}");
            return;
        }
    };
    let mut st = DocumentState::default();
    st.layers = crate::presets::factory().iter().find(|p| p.category.as_deref() == Some("Tracking & HUD")).unwrap().layers.clone();
    st.crop = CropRect { x: 0.1, y: 0.1, w: 0.8, h: 0.6 };
    st.canvas = Some(CanvasSpec { ratio: "1:1".into(), fill: "blur".into(), ..Default::default() });
    app.open_image(img, None, st.clone(), None, true);
    let path = out_dir.join("test.effector");
    if let Err(e) = app.write_project(&path) {
        println!("roundtrip error: {e}");
        return;
    }
    match app.open_project(&path, true) {
        Ok(_) => {
            let back = app.state();
            println!("roundtrip equal: {} layers: {} file size: {} bytes", back == st, back.layers.len(), std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0));
            if back != st {
                let a = serde_json::to_string_pretty(&st).unwrap();
                let b = serde_json::to_string_pretty(&back).unwrap();
                for (i, (x, y)) in a.lines().zip(b.lines()).enumerate() {
                    if x != y {
                        println!("first difference at line {i}:
  saved:  {x}
  loaded: {y}");
                        break;
                    }
                }
            }
        }
        Err(e) => println!("roundtrip error: {e}"),
    }
}

/// `--fuzz <image> [iterations]`: renders every effect with random / extreme settings through a
/// cached renderer (like the preview worker) and reports panics and GPU validation errors.
pub fn fuzz(args: &[String]) {
    let Some(i) = args.iter().position(|a| a == "--fuzz") else { return };
    if let Err(e) = crate::gpu::init_headless() {
        println!("error: {e}");
        return;
    }
    let Some(path) = args.get(i + 1) else { return };
    let iters: usize = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(4);
    let src = SourceImage::load_path(Path::new(path)).expect("image");
    let mut rng = crate::swatches::SplitMix::new(7);
    let mut failures = 0;
    for it in 0..iters {
        let scale = [0.25, 0.5, 1.0][it % 3];
        let tex = src.make_texture(&CropRect::default(), scale);
        let mut r = Renderer::new(true);
        for e in effects::all() {
            let mut node = LayerNode::effect(e.id, None, None);
            for d in &e.params {
                match &d.kind {
                    ParamKind::Slider { min, max, .. } => {
                        let v = match rng.next() % 4 { 0 => *min, 1 => *max, _ => min + (max - min) * (rng.next() % 1000) as f64 / 1000.0 };
                        node.params.set(&d.key, ParamValue::Number(v));
                    }
                    ParamKind::Choice(o, _) | ParamKind::Segmented(o, _) if !o.is_empty() => {
                        node.params.set(&d.key, ParamValue::Choice(o[(rng.next() % o.len() as u64) as usize].id.clone()));
                    }
                    ParamKind::Toggle(_) => node.params.set(&d.key, ParamValue::Bool(rng.next() % 2 == 0)),
                    _ => {}
                }
            }
            let layers = vec![node.clone()];
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let extras = crate::renderer::Extras::for_render(&src, &CropRect::default(), &layers, true);
                let out = r.render(&layers, &tex, it as u64, false, None, extras);
                let _ = imageio::read_rgba8(&out);
            }));
            if res.is_err() {
                failures += 1;
                println!("PANIC effect={} scale={scale} params={}", e.id, serde_json::to_string(&node.params).unwrap_or_default());
            }
        }
        println!("iteration {it} done");
    }
    println!("fuzz finished: {failures} failures");
}

/// `--list-looks`: effects and how many Looks each has (dev).
pub fn list_looks() {
    for e in effects::all().iter().filter(|e| !e.hidden) {
        println!("{:3} {:14} {}", e.looks.len(), e.id, e.name);
    }
}
