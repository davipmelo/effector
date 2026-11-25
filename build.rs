// Embeds every compute kernel in src/shaders/kernels into the binary (see gpu::kernel_source).
use std::{env, fs, path::Path};

fn main() {
    let dir = Path::new("src/shaders/kernels");
    println!("cargo:rerun-if-changed=src/shaders");
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|n| n.ends_with(".glsl"))
        .collect();
    names.sort();
    let mut out = String::from("pub const KERNELS: &[(&str, &str)] = &[\n");
    for n in &names {
        let stem = n.trim_end_matches(".glsl");
        let abs = fs::canonicalize(dir.join(n)).unwrap();
        out += &format!("    ({:?}, include_str!({:?})),\n", stem, abs.to_string_lossy().trim_start_matches(r"\?\"));
    }
    out += "];\n";
    fs::write(Path::new(&env::var("OUT_DIR").unwrap()).join("kernels.rs"), out).unwrap();
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");
    }
}
