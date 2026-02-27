use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

fn modified_time(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).ok()?.modified().ok()
}

fn needs_rebuild(source: &Path, output: &Path) -> bool {
    let Some(src_time) = modified_time(source) else {
        return false;
    };
    let Some(out_time) = modified_time(output) else {
        return true;
    };
    src_time > out_time
}

fn shader_stage(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()? {
        "vert" => Some("vertex"),
        "frag" => Some("fragment"),
        "comp" => Some("compute"),
        "geom" => Some("geometry"),
        "tesc" => Some("tesscontrol"),
        "tese" => Some("tesseval"),
        _ => None,
    }
}

fn log(msg: &str) {
    if let Ok(mut tty) = File::create("/dev/tty") {
        let _ = writeln!(tty, "{msg}");
    }
}

pub fn prepare_shaders(shader_dir : &Path, shader_intermediary_dir : &Path) {
    let out_dir = PathBuf::from(shader_intermediary_dir);
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let apple = target_os == "macos" || target_os == "ios";
    let windows = target_os == "windows";

    println!("cargo:rerun-if-changed=src/shaders");

    if !shader_dir.exists() {
        return;
    }

    fs::create_dir_all(&out_dir).expect("failed to create target/shader_il");

    let entries = fs::read_dir(shader_dir).expect("failed to read src/shaders");

    for entry in entries {
        let entry = entry.expect("failed to read dir entry");
        let src_path = entry.path();

        if !src_path.is_file() {
            continue;
        }

        let Some(_stage) = shader_stage(&src_path) else {
            continue;
        };

        let stem = src_path.file_stem().unwrap().to_str().unwrap();
        let ext = src_path.extension().unwrap().to_str().unwrap();

        let json_name = format!("{stem}.{ext}.json");
        let json_path = out_dir.join(&json_name);

        if apple {
            // Compile GLSL -> MSL directly with glslcc
            let msl_name = format!("{stem}.{ext}.msl");
            let msl_path = out_dir.join(&msl_name);

            if needs_rebuild(&src_path, &msl_path) {
                let stage_flag = format!("--{}={}", ext, src_path.to_str().unwrap());
                let output_flag = format!("--output={}", msl_path.to_str().unwrap());
                let reflect_flag = format!("--reflect={}", json_path.to_str().unwrap());
                log(&format!("glslcc {} --lang=msl --reflect", src_path.display()));

                let status = Command::new("glslcc")
                    .args([&stage_flag, &output_flag, "--lang=msl", &reflect_flag])
                    .status()
                    .expect("failed to run glslcc — is it installed?");

                if !status.success() {
                    panic!("glslcc failed for {}", src_path.display());
                }
            }
        } else {
            // Compile GLSL -> SPIR-V with glslc
            let spv_name = format!("{stem}.{ext}.spv");
            let spv_path = out_dir.join(&spv_name);

            if needs_rebuild(&src_path, &spv_path) {
                log(&format!("glslc {} -o {}", src_path.display(), spv_path.display()));

                let status = Command::new("glslc")
                    .args([
                        src_path.to_str().unwrap(),
                        "-o",
                        spv_path.to_str().unwrap(),
                    ])
                    .status()
                    .expect("failed to run glslc — is it installed?");

                if !status.success() {
                    panic!("glslc failed for {}", src_path.display());
                }
            }

            // Convert SPIR-V -> DXIL with shadercross (Windows only)
            if windows {
                let dxil_name = format!("{stem}.{ext}.dxil");
                let dxil_path = out_dir.join(&dxil_name);

                if needs_rebuild(&spv_path, &dxil_path) {
                    log(&format!("shadercross {} -o {}", spv_path.display(), dxil_path.display()));

                    let status = Command::new("shadercross")
                        .args([
                            spv_path.to_str().unwrap(),
                            "-o",
                            dxil_path.to_str().unwrap(),
                        ])
                        .status()
                        .expect("failed to run shadercross — is it installed?");

                    if !status.success() {
                        panic!("shadercross failed for {}", spv_path.display());
                    }
                }
            }

            // Generate reflection JSON from SPIR-V with shadercross
            if needs_rebuild(&spv_path, &json_path) {
                log(&format!("shadercross {} -d JSON -o {}", spv_path.display(), json_path.display()));

                let status = Command::new("shadercross")
                    .args([
                        spv_path.to_str().unwrap(),
                        "-d", "JSON",
                        "-o", json_path.to_str().unwrap(),
                    ])
                    .status()
                    .expect("failed to run shadercross — is it installed?");

                if !status.success() {
                    panic!("shadercross (reflect) failed for {}", spv_path.display());
                }
            }
        }
    }
}
