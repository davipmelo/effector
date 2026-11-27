//! Inspector control of an image parameter (Image Overlay, 0.9.1): the current image, the
//! built-in texture library by category (downloaded once), the user's own files (button or drop)
//! and Shuffle.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use egui::{pos2, vec2, Color32, CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui};
use uuid::Uuid;

use super::theme::*;
use super::widgets::*;
use crate::app::AppState;
use crate::l10n::{tr, trf};
use crate::model::{ImageRef, LayerNode, ParamValue};
use crate::textures::{self, Download};

/// egui textures of thumbnails, by image id.
fn thumbs() -> &'static Mutex<HashMap<String, Option<egui::TextureHandle>>> {
    static T: OnceLock<Mutex<HashMap<String, Option<egui::TextureHandle>>>> = OnceLock::new();
    T.get_or_init(Default::default)
}

/// Thumbnail of an image; loads a few per frame (`budget`) so opening a category never stalls.
fn thumb_tex(ui: &Ui, r: &ImageRef, budget: &mut usize) -> Option<egui::TextureHandle> {
    if let Some(t) = thumbs().lock().unwrap().get(&r.id) {
        return t.clone();
    }
    if *budget == 0 {
        ui.ctx().request_repaint();
        return None;
    }
    *budget -= 1;
    let t = textures::thumbnail(r, 256).map(|d| {
        ui.ctx().load_texture(format!("ovthumb-{}", r.id), egui::ColorImage::from_rgba_unmultiplied([d.w, d.h], &d.rgba), Default::default())
    });
    let mut m = thumbs().lock().unwrap();
    // keep it from growing without bound (user images change ids)
    if m.len() > 600 {
        m.clear();
    }
    m.insert(r.id.clone(), t.clone());
    t
}

fn paint_thumb(ui: &Ui, rect: Rect, tex: Option<&egui::TextureHandle>, highlighted: bool) {
    ui.painter().rect_filled(rect, CornerRadius::same(5), field());
    if let Some(t) = tex {
        // aspect fill
        let ts = t.size_vec2();
        let s = (rect.width() / ts.x).max(rect.height() / ts.y);
        let (uw, uh) = ((rect.width() / (ts.x * s)).min(1.0), (rect.height() / (ts.y * s)).min(1.0));
        let uv = Rect::from_min_max(pos2((1.0 - uw) / 2.0, (1.0 - uh) / 2.0), pos2((1.0 + uw) / 2.0, (1.0 + uh) / 2.0));
        let mut mesh = egui::Mesh::with_texture(t.id());
        mesh.add_rect_with_uv(rect, uv, Color32::WHITE);
        ui.painter().with_clip_rect(rect).add(egui::Shape::mesh(mesh));
    } else {
        paint_icon(ui, rect, "photo", 14.0, text_faint());
    }
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(5),
        Stroke::new(if highlighted { 1.5 } else { 1.0 }, if highlighted { ACCENT } else { separator() }),
        StrokeKind::Inside,
    );
}

/// Shown name: "Paper 10" (category translated) or the user's file name.
pub fn display_name(r: &ImageRef) -> String {
    if let Some(id) = r.builtin_id() {
        let (cat, n) = id.split_once('/').unwrap_or(("", id));
        let num = n.rsplit('-').next().unwrap_or(n);
        if let Some(c) = textures::library().and_then(|l| l.iter().find(|c| c.id == cat).map(|c| c.name.clone())) {
            return format!("{} {}", tr(&c), num);
        }
        return r.name.clone();
    }
    r.name.clone()
}

fn category_of(r: &ImageRef) -> Option<String> { r.builtin_id().and_then(|id| id.split_once('/')).map(|(c, _)| c.to_string()) }

/// A tiny xorshift seeded by the clock (Shuffle).
fn random(n: usize) -> usize {
    static S: Mutex<u64> = Mutex::new(0);
    let mut s = S.lock().unwrap();
    if *s == 0 {
        *s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1;
    }
    *s ^= *s << 13;
    *s ^= *s >> 7;
    *s ^= *s << 17;
    (*s % n.max(1) as u64) as usize
}

#[derive(Clone, Default)]
struct Mem {
    open: bool,
    category: String,
}

pub fn image_control(ui: &mut Ui, app: &mut AppState, layer: &LayerNode, key: &str, cur: Option<ImageRef>) {
    let lid = layer.id;
    let mem_id = Id::new(("img-picker", lid, key));
    let mut mem: Mem = ui.data(|d| d.get_temp(mem_id).unwrap_or_default());
    let lib = textures::library();
    let dl = textures::download_state();
    let hovering_files = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    let mut budget = 6usize;
    let mut pick: Option<ImageRef> = None;
    let mut shuffle = false;
    let frame = egui::Frame::new().inner_margin(8).stroke(Stroke::new(1.0, if hovering_files { ACCENT } else { separator() })).corner_radius(CornerRadius::same(8));
    let resp = frame.show(ui, |ui| {
        // current image
        ui.horizontal(|ui| {
            let (r, tresp) = ui.allocate_exact_size(vec2(72.0, 50.0), Sense::click());
            let tex = cur.as_ref().and_then(|c| thumb_tex(ui, c, &mut budget));
            paint_thumb(ui, r, tex.as_ref(), mem.open);
            if tresp.on_hover_text(tr("Browse textures")).clicked() {
                mem.open = !mem.open;
            }
            ui.vertical(|ui| {
                let name = cur.as_ref().map(display_name).unwrap_or_else(|| tr("No image"));
                ui.label(egui::RichText::new(name).font(medium(12.0)).color(text()));
                let kind = match &cur {
                    Some(c) if c.data.is_some() => tr("Your image · saved with presets and projects"),
                    Some(_) => tr("Built-in texture"),
                    None => String::new(),
                };
                ui.label(egui::RichText::new(kind).font(regular(10.0)).color(text_faint()));
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if pill_icon_button(ui, "square.grid.2x2", &tr("Textures"), mem.open).clicked() {
                        mem.open = !mem.open;
                    }
                    if pill_icon_button(ui, "folder", &tr("Choose File…"), false).on_hover_text(tr("Use an image of your own (it is saved inside presets and projects)")).clicked() {
                        if let Some(f) = rfd::FileDialog::new().add_filter(&tr("Images"), &crate::imageio::IMAGE_EXTS).pick_file() {
                            match textures::user_image_from_file(&f) {
                                Ok(r) => pick = Some(r),
                                Err(e) => app.alert(&tr("This image could not be opened."), &e),
                            }
                        }
                    }
                    if icon_button(ui, "dice", &tr("Shuffle: another texture of the same kind, flipped or turned"), false, 13.0).clicked() {
                        shuffle = true;
                    }
                });
            });
        });
        // the library
        if lib.is_none() {
            ui.add_space(6.0);
            match &dl {
                Download::Running { done, total } => {
                    let mb = |b: u64| b as f64 / 1_000_000.0;
                    let total_mb = if *total > 0 { mb(*total) } else { textures::PACK_APPROX_MB as f64 };
                    ui.add(egui::ProgressBar::new(if *total > 0 { *done as f32 / *total as f32 } else { 0.0 }).desired_height(6.0));
                    ui.label(
                        egui::RichText::new(trf("Downloading the texture library… %@ of %@ MB", &[&format!("{:.0}", mb(*done)), &format!("{total_mb:.0}")]))
                            .font(regular(10.5))
                            .color(text_dim()),
                    );
                }
                Download::Unpacking => {
                    ui.label(egui::RichText::new(tr("Installing the texture library…")).font(regular(10.5)).color(text_dim()));
                }
                Download::Failed(e) => {
                    ui.label(egui::RichText::new(format!("{} ({e})", tr("The texture library could not be downloaded"))).font(regular(10.5)).color(text_dim()));
                    if pill_button(ui, &tr("Try Again"), true).clicked() {
                        textures::ensure_library(true);
                    }
                }
                Download::Idle => {
                    ui.label(
                        egui::RichText::new(trf("The built-in textures are downloaded once (about %d MB).", &[&textures::PACK_APPROX_MB]))
                            .font(regular(10.5))
                            .color(text_dim()),
                    );
                    if pill_button(ui, &tr("Download Textures"), true).clicked() {
                        textures::ensure_library(true);
                    }
                }
            }
        } else if let (true, Some(lib)) = (mem.open, &lib) {
            ui.add_space(6.0);
            if mem.category.is_empty() || !lib.iter().any(|c| c.id == mem.category) {
                mem.category = cur.as_ref().and_then(category_of).filter(|c| lib.iter().any(|x| x.id == *c)).unwrap_or_else(|| lib[0].id.clone());
            }
            // categories
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for c in lib.iter() {
                    let on = c.id == mem.category;
                    if pill_button(ui, &format!("{} {}", tr(&c.name), c.items.len()), on).clicked() {
                        mem.category = c.id.clone();
                    }
                }
            });
            ui.add_space(6.0);
            let Some(cat) = lib.iter().find(|c| c.id == mem.category) else { return };
            let w = ui.available_width();
            let cols = ((w + 6.0) / (64.0 + 6.0)).floor().max(2.0) as usize;
            let cw = (w - (cols - 1) as f32 * 6.0) / cols as f32;
            egui::ScrollArea::vertical().id_salt(("img-grid", lid, &mem.category)).max_height(260.0).auto_shrink([false, true]).show(ui, |ui| {
                for row in cat.items.chunks(cols) {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        for it in row {
                            let r = ImageRef::builtin(&it.id, &it.name);
                            let (rect, resp) = ui.allocate_exact_size(vec2(cw, cw * 0.72), Sense::click());
                            let sel = cur.as_ref().map(|c| c.id == r.id).unwrap_or(false);
                            let tex = if ui.is_rect_visible(rect) { thumb_tex(ui, &r, &mut budget) } else { None };
                            paint_thumb(ui, rect, tex.as_ref(), sel || resp.hovered());
                            if resp.on_hover_text(display_name(&r)).clicked() {
                                pick = Some(r);
                            }
                        }
                    });
                    ui.add_space(6.0);
                }
            });
        }
        ui.label(
            egui::RichText::new(if hovering_files { tr("Drop to use this image") } else { tr("Drop an image here to use your own") })
                .font(regular(10.0))
                .color(if hovering_files { accent_text() } else { text_faint() }),
        );
    });
    // files dropped on this control
    let rect = resp.response.rect;
    let dropped: Vec<std::path::PathBuf> = ui.ctx().input(|i| {
        let over = i.pointer.hover_pos().map(|p| rect.contains(p)).unwrap_or(false);
        if over { i.raw.dropped_files.iter().map(|f| f.path().to_path_buf()).collect() } else { vec![] }
    });
    if let Some(f) = dropped.first() {
        match textures::user_image_from_file(f) {
            Ok(r) => pick = Some(r),
            Err(e) => app.alert(&tr("This image could not be opened."), &e),
        }
        ui.ctx().input_mut(|i| i.raw.dropped_files.clear());
    }
    if shuffle {
        do_shuffle(app, lid, key, cur.as_ref(), lib.as_deref().map(|v| v.as_slice()));
    }
    if let Some(r) = pick {
        app.set_param(lid, key, ParamValue::Image(r), false);
    }
    ui.data_mut(|d| d.insert_temp(mem_id, mem));
}

/// Another texture of the same category (any category for the user's images), randomly flipped
/// and turned 0° / 180°, as one undo step.
fn do_shuffle(app: &mut AppState, lid: Uuid, key: &str, cur: Option<&ImageRef>, lib: Option<&[textures::Category]>) {
    let Some(lib) = lib else {
        textures::ensure_library(true);
        return;
    };
    let cat = cur.and_then(category_of);
    let pool: Vec<&textures::Item> = match lib.iter().find(|c| Some(&c.id) == cat.as_ref()) {
        Some(c) => c.items.iter().collect(),
        None => lib.iter().flat_map(|c| c.items.iter()).collect(),
    };
    let pool: Vec<&textures::Item> = pool.into_iter().filter(|i| cur.map(|c| c.builtin_id() != Some(i.id.as_str())).unwrap_or(true)).collect();
    if pool.is_empty() {
        return;
    }
    let it = pool[random(pool.len())];
    let r = ImageRef::builtin(&it.id, &it.name);
    let (fh, fv, rot) = (random(2) == 1, random(2) == 1, if random(2) == 1 { 180.0 } else { 0.0 });
    let key = key.to_string();
    app.update_layer(lid, None, |n| {
        n.params.set(&key, ParamValue::Image(r.clone()));
        n.params.set("flipH", ParamValue::Bool(fh));
        n.params.set("flipV", ParamValue::Bool(fv));
        n.params.set("rotation", ParamValue::Number(rot));
    });
}
