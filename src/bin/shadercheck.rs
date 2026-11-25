// Dev tool: compiles every GLSL kernel with naga and reports errors.
include!(concat!(env!("OUT_DIR"), "/kernels.rs"));
const HEADER: &str = include_str!("../shaders/header.glsl");
const PRELUDE: &str = include_str!("../shaders/prelude.glsl");

fn main() {
    let filter = std::env::args().nth(1);
    let mut bad = 0;
    for (name, body) in KERNELS {
        if let Some(f) = &filter { if !name.contains(f.as_str()) { continue; } }
        let src = format!("{HEADER}\n{PRELUDE}\n{body}");
        let mut fe = naga::front::glsl::Frontend::default();
        let opts = naga::front::glsl::Options::from(naga::ShaderStage::Compute);
        match fe.parse(&opts, &src) {
            Ok(m) => {
                let mut v = naga::valid::Validator::new(naga::valid::ValidationFlags::all(), naga::valid::Capabilities::all());
                if let Err(e) = v.validate(&m) {
                    bad += 1;
                    println!("== {name}: validation: {}", e.emit_to_string(&src));
                }
            }
            Err(e) => {
                bad += 1;
                println!("== {name}:\n{}", e.emit_to_string(&src));
            }
        }
    }
    println!("{} kernels, {bad} failed", KERNELS.len());
}
