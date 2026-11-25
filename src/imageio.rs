//! Image loading (JPG, PNG, TIFF, WebP, BMP, GIF, RAW) with EXIF orientation and ICC → sRGB
//! conversion, GPU upload of the cropped source, and PNG/JPG export.

use std::path::Path;
use std::sync::Arc;

use image::{DynamicImage, ImageDecoder};

use crate::gpu::{Ctx, T};
use crate::model::CropRect;

/// Decoded pixels: 8-bit or 16-bit straight-alpha RGBA, sRGB-encoded.
pub enum Pixels {
    Rgba8(Vec<u8>),
    Rgba16(Vec<u16>),
}

/// The original image of a document.
pub struct SourceImage {
    pub pixels: Pixels,
    pub width: usize,
    pub height: usize,
    /// Original file bytes (stored in projects so they stay editable).
    pub data: Arc<Vec<u8>>,
    pub file_name: String,
    pub id: uuid::Uuid,
    pub has_alpha: bool,
    pub kind: String,
}

pub const RAW_EXTS: [&str; 22] = [
    "cr2", "cr3", "crw", "nef", "nrw", "arw", "srf", "sr2", "dng", "raf", "orf", "rw2", "pef", "srw", "x3f", "3fr", "fff", "iiq", "mos",
    "erf", "mef", "kdc",
];
pub const IMAGE_EXTS: [&str; 11] = ["jpg", "jpeg", "png", "tif", "tiff", "webp", "bmp", "gif", "heic", "heif", "avif"];

pub fn is_raw_ext(ext: &str) -> bool { RAW_EXTS.contains(&ext.to_lowercase().as_str()) }
pub fn is_image_ext(ext: &str) -> bool {
    let e = ext.to_lowercase();
    IMAGE_EXTS.contains(&e.as_str()) || is_raw_ext(&e)
}

fn ext_of(name: &str) -> String { Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default() }

impl SourceImage {
    pub fn load_path(path: &Path) -> Result<SourceImage, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_else(|| "Image".into());
        SourceImage::load_data(Arc::new(data), &name)
    }

    pub fn load_data(data: Arc<Vec<u8>>, file_name: &str) -> Result<SourceImage, String> {
        let ext = ext_of(file_name);
        if is_raw_ext(&ext) {
            if let Ok(s) = crate::raw::decode(&data, file_name) {
                return Ok(s);
            }
        }
        match decode_standard(&data) {
            Ok((img, kind)) => Ok(from_dynamic(img, data, file_name, kind)),
            Err(e) => {
                // maybe a RAW without the right extension
                crate::raw::decode(&data, file_name).map_err(|_| e)
            }
        }
    }

    /// Cropped size in original pixels.
    pub fn cropped_size(&self, crop: &CropRect) -> (usize, usize) {
        let r = crop.pixel_rect(self.width, self.height);
        (r.2 as usize, r.3 as usize)
    }

    /// Straight-alpha working texture of the cropped image at `scale`.
    pub fn make_texture(&self, crop: &CropRect, scale: f64) -> T {
        let g = crate::gpu::gpu();
        let (rx, ry, rw, rh) = crop.pixel_rect(self.width, self.height);
        let (x0, y0) = ((rx as usize).min(self.width - 1), (ry as usize).min(self.height - 1));
        let (cw, ch) = ((rw as usize).min(self.width - x0).max(1), (rh as usize).min(self.height - y0).max(1));
        let tw = ((rw * scale).round() as usize).max(1);
        let th = ((rh * scale).round() as usize).max(1);
        // keep the upload within the GPU limit (CPU pre-shrink for giant images)
        let max = g.max_dim as usize;
        let step = cw.max(ch).div_ceil(max).max(1);
        let (uw, uh) = (cw.div_ceil(step), ch.div_ceil(step));
        let up = match &self.pixels {
            Pixels::Rgba8(p) => {
                let mut buf = Vec::with_capacity(uw * uh * 4);
                for y in 0..uh {
                    let sy = y0 + (y * step).min(ch - 1);
                    for x in 0..uw {
                        let sx = x0 + (x * step).min(cw - 1);
                        let i = (sy * self.width + sx) * 4;
                        buf.extend_from_slice(&p[i..i + 4]);
                    }
                }
                g.texture_from_rgba8(&buf, uw, uh)
            }
            Pixels::Rgba16(p) => {
                let mut buf: Vec<f32> = Vec::with_capacity(uw * uh * 4);
                for y in 0..uh {
                    let sy = y0 + (y * step).min(ch - 1);
                    for x in 0..uw {
                        let sx = x0 + (x * step).min(cw - 1);
                        let i = (sy * self.width + sx) * 4;
                        for c in 0..4 {
                            buf.push(p[i + c] as f32 / 65535.0);
                        }
                    }
                }
                let mut ctx = Ctx::new(1.0, g.dummy_tex(), false);
                let t = ctx.upload(&buf, uw, uh);
                ctx.flush();
                t
            }
        };
        let mut ctx = Ctx::new(1.0, up.clone(), false);
        let full = ctx.apply("k_copy", &[&up], &[]);
        let out = ctx.resize(&full, tw, th, false);
        ctx.flush();
        out
    }

    /// Small rgba8 copy (for palette extraction / thumbnails).
    pub fn rgba8_thumbnail(&self, max_side: usize) -> (Vec<u8>, usize, usize) {
        let s = (max_side as f64 / self.width.max(self.height) as f64).min(1.0);
        let (w, h) = (((self.width as f64 * s) as usize).max(1), ((self.height as f64 * s) as usize).max(1));
        let mut out = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            let sy = (y * self.height / h).min(self.height - 1);
            for x in 0..w {
                let sx = (x * self.width / w).min(self.width - 1);
                let i = (sy * self.width + sx) * 4;
                match &self.pixels {
                    Pixels::Rgba8(p) => out.extend_from_slice(&p[i..i + 4]),
                    Pixels::Rgba16(p) => out.extend((0..4).map(|c| (p[i + c] >> 8) as u8)),
                }
            }
        }
        (out, w, h)
    }
}

fn decode_standard(data: &[u8]) -> Result<(DynamicImage, String), String> {
    let reader = image::ImageReader::new(std::io::Cursor::new(data)).with_guessed_format().map_err(|e| e.to_string())?;
    let kind = reader.format().map(|f| format!("{f:?}").to_uppercase()).unwrap_or_else(|| "IMAGE".into());
    let mut dec = reader.into_decoder().map_err(|_| crate::l10n::tr("This image could not be opened."))?;
    let orientation = dec.orientation().unwrap_or(image::metadata::Orientation::NoTransforms);
    let icc = dec.icc_profile().ok().flatten();
    let mut img = DynamicImage::from_decoder(dec).map_err(|_| crate::l10n::tr("This image could not be opened."))?;
    img.apply_orientation(orientation);
    if let Some(icc) = icc {
        img = to_srgb(img, &icc);
    }
    Ok((img, kind.replace("JPEG", "JPEG")))
}

/// Converts pixels tagged with an ICC profile (Display P3, Adobe RGB…) to sRGB.
fn to_srgb(img: DynamicImage, icc: &[u8]) -> DynamicImage {
    let Some(src) = qcms::Profile::new_from_slice(icc, false) else { return img };
    let mut dst = qcms::Profile::new_sRGB();
    dst.precache_output_transform();
    let has_alpha = img.color().has_alpha();
    let mut rgba = img.to_rgba8();
    let Some(xf) = qcms::Transform::new(&src, &dst, qcms::DataType::RGBA8, qcms::Intent::Perceptual) else {
        return DynamicImage::ImageRgba8(rgba);
    };
    xf.apply(&mut rgba);
    let out = DynamicImage::ImageRgba8(rgba);
    if has_alpha { out } else { DynamicImage::ImageRgb8(out.to_rgb8()) }
}

fn from_dynamic(img: DynamicImage, data: Arc<Vec<u8>>, file_name: &str, kind: String) -> SourceImage {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let has_alpha = img.color().has_alpha();
    let sixteen = matches!(
        img,
        DynamicImage::ImageRgb16(_) | DynamicImage::ImageRgba16(_) | DynamicImage::ImageLuma16(_) | DynamicImage::ImageLumaA16(_)
            | DynamicImage::ImageRgb32F(_) | DynamicImage::ImageRgba32F(_)
    );
    let pixels = if sixteen { Pixels::Rgba16(img.to_rgba16().into_raw()) } else { Pixels::Rgba8(img.to_rgba8().into_raw()) };
    SourceImage { pixels, width: w, height: h, data, file_name: file_name.into(), id: uuid::Uuid::new_v4(), has_alpha, kind }
}

pub fn from_rgba8(rgba: Vec<u8>, w: usize, h: usize, file_name: &str) -> SourceImage {
    let has_alpha = rgba.chunks(4).any(|p| p[3] < 255);
    // keep a PNG of the pasted pixels as the "original file"
    let mut png = vec![];
    let _ = image::RgbaImage::from_raw(w as u32, h as u32, rgba.clone())
        .map(|i| DynamicImage::ImageRgba8(i).write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png));
    SourceImage {
        pixels: Pixels::Rgba8(rgba),
        width: w,
        height: h,
        data: Arc::new(png),
        file_name: file_name.into(),
        id: uuid::Uuid::new_v4(),
        has_alpha,
        kind: "PNG".into(),
    }
}

// MARK: - Export

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Png,
    Jpg,
}

impl Format {
    pub fn ext(&self) -> &'static str { match self { Format::Png => "png", Format::Jpg => "jpg" } }
}

/// Straight-alpha rgba8 pixels of a texture.
pub fn read_rgba8(tex: &T) -> Vec<u8> {
    let mut ctx = Ctx::new(1.0, tex.clone(), true);
    let f = ctx.read_floats(tex);
    f.iter().map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8).collect()
}

pub fn encode(rgba: &[u8], w: usize, h: usize, format: Format, quality: f64, keep_alpha: bool) -> Result<Vec<u8>, String> {
    let flatten = |rgba: &[u8]| -> Vec<u8> {
        let mut out = Vec::with_capacity(w * h * 3);
        for p in rgba.chunks(4) {
            let a = p[3] as f32 / 255.0;
            for c in 0..3 {
                out.push((p[c] as f32 * a + 255.0 * (1.0 - a)).round() as u8);
            }
        }
        out
    };
    let mut out = vec![];
    match format {
        Format::Png => {
            let img: DynamicImage = if keep_alpha {
                DynamicImage::ImageRgba8(image::RgbaImage::from_raw(w as u32, h as u32, rgba.to_vec()).ok_or("bad size")?)
            } else {
                DynamicImage::ImageRgb8(image::RgbImage::from_raw(w as u32, h as u32, flatten(rgba)).ok_or("bad size")?)
            };
            img.write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png).map_err(|e| e.to_string())?;
        }
        Format::Jpg => {
            let rgb = flatten(rgba);
            let q = ((quality.clamp(0.0, 1.0) * 100.0).round() as u8).clamp(1, 100);
            let enc = jpeg_encoder::Encoder::new(&mut out, q);
            enc.encode(&rgb, w as u16, h as u16, jpeg_encoder::ColorType::Rgb).map_err(|e| e.to_string())?;
        }
    }
    Ok(out)
}

pub fn write(tex: &T, path: &Path, format: Format, quality: f64, keep_alpha: bool) -> Result<(), String> {
    let rgba = read_rgba8(tex);
    let bytes = encode(&rgba, tex.width(), tex.height(), format, quality, keep_alpha)?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}

pub fn png_data(tex: &T) -> Option<Vec<u8>> {
    let rgba = read_rgba8(tex);
    encode(&rgba, tex.width(), tex.height(), Format::Png, 1.0, true).ok()
}
