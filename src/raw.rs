//! Camera RAW decoding (CR2/CR3, NEF, ARW, DNG, RAF, ORF, RW2…) with the open-source rawler
//! decoder (replaces the macOS-only CIRAWFilter). Output: 16-bit sRGB.

use std::sync::Arc;

use image::DynamicImage;

use crate::imageio::{Pixels, SourceImage};

pub fn decode(data: &Arc<Vec<u8>>, file_name: &str) -> Result<SourceImage, String> {
    let src = rawler::rawsource::RawSource::new_from_shared_vec(data.clone());
    let params = rawler::decoders::RawDecodeParams::default();
    let decoder = rawler::get_decoder(&src).map_err(|e| e.to_string())?;
    let raw = decoder.raw_image(&src, &params, false).map_err(|e| e.to_string())?;
    let orientation = decoder.raw_metadata(&src, &params).ok().and_then(|m| m.exif.orientation).unwrap_or(1);
    let dev = rawler::imgop::develop::RawDevelop::default();
    let img = dev.develop_intermediate(&raw).map_err(|e| e.to_string())?.to_dynamic_image().ok_or("develop failed")?;
    let mut img = DynamicImage::ImageRgba16(img.to_rgba16());
    if let Some(o) = image::metadata::Orientation::from_exif(orientation as u8) {
        img.apply_orientation(o);
    }
    let (w, h) = (img.width() as usize, img.height() as usize);
    Ok(SourceImage {
        pixels: Pixels::Rgba16(img.to_rgba16().into_raw()),
        width: w,
        height: h,
        data: data.clone(),
        file_name: file_name.into(),
        id: uuid::Uuid::new_v4(),
        has_alpha: false,
        kind: "RAW".into(),
    })
}
