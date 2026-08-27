#[path = "src/shader_build.rs"]
mod shader_build;

use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    shader_build::compile_shaders(
        &manifest_dir.join("src").join("shaders"),
        &manifest_dir.join("target").join("shaders"),
    );
}
