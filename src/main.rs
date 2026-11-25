// EFFECTOR — layer-based image effects for macOS and Windows.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod canvas;
mod effects;
mod gpu;
mod imageio;
mod l10n;
mod model;
mod overlay;
mod presets;
mod raw;
mod renderer;
mod rendertest;
mod settings;
mod swatches;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let _ = settings::settings();
    if args.iter().any(|a| a == "--lint") {
        lint_main();
        return;
    }
    if args.iter().any(|a| a == "--dump-params") {
        rendertest::dump_params();
        return;
    }
    if args.iter().any(|a| a == "--render-test") {
        rendertest::run(&args);
        return;
    }
    eprintln!("UI not built yet");
}

fn lint_main() { rendertest::lint(); }
