//! In-app updates from GitHub Releases: check the latest release, download the installer for this
//! platform and install it (Windows: silent Inno Setup run that reopens the app; macOS: replace the
//! .app bundle once EFFECTOR has quit, then reopen it).

use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const REPO: &str = "davipmelo/effector";
const AGENT: &str = concat!("EFFECTOR-updater/", env!("CARGO_PKG_VERSION"));

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Idle,
    Checking,
    UpToDate,
    Available { version: String, notes: String, url: String, size: u64, page: String },
    Downloading { version: String, done: u64, total: u64 },
    Ready { version: String, path: PathBuf },
    Installing,
    Failed(String),
}

fn state() -> &'static Mutex<State> {
    static S: OnceLock<Mutex<State>> = OnceLock::new();
    S.get_or_init(|| Mutex::new(State::Idle))
}

pub fn current() -> State { state().lock().unwrap_or_else(|e| e.into_inner()).clone() }
fn set(s: State) { *state().lock().unwrap_or_else(|e| e.into_inner()) = s; }

/// Dev/test (`update` screenshot step): pretends a release is available.
pub fn debug_set_available(version: &str, notes: &str) {
    set(State::Available { version: version.into(), notes: notes.into(), url: String::new(), size: 48_000_000, page: String::new() });
}

/// "0.10.2" → (0, 10, 2)
pub fn parse_version(v: &str) -> (u64, u64, u64) {
    let mut it = v.trim().trim_start_matches('v').split(|c: char| c == '.' || c == '-').map(|x| x.parse::<u64>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0))
}

pub fn is_newer(remote: &str, local: &str) -> bool { parse_version(remote) > parse_version(local) }

fn asset_suffix() -> &'static str {
    if cfg!(target_os = "windows") { "-Windows-Setup.exe" } else { "-macOS.dmg" }
}

/// Checks GitHub in the background; `notify` repaints the UI when done.
pub fn check(notify: impl Fn() + Send + 'static) {
    if matches!(current(), State::Checking | State::Downloading { .. } | State::Installing) {
        return;
    }
    set(State::Checking);
    std::thread::spawn(move || {
        set(match fetch_latest() {
            Ok(s) => s,
            Err(e) => State::Failed(e),
        });
        notify();
    });
}

fn fetch_latest() -> Result<State, String> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let mut resp = ureq::get(&url)
        .header("User-Agent", AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| format!("could not reach GitHub ({e})"))?;
    let v: serde_json::Value = resp.body_mut().read_json().map_err(|e| e.to_string())?;
    let tag = v["tag_name"].as_str().unwrap_or("").to_string();
    if tag.is_empty() {
        return Err("no release found".into());
    }
    // tests can pretend to be an older build
    let local = std::env::var("EFFECTOR_FAKE_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    if !is_newer(&tag, &local) {
        return Ok(State::UpToDate);
    }
    let asset = v["assets"].as_array().and_then(|a| a.iter().find(|x| x["name"].as_str().map(|n| n.ends_with(asset_suffix())).unwrap_or(false)));
    let Some(asset) = asset else { return Err(format!("{tag} has no installer for this system yet")) };
    Ok(State::Available {
        version: tag.trim_start_matches('v').to_string(),
        notes: v["body"].as_str().unwrap_or("").to_string(),
        url: asset["browser_download_url"].as_str().unwrap_or("").to_string(),
        size: asset["size"].as_u64().unwrap_or(0),
        page: v["html_url"].as_str().unwrap_or("").to_string(),
    })
}

/// Downloads the installer of an available update (progress is reported through `current()`).
pub fn download(notify: impl Fn() + Send + Sync + 'static) {
    let State::Available { version, url, size, .. } = current() else { return };
    set(State::Downloading { version: version.clone(), done: 0, total: size });
    std::thread::spawn(move || {
        let r = (|| -> Result<PathBuf, String> {
            let dir = std::env::temp_dir().join("effector-update");
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let name = url.rsplit('/').next().unwrap_or("update").to_string();
            let path = dir.join(&name);
            let mut resp = ureq::get(&url).header("User-Agent", AGENT).call().map_err(|e| format!("download failed ({e})"))?;
            let mut reader = resp.body_mut().as_reader();
            let mut out = std::fs::File::create(&path).map_err(|e| e.to_string())?;
            let mut buf = vec![0u8; 1 << 16];
            let mut done = 0u64;
            let mut last = std::time::Instant::now();
            loop {
                let n = reader.read(&mut buf).map_err(|e| format!("download interrupted ({e})"))?;
                if n == 0 {
                    break;
                }
                std::io::Write::write_all(&mut out, &buf[..n]).map_err(|e| e.to_string())?;
                done += n as u64;
                if last.elapsed().as_millis() > 100 {
                    set(State::Downloading { version: version.clone(), done, total: size });
                    notify();
                    last = std::time::Instant::now();
                }
            }
            if size > 0 && done != size {
                return Err("download incomplete".into());
            }
            Ok(path)
        })();
        set(match r {
            Ok(path) => State::Ready { version, path },
            Err(e) => State::Failed(e),
        });
        notify();
    });
}

/// Starts installing the downloaded update. Returns true when the app should quit now.
pub fn install() -> Result<bool, String> {
    let State::Ready { path, .. } = current() else { return Err("nothing to install".into()) };
    set(State::Installing);
    let r = install_platform(&path);
    if let Err(e) = &r {
        set(State::Failed(e.clone()));
    }
    r
}

#[cfg(target_os = "windows")]
fn install_platform(path: &std::path::Path) -> Result<bool, String> {
    // Inno Setup: silent, closes EFFECTOR if needed, relaunches it at the end (see effector.iss)
    std::process::Command::new(path)
        .args(["/SILENT", "/SP-", "/NOCANCEL", "/CLOSEAPPLICATIONS"])
        .spawn()
        .map_err(|e| format!("could not start the installer ({e})"))?;
    Ok(true)
}

#[cfg(target_os = "macos")]
fn install_platform(path: &std::path::Path) -> Result<bool, String> {
    // the running bundle: …/EFFECTOR.app/Contents/MacOS/effector
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let bundle = exe.parent().and_then(|p| p.parent()).and_then(|p| p.parent()).map(|p| p.to_path_buf());
    let Some(bundle) = bundle.filter(|b| b.extension().map(|e| e == "app").unwrap_or(false)) else {
        // not running from an .app (dev build): just open the disk image
        let _ = std::process::Command::new("open").arg(path).spawn();
        return Ok(false);
    };
    let mount = path.with_extension("mnt");
    let script = format!(
        r#"#!/bin/bash
# EFFECTOR updater: waits for the app to quit, swaps the bundle, reopens it.
while kill -0 {pid} 2>/dev/null; do sleep 0.3; done
hdiutil attach -nobrowse -readonly -mountpoint "{mount}" "{dmg}" >/dev/null || {{ open "{dmg}"; exit 1; }}
NEW="{mount}/EFFECTOR.app"
if [ -d "$NEW" ]; then
  rm -rf "{bundle}.old"
  mv "{bundle}" "{bundle}.old" && ditto "$NEW" "{bundle}" && rm -rf "{bundle}.old" || {{ rm -rf "{bundle}"; mv "{bundle}.old" "{bundle}"; }}
  xattr -dr com.apple.quarantine "{bundle}" 2>/dev/null
fi
hdiutil detach "{mount}" -quiet
{reopen}
"#,
        reopen = if std::env::var("EFFECTOR_UPDATE_NO_REOPEN").is_ok() { String::new() } else { format!("open \"{}\"", bundle.display()) },
        pid = std::process::id(),
        mount = mount.display(),
        dmg = path.display(),
        bundle = bundle.display(),
    );
    let sp = path.with_extension("sh");
    std::fs::write(&sp, script).map_err(|e| e.to_string())?;
    std::process::Command::new("/bin/bash")
        .arg(&sp)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| format!("could not start the updater ({e})"))?;
    Ok(true)
}

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
fn install_platform(path: &std::path::Path) -> Result<bool, String> {
    let _ = open::that(path);
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert!(is_newer("v0.8.0", "0.7.0"));
        assert!(is_newer("v0.10.0", "0.9.5"));
        assert!(!is_newer("v0.8.0", "0.8.0"));
        assert!(!is_newer("v0.7.9", "0.8.0"));
        assert_eq!(parse_version("v1.2.3"), (1, 2, 3));
    }
}
