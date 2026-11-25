//! Formats the bundled decoders don't read (HEIC / HEIF / AVIF…) are decoded with the operating
//! system's own codecs, like the macOS app did: Windows Imaging Component on Windows (needs the
//! free "HEIF Image Extensions" from the Microsoft Store for HEIC), ImageIO via `sips` on macOS.

/// Returns straight rgba8 pixels (orientation already applied when the system does it).
pub fn decode(data: &[u8], file_name: &str) -> Result<(Vec<u8>, usize, usize), String> {
    imp::decode(data, file_name)
}

#[cfg(target_os = "windows")]
mod imp {
    use windows::core::GUID;
    use windows::Win32::Graphics::Imaging::*;
    use windows::Win32::System::Com::*;

    pub fn decode(data: &[u8], _file_name: &str) -> Result<(Vec<u8>, usize, usize), String> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let factory: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).map_err(|e| e.to_string())?;
            let stream = factory.CreateStream().map_err(|e| e.to_string())?;
            stream.InitializeFromMemory(data).map_err(|e| e.to_string())?;
            let decoder = factory
                .CreateDecoderFromStream(&stream, std::ptr::null::<GUID>(), WICDecodeMetadataCacheOnDemand)
                .map_err(|_| crate::l10n::tr("This image could not be opened. For HEIC photos, install the free “HEIF Image Extensions” from the Microsoft Store."))?;
            let frame = decoder.GetFrame(0).map_err(|e| e.to_string())?;
            let conv = factory.CreateFormatConverter().map_err(|e| e.to_string())?;
            conv.Initialize(&frame, &GUID_WICPixelFormat32bppRGBA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom).map_err(|e| e.to_string())?;
            let (mut w, mut h) = (0u32, 0u32);
            conv.GetSize(&mut w, &mut h).map_err(|e| e.to_string())?;
            let mut buf = vec![0u8; (w * h * 4) as usize];
            conv.CopyPixels(std::ptr::null(), w * 4, &mut buf).map_err(|e| e.to_string())?;
            // HEIF stores rotation in the container (irot/imir); the WIC HEIF codec applies it.
            let orient: u16 = 1;
            let img = image::RgbaImage::from_raw(w, h, buf).ok_or("bad image")?;
            let mut dynimg = image::DynamicImage::ImageRgba8(img);
            if let Some(o) = image::metadata::Orientation::from_exif(orient as u8) {
                dynimg.apply_orientation(o);
            }
            let (w, h) = (dynimg.width() as usize, dynimg.height() as usize);
            Ok((dynimg.into_rgba8().into_raw(), w, h))
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    pub fn decode(data: &[u8], file_name: &str) -> Result<(Vec<u8>, usize, usize), String> {
        let dir = std::env::temp_dir().join(format!("effector-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let ext = std::path::Path::new(file_name).extension().map(|e| e.to_string_lossy().to_string()).unwrap_or_else(|| "heic".into());
        let input = dir.join(format!("in.{ext}"));
        let output = dir.join("out.png");
        std::fs::write(&input, data).map_err(|e| e.to_string())?;
        // sips applies the EXIF orientation and the colour profile with ImageIO, like the Swift app
        let st = std::process::Command::new("/usr/bin/sips")
            .args(["-s", "format", "png", "-m", "/System/Library/ColorSync/Profiles/sRGB Profile.icc"])
            .arg(&input)
            .arg("--out")
            .arg(&output)
            .output()
            .map_err(|e| e.to_string())?;
        let r = if st.status.success() {
            image::open(&output).map_err(|e| e.to_string()).map(|i| {
                let (w, h) = (i.width() as usize, i.height() as usize);
                (i.into_rgba8().into_raw(), w, h)
            })
        } else {
            Err(crate::l10n::tr("This image could not be opened."))
        };
        let _ = std::fs::remove_dir_all(&dir);
        r
    }
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
mod imp {
    pub fn decode(_data: &[u8], _file_name: &str) -> Result<(Vec<u8>, usize, usize), String> {
        Err(crate::l10n::tr("This image could not be opened."))
    }
}
