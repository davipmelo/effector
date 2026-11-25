//! App settings (language, theme, export defaults…) and the support folder.
//! Stored in the same "EFFECTOR" folder as the macOS app (Application Support on Mac, AppData on Windows).

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::imageio::Format;

pub fn support_dir() -> PathBuf {
    // tests use their own folder so they never touch the user's settings / session
    if let Ok(d) = std::env::var("EFFECTOR_SUPPORT_DIR") {
        let d = PathBuf::from(d);
        let _ = std::fs::create_dir_all(&d);
        return d;
    }
    let base = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    let d = base.join("EFFECTOR");
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Appends a line to `effector.log` in the support folder (errors that would otherwise be lost:
/// the Windows build has no console). The log is restarted when it grows past 1 MB.
pub fn log_error(msg: &str) {
    use std::io::Write;
    eprintln!("{msg}");
    let path = support_dir().join("effector.log");
    if std::fs::metadata(&path).map(|m| m.len() > 1 << 20).unwrap_or(false) {
        let _ = std::fs::remove_file(&path);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = writeln!(f, "[{t}] v{} {msg}", env!("CARGO_PKG_VERSION"));
    }
}

/// Writes through a temp file so a crash never leaves a half-written file.
pub fn write_atomic(path: &Path, data: &[u8]) {
    let tmp = path.with_extension("tmp~");
    if std::fs::write(&tmp, data).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Pt,
    System,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    #[default]
    Dark,
    Light,
    System,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub language: Language,
    pub theme: Theme,
    pub export_format: Format,
    pub export_quality: f64,
    pub export_scale: i32,
    /// Reopen the last image and its layers when the app starts.
    pub reopen_last_session: bool,
    /// Hovering effects / presets previews them on the image.
    pub hover_preview: bool,
    pub show_filmstrip: bool,
    pub factory_presets_visible: bool,
    pub presets_open: bool,
    pub presets_split: f64,
    pub left_width: f64,
    pub right_width: f64,
    pub browser_list: bool,
    pub presets_list: bool,
    /// Favourite items in pickers (keys "picker:id").
    pub favorites: Vec<String>,
    /// Custom shortcuts: action id → shortcut string.
    pub shortcuts: std::collections::BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            language: Language::En,
            theme: Theme::Dark,
            export_format: Format::Png,
            export_quality: 0.92,
            export_scale: 1,
            reopen_last_session: true,
            hover_preview: true,
            show_filmstrip: true,
            factory_presets_visible: true,
            presets_open: false,
            presets_split: 0.5,
            left_width: 270.0,
            right_width: 300.0,
            browser_list: false,
            presets_list: false,
            favorites: vec![],
            shortcuts: Default::default(),
        }
    }
}

fn file() -> PathBuf { support_dir().join("settings.json") }

pub fn settings() -> &'static Mutex<Settings> {
    static S: OnceLock<Mutex<Settings>> = OnceLock::new();
    S.get_or_init(|| {
        let s: Settings = std::fs::read(file()).ok().and_then(|d| serde_json::from_slice(&d).ok()).unwrap_or_default();
        apply_language(s.language);
        Mutex::new(s)
    })
}

pub fn get() -> Settings { settings().lock().unwrap().clone() }

pub fn update(f: impl FnOnce(&mut Settings)) {
    let mut s = settings().lock().unwrap();
    let before = s.language;
    f(&mut s);
    if s.language != before {
        apply_language(s.language);
    }
    if let Ok(d) = serde_json::to_vec_pretty(&*s) {
        write_atomic(&file(), &d);
    }
}

pub fn apply_language(l: Language) {
    crate::l10n::set_portuguese(match l {
        Language::En => false,
        Language::Pt => true,
        Language::System => crate::l10n::system_is_portuguese(),
    });
}
