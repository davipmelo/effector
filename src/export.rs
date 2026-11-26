//! Export settings (Lightroom-style): where files go, how they are named, format, size limit,
//! resizing and what happens after. Also export presets and the background export job.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::exif::DateTaken;
use crate::imageio::{self, Format, SourceImage};
use crate::l10n::{tr, trf};
use crate::model::DocumentState;

// MARK: - Settings

/// One block of the file name template.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum NameToken {
    /// Original file name (without extension).
    Filename,
    /// The "Custom Text" field of the dialog.
    Custom,
    /// Literal text typed into the template (separators, words).
    Text(String),
    /// Sequence number with this many digits (1 = "1", 3 = "001").
    Sequence(u8),
    /// Export date, in one of `DATE_FORMATS`.
    Date(String),
    /// Capture date from the photo's EXIF (falls back to the export date), in one of `DATE_FORMATS`.
    CaptureDate(String),
    /// Output size, "4000x3000".
    Dimensions,
}

pub const DATE_FORMATS: [(&str, &str); 7] = [
    ("YYYY-MM-DD", "2026-11-26"),
    ("YYYYMMDD", "20261126"),
    ("DD-MM-YYYY", "26-11-2026"),
    ("YYYY", "2026"),
    ("MM", "11"),
    ("DD", "26"),
    ("HHMMSS", "143005"),
];

impl NameToken {
    /// Short label of the block in the template editor.
    pub fn label(&self) -> String {
        match self {
            NameToken::Filename => tr("Filename"),
            NameToken::Custom => tr("Custom Text"),
            NameToken::Text(t) => {
                if t == " " {
                    "␣".into()
                } else {
                    t.clone()
                }
            }
            NameToken::Sequence(d) => format!("{} {}", tr("Sequence"), "0".repeat((*d as usize).saturating_sub(1)) + "1"),
            NameToken::Date(f) => format!("{} {}", tr("Date"), f),
            NameToken::CaptureDate(f) => format!("{} {}", tr("Capture Date"), f),
            NameToken::Dimensions => tr("Dimensions"),
        }
    }
    pub fn is_text(&self) -> bool { matches!(self, NameToken::Text(_)) }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Location {
    /// A folder chosen in the dialog.
    #[default]
    Folder,
    /// Next to the original photo (or the project).
    SameAsOriginal,
    /// Ask for a folder each time.
    Ask,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Existing {
    /// Ask what to do when files already exist.
    #[default]
    Ask,
    /// Add "-2", "-3"… to the new file.
    Rename,
    Overwrite,
    Skip,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ResizeMode {
    #[default]
    WidthHeight,
    Dimensions,
    LongEdge,
    ShortEdge,
    Megapixels,
    Percentage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum Units {
    #[default]
    Pixels,
    Inches,
    Centimeters,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum After {
    #[default]
    Nothing,
    Reveal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct ExportSettings {
    pub location: Location,
    pub folder: String,
    pub use_subfolder: bool,
    pub subfolder: String,
    pub existing: Existing,
    pub template: Vec<NameToken>,
    pub custom_text: String,
    pub start_number: u32,
    pub format: Format,
    /// 0..1 (JPG)
    pub quality: f64,
    pub limit_size: bool,
    pub limit_kb: u32,
    pub resize: bool,
    pub resize_mode: ResizeMode,
    pub width: f64,
    pub height: f64,
    pub long_edge: f64,
    pub short_edge: f64,
    pub megapixels: f64,
    pub percent: f64,
    pub units: Units,
    pub ppi: f64,
    pub dont_enlarge: bool,
    pub after: After,
}

impl Default for ExportSettings {
    fn default() -> Self {
        ExportSettings {
            location: Location::Ask,
            folder: dirs::picture_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            use_subfolder: false,
            subfolder: "EFFECTOR".into(),
            existing: Existing::Ask,
            template: vec![NameToken::Filename, NameToken::Text("-".into()), NameToken::Text("edit".into())],
            custom_text: String::new(),
            start_number: 1,
            format: Format::Jpg,
            quality: 0.92,
            limit_size: false,
            limit_kb: 2000,
            resize: false,
            resize_mode: ResizeMode::LongEdge,
            width: 2048.0,
            height: 2048.0,
            long_edge: 2048.0,
            short_edge: 1080.0,
            megapixels: 12.0,
            percent: 100.0,
            units: Units::Pixels,
            ppi: 300.0,
            dont_enlarge: true,
            after: After::Nothing,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPreset {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub name: String,
    pub settings: ExportSettings,
}

/// Ready-made templates of the File Naming dropdown.
pub fn template_presets() -> Vec<(&'static str, Vec<NameToken>)> {
    use NameToken::*;
    let t = |s: &str| Text(s.into());
    vec![
        ("Filename", vec![Filename]),
        ("Filename - edit", vec![Filename, t("-"), t("edit")]),
        ("Filename - Sequence", vec![Filename, t("-"), Sequence(3)]),
        ("Custom Text - Sequence", vec![Custom, t("-"), Sequence(3)]),
        ("Custom Text", vec![Custom]),
        ("Date - Filename", vec![Date("YYYY-MM-DD".into()), t("_"), Filename]),
        ("Capture Date - Filename", vec![CaptureDate("YYYY-MM-DD".into()), t("_"), Filename]),
        ("Capture Date - Sequence", vec![CaptureDate("YYYYMMDD".into()), t("-"), Sequence(4)]),
        ("Filename - Dimensions", vec![Filename, t("_"), Dimensions]),
    ]
}

// MARK: - Names

/// Local date and time (the export date in file names).
pub fn local_now() -> DateTaken {
    #[cfg(windows)]
    {
        let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        return DateTaken { year: t.wYear as i32, month: t.wMonth as u32, day: t.wDay as u32, hour: t.wHour as u32, minute: t.wMinute as u32, second: t.wSecond as u32 };
    }
    #[cfg(not(windows))]
    {
        if let Ok(o) = std::process::Command::new("date").arg("+%Y %m %d %H %M %S").output() {
            let v: Vec<u32> = String::from_utf8_lossy(&o.stdout).split_whitespace().filter_map(|x| x.parse().ok()).collect();
            if v.len() == 6 {
                return DateTaken { year: v[0] as i32, month: v[1], day: v[2], hour: v[3], minute: v[4], second: v[5] };
            }
        }
        DateTaken::now()
    }
}

fn fmt_date(d: &DateTaken, f: &str) -> String {
    match f {
        "YYYYMMDD" => format!("{:04}{:02}{:02}", d.year, d.month, d.day),
        "DD-MM-YYYY" => format!("{:02}-{:02}-{:04}", d.day, d.month, d.year),
        "YYYY" => format!("{:04}", d.year),
        "MM" => format!("{:02}", d.month),
        "DD" => format!("{:02}", d.day),
        "HHMMSS" => format!("{:02}{:02}{:02}", d.hour, d.minute, d.second),
        _ => format!("{:04}-{:02}-{:02}", d.year, d.month, d.day),
    }
}

/// What a file name is built from.
pub struct NameInput<'a> {
    pub stem: &'a str,
    pub index: usize,
    pub size: (usize, usize),
    pub taken: Option<DateTaken>,
    pub now: DateTaken,
}

/// Characters Windows and macOS don't allow in file names.
fn clean(s: &str) -> String {
    let c: String = s.chars().map(|c| if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || c.is_control() { '-' } else { c }).collect();
    c.trim().trim_end_matches('.').to_string()
}

pub fn file_name(s: &ExportSettings, inp: &NameInput) -> String {
    let mut out = String::new();
    for t in &s.template {
        match t {
            NameToken::Filename => out += inp.stem,
            NameToken::Custom => out += &s.custom_text,
            NameToken::Text(x) => out += x,
            NameToken::Sequence(d) => out += &format!("{:0width$}", s.start_number as usize + inp.index, width = (*d).max(1) as usize),
            NameToken::Date(f) => out += &fmt_date(&inp.now, f),
            NameToken::CaptureDate(f) => out += &fmt_date(&inp.taken.unwrap_or(inp.now), f),
            NameToken::Dimensions => out += &format!("{}x{}", inp.size.0, inp.size.1),
        }
    }
    let n = clean(&out);
    if !n.is_empty() {
        return n;
    }
    let c = clean(inp.stem);
    if c.is_empty() { "image".into() } else { c }
}

// MARK: - Size

impl ExportSettings {
    fn to_px(&self, v: f64) -> f64 {
        match self.units {
            Units::Pixels => v,
            Units::Inches => v * self.ppi,
            Units::Centimeters => v / 2.54 * self.ppi,
        }
    }

    /// Scale applied to an image of `w`×`h` px (1 = full resolution).
    pub fn scale_for(&self, w: usize, h: usize) -> f64 {
        if !self.resize || w == 0 || h == 0 {
            return 1.0;
        }
        let (w, h) = (w as f64, h as f64);
        let s = match self.resize_mode {
            ResizeMode::WidthHeight => {
                let (tw, th) = (self.to_px(self.width), self.to_px(self.height));
                let sw = if tw > 0.0 { tw / w } else { f64::INFINITY };
                let sh = if th > 0.0 { th / h } else { f64::INFINITY };
                sw.min(sh)
            }
            ResizeMode::Dimensions => {
                let (a, b) = (self.to_px(self.width), self.to_px(self.height));
                let (big, small) = (a.max(b), a.min(b));
                (big / w.max(h)).min(small / w.min(h))
            }
            ResizeMode::LongEdge => self.to_px(self.long_edge) / w.max(h),
            ResizeMode::ShortEdge => self.to_px(self.short_edge) / w.min(h),
            ResizeMode::Megapixels => (self.megapixels.max(0.01) * 1e6 / (w * h)).sqrt(),
            ResizeMode::Percentage => self.percent / 100.0,
        };
        let s = if s.is_finite() && s > 0.0 { s } else { 1.0 };
        let s = if self.dont_enlarge { s.min(1.0) } else { s };
        s.clamp(0.01, 8.0)
    }

    pub fn out_size(&self, w: usize, h: usize) -> (usize, usize) {
        let s = self.scale_for(w, h);
        (((w as f64 * s).round() as usize).max(1), ((h as f64 * s).round() as usize).max(1))
    }

    pub fn uses(&self, f: impl Fn(&NameToken) -> bool) -> bool { self.template.iter().any(f) }
}

// MARK: - Jobs

/// One image to export.
pub struct Job {
    pub name: String,
    pub data: Arc<Vec<u8>>,
    pub source: Option<Arc<SourceImage>>,
    pub state: DocumentState,
    pub file_path: Option<PathBuf>,
    pub project_path: Option<PathBuf>,
    /// Output size at full resolution (crop + canvas), for names and sizing; (0, 0) when the
    /// image is not loaded yet (then it is measured while exporting).
    pub full_size: (usize, usize),
    pub taken: Option<DateTaken>,
}

impl Job {
    pub fn stem(&self) -> String {
        self.project_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .or_else(|| Path::new(&self.name).file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "image".into())
    }
    /// Folder of the original photo (or the project).
    pub fn original_dir(&self) -> Option<PathBuf> {
        self.file_path.as_ref().or(self.project_path.as_ref()).and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }
}

/// Output size at full resolution: the cropped photo, plus the canvas borders when on.
pub fn full_size(src: &SourceImage, st: &DocumentState) -> (usize, usize) {
    let (w, h) = src.cropped_size(&st.crop);
    match st.active_canvas() {
        None => (w, h),
        Some(c) => {
            let (s, _) = c.layout(w as f64, h as f64);
            (s.0 as usize, s.1 as usize)
        }
    }
}

/// A planned output file.
pub struct Planned {
    pub job: usize,
    pub path: PathBuf,
    pub exists: bool,
}

/// Where each image goes. `ask_dir` is the folder picked for "Choose folder later". Errors when a
/// folder is missing.
pub fn plan(s: &ExportSettings, jobs: &[Job], ask_dir: Option<&Path>) -> Result<Vec<Planned>, String> {
    let now = local_now();
    let mut out: Vec<Planned> = vec![];
    for (i, j) in jobs.iter().enumerate() {
        let base = match s.location {
            Location::Folder => {
                if s.folder.trim().is_empty() {
                    return Err(tr("Choose the export folder"));
                }
                PathBuf::from(&s.folder)
            }
            Location::SameAsOriginal => match j.original_dir().or_else(|| ask_dir.map(|d| d.to_path_buf())) {
                Some(d) => d,
                None => return Err(trf("%@ has no folder of its own (pasted image). Choose another location.", &[&j.name])),
            },
            Location::Ask => ask_dir.map(|d| d.to_path_buf()).ok_or_else(|| tr("Choose the export folder"))?,
        };
        let dir = if s.use_subfolder && !s.subfolder.trim().is_empty() { base.join(clean(&s.subfolder)) } else { base };
        let size = s.out_size(j.full_size.0, j.full_size.1);
        let stem = j.stem();
        let name = file_name(s, &NameInput { stem: &stem, index: i, size, taken: j.taken, now });
        let mut path = dir.join(format!("{name}.{}", s.format.ext()));
        // two images of this export with the same name never overwrite each other
        let mut k = 2;
        while out.iter().any(|p| p.path == path) {
            path = dir.join(format!("{name}-{k}.{}", s.format.ext()));
            k += 1;
        }
        let exists = path.exists();
        out.push(Planned { job: i, path, exists });
    }
    Ok(out)
}

/// A free name next to `p` ("photo-2.jpg", "photo-3.jpg"…).
fn free_name(p: &Path, taken: &[PathBuf]) -> PathBuf {
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext = p.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let dir = p.parent().map(|d| d.to_path_buf()).unwrap_or_default();
    (2..).map(|k| dir.join(format!("{stem}-{k}.{ext}"))).find(|c| !c.exists() && !taken.contains(c)).unwrap_or_else(|| p.to_path_buf())
}

/// JPG bytes that fit in `limit` bytes (highest quality that fits, never above `max_q`).
fn jpg_within(rgba: &[u8], w: usize, h: usize, max_q: f64, limit: usize) -> Result<Vec<u8>, String> {
    let first = imageio::encode(rgba, w, h, Format::Jpg, max_q, false)?;
    if first.len() <= limit {
        return Ok(first);
    }
    let (mut lo, mut hi) = (0.05f64, max_q);
    let mut best: Option<Vec<u8>> = None;
    for _ in 0..8 {
        let q = (lo + hi) / 2.0;
        let b = imageio::encode(rgba, w, h, Format::Jpg, q, false)?;
        if b.len() <= limit {
            lo = q;
            best = Some(b);
        } else {
            hi = q;
        }
    }
    best.map(Ok).unwrap_or_else(|| imageio::encode(rgba, w, h, Format::Jpg, 0.05, false))
}

/// Renders and writes every planned file (on a background thread). `progress` counts done files.
pub fn run(s: &ExportSettings, jobs: Vec<Job>, planned: Vec<Planned>, existing: Existing, progress: Arc<std::sync::atomic::AtomicUsize>) -> Result<String, String> {
    let mut failed: Vec<String> = vec![];
    let mut written: Vec<PathBuf> = vec![];
    let mut skipped = 0;
    let now = local_now();
    for p in planned {
        let j = &jobs[p.job];
        let src = match &j.source {
            Some(s) => s.clone(),
            None => match SourceImage::load_data(j.data.clone(), &j.name) {
                Ok(s) => Arc::new(s),
                Err(e) => {
                    failed.push(format!("{}: {e}", j.name));
                    progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    continue;
                }
            },
        };
        let full = if j.full_size.0 == 0 { full_size(&src, &j.state) } else { j.full_size };
        let mut path = p.path.clone();
        if j.full_size.0 == 0 && s.uses(|t| *t == NameToken::Dimensions) {
            // the size was not known when planning
            let stem = j.stem();
            let name = file_name(s, &NameInput { stem: &stem, index: p.job, size: s.out_size(full.0, full.1), taken: j.taken, now });
            path = path.with_file_name(format!("{name}.{}", s.format.ext()));
        }
        if path.exists() {
            match existing {
                Existing::Skip => {
                    skipped += 1;
                    progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    continue;
                }
                // "Ask" was answered before the export started; files that appear later are kept
                Existing::Rename | Existing::Ask => path = free_name(&path, &written),
                Existing::Overwrite => {}
            }
        }
        let r = (|| -> Result<(), String> {
            let st = &j.state;
            let scale = s.scale_for(full.0, full.1);
            let tex = src.make_texture(&st.crop, scale);
            let layers = st.render_layers();
            let extras = crate::renderer::Extras::for_render(&src, &st.crop, &layers, true);
            let out = crate::renderer::Renderer::new(false).render(&layers, &tex, crate::worker::content_key(src.id, &st.crop), true, st.active_canvas(), extras);
            let keep_alpha = src.has_alpha && st.active_canvas().is_none();
            let rgba = imageio::read_rgba8(&out);
            let (w, h) = (out.width(), out.height());
            let bytes = if s.format == Format::Jpg && s.limit_size {
                jpg_within(&rgba, w, h, s.quality, s.limit_kb.max(10) as usize * 1000)?
            } else {
                imageio::encode(&rgba, w, h, s.format, s.quality, keep_alpha)?
            };
            if let Some(d) = path.parent() {
                std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
            }
            std::fs::write(&path, bytes).map_err(|e| e.to_string())
        })();
        match r {
            Ok(()) => written.push(path),
            Err(e) => failed.push(format!("{}: {e}", j.name)),
        }
        progress.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    if s.after == After::Reveal {
        if let Some(first) = written.first() {
            reveal(first);
        }
    }
    if !failed.is_empty() {
        return Err(failed.join("\n"));
    }
    let mut msg = if written.len() == 1 {
        trf("Exported %@", &[&written[0].file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()])
    } else {
        trf("Exported %d images", &[&written.len()])
    };
    if skipped > 0 {
        msg += &format!(" · {}", trf("%d skipped (already existed)", &[&skipped]));
    }
    Ok(msg)
}

/// Shows a file in Explorer / Finder (selected).
pub fn reveal(p: &Path) {
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(format!("/select,{}", p.display())).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg("-R").arg(p).spawn();
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = open::that(p.parent().unwrap_or(p));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inp(stem: &str, index: usize) -> NameInput<'_> {
        NameInput { stem, index, size: (4000, 3000), taken: Some(DateTaken { year: 2024, month: 5, day: 7, hour: 9, minute: 3, second: 1 }), now: DateTaken { year: 2026, month: 11, day: 26, hour: 14, minute: 30, second: 5 } }
    }

    #[test]
    fn names_from_templates() {
        let mut s = ExportSettings::default();
        s.template = vec![NameToken::Filename, NameToken::Text("-".into()), NameToken::Sequence(3)];
        s.start_number = 7;
        assert_eq!(file_name(&s, &inp("IMG_1", 2)), "IMG_1-009");
        s.template = vec![NameToken::CaptureDate("YYYY-MM-DD".into()), NameToken::Text("_".into()), NameToken::Custom];
        s.custom_text = "trip: day/1".into();
        assert_eq!(file_name(&s, &inp("x", 0)), "2024-05-07_trip- day-1");
        s.template = vec![NameToken::Dimensions];
        assert_eq!(file_name(&s, &inp("x", 0)), "4000x3000");
        s.template = vec![];
        assert_eq!(file_name(&s, &inp("photo", 0)), "photo");
    }

    #[test]
    fn sizing() {
        let mut s = ExportSettings::default();
        assert_eq!(s.out_size(4000, 3000), (4000, 3000));
        s.resize = true;
        s.resize_mode = ResizeMode::LongEdge;
        s.long_edge = 2000.0;
        assert_eq!(s.out_size(4000, 3000), (2000, 1500));
        s.resize_mode = ResizeMode::ShortEdge;
        s.short_edge = 1080.0;
        assert_eq!(s.out_size(4000, 3000), (1440, 1080));
        s.resize_mode = ResizeMode::WidthHeight;
        s.width = 1000.0;
        s.height = 1000.0;
        assert_eq!(s.out_size(4000, 3000), (1000, 750));
        s.resize_mode = ResizeMode::Dimensions;
        s.width = 1000.0;
        s.height = 2000.0;
        // a 2000 x 1000 box whatever the orientation
        assert_eq!(s.out_size(4000, 3000), (1333, 1000));
        assert_eq!(s.out_size(3000, 4000), (1000, 1333));
        s.resize_mode = ResizeMode::Percentage;
        s.percent = 50.0;
        assert_eq!(s.out_size(4000, 3000), (2000, 1500));
        s.percent = 200.0;
        assert_eq!(s.out_size(4000, 3000), (4000, 3000));
        s.dont_enlarge = false;
        assert_eq!(s.out_size(4000, 3000), (8000, 6000));
        s.resize_mode = ResizeMode::LongEdge;
        s.units = Units::Inches;
        s.ppi = 100.0;
        s.long_edge = 10.0;
        assert_eq!(s.out_size(4000, 3000), (1000, 750));
    }
}
