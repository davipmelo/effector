// EFFECTOR — layer-based image effects for macOS and Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod appicon;
mod canvas;
mod depth;
mod effects;
mod exif;
mod gpu;
mod imageio;
mod l10n;
mod model;
mod overlay;
mod presets;
mod project;
mod raw;
mod renderer;
mod rendertest;
mod settings;
mod shortcuts;
mod swatches;
mod sysdecode;
mod ui;
mod updater;
mod worker;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let _ = settings::settings();
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let thread = std::thread::current().name().unwrap_or("main").to_string();
        settings::log_error(&format!("panic in thread '{thread}': {info}"));
        default_hook(info);
    }));
    if args.iter().any(|a| a == "--lint") {
        rendertest::lint();
        return;
    }
    if args.iter().any(|a| a == "--dump-params") {
        rendertest::dump_params();
        return;
    }
    if args.iter().any(|a| a == "--fuzz") {
        rendertest::fuzz(&args);
        return;
    }
    if args.iter().any(|a| a == "--check-update") {
        updater::check(|| {});
        while matches!(updater::current(), updater::State::Checking) {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        match updater::current() {
            updater::State::Available { version, url, size, .. } => {
                println!("available {version} {url} {size}");
                if args.iter().any(|a| a == "--install") {
                    updater::download(|| {});
                    while matches!(updater::current(), updater::State::Downloading { .. }) {
                        std::thread::sleep(std::time::Duration::from_millis(200));
                    }
                    println!("{:?}", updater::current());
                    println!("install: {:?}", updater::install());
                }
            }
            s => println!("{s:?}"),
        }
        return;
    }
    if args.iter().any(|a| a == "--missing-pt") {
        rendertest::missing_pt();
        return;
    }
    if args.iter().any(|a| a == "--slider-audit") {
        rendertest::slider_audit();
        return;
    }
    if args.iter().any(|a| a == "--list-looks") {
        rendertest::list_looks();
        return;
    }
    if args.iter().any(|a| a == "--depth-view") {
        rendertest::depth_view(&args);
        return;
    }
    if args.iter().any(|a| a == "--render-test") {
        rendertest::run(&args);
        return;
    }
    // packaging helpers: --write-icons <dir> writes icon.ico / icon.icns / icon.png
    if let Some(i) = args.iter().position(|a| a == "--write-icons") {
        let dir = std::path::PathBuf::from(args.get(i + 1).cloned().unwrap_or_else(|| ".".into()));
        let _ = std::fs::create_dir_all(&dir);
        appicon::write_ico(&dir.join("icon.ico")).expect("ico");
        appicon::write_icns(&dir.join("icon.icns")).expect("icns");
        std::fs::write(dir.join("icon.png"), appicon::icon_png(1024)).expect("png");
        println!("icons written to {}", dir.display());
        return;
    }
    if args.iter().any(|a| a == "--version") {
        println!("{}", ui::version_line());
        return;
    }
    // files passed on the command line (double-click in Explorer / Finder "Open with")
    let files: Vec<std::path::PathBuf> = args.iter().skip(1).filter(|a| !a.starts_with("--")).map(std::path::PathBuf::from).filter(|p| p.exists()).collect();
    if let Err(e) = ui::run(files) {
        eprintln!("EFFECTOR could not start: {e}");
    }
}
