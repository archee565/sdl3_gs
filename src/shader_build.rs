use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;

const ENABLE_MSL: bool = true;
const ENABLE_DXIL: bool = true;

/// Compile all GLSL shaders found under `shader_dir`, writing preprocessed
/// variants and compiled outputs into `output_dir` (e.g. `target/shaders`).
pub fn compile_shaders(shader_dir: &Path, output_dir: &Path) {
    let pp_dir = output_dir.join("preprocessed");
    let spirv_dir = output_dir.join("obj_spirv");
    let dxil_dir = output_dir.join("obj_dxil");
    let msl_dir = output_dir.join("obj_msl");
    let json_dir = output_dir.join("obj_json");

    println!("cargo:rerun-if-changed={}", shader_dir.display());

    fs::create_dir_all(&spirv_dir).expect("Failed to create obj_spirv directory");
    if ENABLE_DXIL {
        fs::create_dir_all(&dxil_dir).expect("Failed to create obj_dxil directory");
    }
    if ENABLE_MSL {
        fs::create_dir_all(&msl_dir).expect("Failed to create obj_msl directory");
    }
    fs::create_dir_all(&json_dir).expect("Failed to create obj_json directory");

    // Preprocess: split multi-entry shaders into per-entry variants
    let (preprocessed, included_files) = preprocess_shaders(shader_dir, &pp_dir);

    for inc in &included_files {
        println!("cargo:rerun-if-changed={}", inc.display());
    }

    // Collect expected base names (e.g. "mesh_00.frag") from preprocessed files
    let expected_names: Vec<String> = preprocessed
        .iter()
        .map(|(p, _)| format!("{}.{}", p.file_stem().unwrap().to_str().unwrap(), p.extension().unwrap().to_str().unwrap()))
        .collect();

    // Remove stale outputs that no longer correspond to any preprocessed shader
    let mut output_dirs: Vec<(&Path, &str)> = vec![(&spirv_dir, "spv"), (&json_dir, "json")];
    if ENABLE_DXIL {
        output_dirs.push((&dxil_dir, "dxil"));
    }
    if ENABLE_MSL {
        output_dirs.push((&msl_dir, "metal"));
    }
    for &(dir, ext) in &output_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                    if let Some(stripped) = fname.strip_suffix(&format!(".{}", ext)) {
                        if !expected_names.contains(&stripped.to_string()) {
                            let _ = fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }

    let shadercross_available = Command::new("shadercross").arg("--help").output().is_ok();

    let warned_no_shadercross = AtomicBool::new(false);
    let warned_sources = Mutex::new(HashSet::<String>::new());
    preprocessed.par_iter().for_each(|(shader_file, newest_dep)| {
        let stem = shader_file.file_stem().unwrap().to_str().unwrap();
        let ext = shader_file.extension().unwrap().to_str().unwrap();
        let name = format!("{}.{}", stem, ext);
        let spv_path = spirv_dir.join(format!("{}.spv", name));
        let dxil_path = if ENABLE_DXIL {
            Some(dxil_dir.join(format!("{}.dxil", name)))
        } else {
            None
        };
        let msl_path = if ENABLE_MSL {
            Some(msl_dir.join(format!("{}.metal", name)))
        } else {
            None
        };
        let json_path = json_dir.join(format!("{}.json", name));

        let mut outputs: Vec<PathBuf> = vec![spv_path.clone(), json_path.clone()];
        if let Some(ref p) = dxil_path {
            outputs.push(p.clone());
        }
        if let Some(ref p) = msl_path {
            outputs.push(p.clone());
        }
        if outputs_up_to_date(*newest_dep, &outputs) {
            return;
        }

        if !shadercross_available {
            warned_no_shadercross
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
                .ok();
            let mut backends = Vec::new();
            if ENABLE_DXIL {
                backends.push("DXIL");
            }
            if ENABLE_MSL {
                backends.push("MSL");
            }
            backends.push("JSON");
            println!("cargo:warning=shadercross not found - {} conversion skipped", backends.join("/"));
        }

        // Derive source file stem (e.g. "sim" from "sim_00.comp")
        let source_stem = stem.rsplit_once('_').map(|(s, _)| s).unwrap_or(stem);
        if warned_sources.lock().unwrap().insert(source_stem.to_string()) {
            println!("cargo:warning=Converting shader: {}", source_stem);
        }

        compile_glsl_to_spirv(shader_file, &spv_path);

        if shadercross_available {
            let stage = match ext {
                "vert" => "vertex",
                "frag" => "fragment",
                "comp" => "compute",
                _ => "vertex",
            };
            let dxil_dir_ref = if ENABLE_DXIL { Some(dxil_dir.as_path()) } else { None };
            let msl_dir_ref = if ENABLE_MSL { Some(msl_dir.as_path()) } else { None };
            convert_spirv_to_formats(&spv_path, dxil_dir_ref, msl_dir_ref, &json_dir, &name, stage);
        } else {
            // Input changed but we can't regenerate — remove stale outputs
            let _ = fs::remove_file(&json_path);
            if let Some(ref p) = dxil_path {
                let _ = fs::remove_file(p);
            }
            if let Some(ref p) = msl_path {
                let _ = fs::remove_file(p);
            }
        }
    });
}

// -- Shader preprocessing ----------------------------------------------------

/// Remove `//` line comments and `/* … */` block comments from GLSL source.
/// Newlines inside block comments are preserved so line numbers stay valid.
fn strip_comments(source: &str) -> String {
    let b = source.as_bytes();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    let len = b.len();

    while i < len {
        if i + 1 < len && b[i] == b'/' && b[i + 1] == b'/' {
            while i < len && b[i] != b'\n' {
                i += 1;
            }
            if i < len {
                out.push('\n');
                i += 1;
            }
        } else if i + 1 < len && b[i] == b'/' && b[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(b[i] == b'*' && b[i + 1] == b'/') {
                if b[i] == b'\n' {
                    out.push('\n');
                }
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
        } else if b[i] == b'"' {
            out.push('"');
            i += 1;
            while i < len && b[i] != b'"' {
                if b[i] == b'\\' && i + 1 < len {
                    out.push(b[i] as char);
                    out.push(b[i + 1] as char);
                    i += 2;
                } else {
                    out.push(b[i] as char);
                    i += 1;
                }
            }
            if i < len {
                out.push('"');
                i += 1;
            }
        } else {
            out.push(b[i] as char);
            i += 1;
        }
    }
    out
}

/// Recursively resolve `#include "path"` directives in `source`, replacing each
/// with the contents of the referenced file (relative to `shader_dir`).
/// Returns the fully-resolved source and a list of all included file paths.
fn resolve_includes(source: &str, shader_dir: &Path, visited: &mut HashSet<PathBuf>) -> (String, Vec<PathBuf>) {
    let mut out = String::with_capacity(source.len());
    let mut included = Vec::new();

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("#include") {
            let rest = rest.trim();
            if let Some(path_str) = rest.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
                let inc_path = shader_dir.join(path_str);
                if !visited.insert(inc_path.clone()) {
                    continue;
                }
                let inc_source = fs::read_to_string(&inc_path)
                    .unwrap_or_else(|e| panic!("failed to read include {:?}: {}", inc_path, e));
                included.push(inc_path.clone());
                let (resolved, sub) = resolve_includes(&inc_source, shader_dir, visited);
                included.extend(sub);
                out.push_str(&resolved);
                if !resolved.ends_with('\n') {
                    out.push('\n');
                }
            } else {
                out.push_str(line);
                out.push('\n');
            }
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    (out, included)
}

struct EntryPoint {
    name: String,
    start: usize,
    end: usize,
}

/// Scan source for `void comp_XX(`, `void frag_XX(`, `void vert_XX(` definitions,
/// returning name and body span. The prefix (`comp`/`frag`/`vert`) identifies the
/// shader type used when naming the output file.
fn find_entry_points(source: &str) -> Vec<EntryPoint> {
    let mut entries = Vec::new();
    let prefixes = [b"void comp_", b"void frag_", b"void vert_"];
    let bytes = source.as_bytes();
    let tag_len = 10; // length of each prefix (e.g. "void comp_")
    let mut i = 0;

    while i + tag_len + 3 <= bytes.len() {
        for prefix in &prefixes {
            if &bytes[i..i + tag_len] == *prefix {
                let d = &bytes[i + tag_len..i + tag_len + 3];
                if d[0].is_ascii_digit() && d[1].is_ascii_digit() && d[2] == b'(' {
                    let name =
                        format!("{}{}{}", std::str::from_utf8(&prefix[5..]).unwrap(), d[0] as char, d[1] as char);
                    if let Some(br) = source[i..].find('{') {
                        let brace = i + br;
                        let mut depth = 1usize;
                        let mut j = brace + 1;
                        while j < bytes.len() && depth > 0 {
                            match bytes[j] {
                                b'{' => depth += 1,
                                b'}' => depth -= 1,
                                _ => {}
                            }
                            j += 1;
                        }
                        if depth == 0 {
                            entries.push(EntryPoint { name, start: i, end: j });
                            i = j;
                            break;
                        }
                    }
                }
            }
        }
        i += 1;
    }
    entries
}

/// Insert a line after leading `#version` / `#extension` / `#pragma` directives.
fn insert_after_directives(source: &str, line: &str) -> String {
    let mut result = String::with_capacity(source.len() + line.len() + 2);
    let mut past_directives = false;
    for l in source.lines() {
        if !past_directives {
            let trimmed = l.trim();
            if trimmed.is_empty()
                || trimmed.starts_with("#version")
                || trimmed.starts_with("#extension")
                || trimmed.starts_with("#pragma")
            {
                result.push_str(l);
                result.push('\n');
                continue;
            }
            result.push_str(line);
            result.push('\n');
            past_directives = true;
        }
        result.push_str(l);
        result.push('\n');
    }
    if !past_directives {
        result.push_str(line);
        result.push('\n');
    }
    result
}

/// Derive the file extension from an entry point name prefix.
fn entry_ext(name: &str) -> &'static str {
    if name.starts_with("comp_") {
        "comp"
    } else if name.starts_with("frag_") {
        "frag"
    } else if name.starts_with("vert_") {
        "vert"
    } else {
        "comp"
    }
}

/// Return the newest modification time among all given paths.
fn newest_modified(paths: &[PathBuf]) -> Option<SystemTime> {
    let mut newest: Option<SystemTime> = None;
    for p in paths {
        if let Ok(t) = fs::metadata(p).and_then(|m| m.modified()) {
            newest = Some(match newest {
                Some(n) => {
                    if t > n {
                        t
                    } else {
                        n
                    }
                }
                None => t,
            });
        }
    }
    newest
}

/// Build one variant: rename `entry_name` to `main`, strip all other entry points.
fn generate_variant(source: &str, entry_name: &str, entries: &[EntryPoint]) -> String {
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0;

    for ep in entries {
        let line_start = source[..ep.start].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line_end = source[ep.end..].find('\n').map(|p| ep.end + p + 1).unwrap_or(source.len());

        if ep.name == entry_name {
            out.push_str(&source[cursor..ep.start]);
            let body = &source[ep.start..ep.end];
            out.push_str(&body.replace(&format!("void {}(", ep.name), "void main("));
            out.push_str(&source[ep.end..line_end]);
            cursor = line_end;
        } else {
            out.push_str(&source[cursor..line_start]);
            // Replace removed function with blank lines to preserve line numbers
            let nl_count = source[line_start..line_end].matches('\n').count();
            for _ in 0..nl_count {
                out.push('\n');
            }
            cursor = line_end;
        }
    }

    out.push_str(&source[cursor..]);
    out
}

/// Read GLSL shaders from `shader_dir`, split multi-entry files into per-entry
/// variants, and write them into `out_dir/`.
fn preprocess_shaders(shader_dir: &Path, out_dir: &Path) -> (Vec<(PathBuf, SystemTime)>, Vec<PathBuf>) {
    let pp = out_dir;
    fs::create_dir_all(pp).expect("failed to create preprocessed/");

    let mut result = Vec::new();
    let mut expected_preprocessed = Vec::new();
    let mut all_includes = Vec::new();

    for src_path in find_glsl_shaders(shader_dir) {
        let source = fs::read_to_string(&src_path).expect("failed to read shader");
        let mut visited = HashSet::new();
        let (resolved, includes) = resolve_includes(&source, shader_dir, &mut visited);
        all_includes.extend(includes.clone());
        let stripped = strip_comments(&resolved);
        let entries = find_entry_points(&stripped);

        // Newest modification time among the source file and all its recursive
        // includes; used to decide whether downstream outputs need recompilation.
        let mut dep_paths = vec![src_path.clone()];
        dep_paths.extend(includes);
        let newest_dep = newest_modified(&dep_paths).unwrap_or(SystemTime::UNIX_EPOCH);

        let stem = src_path.file_stem().unwrap().to_str().unwrap();

        let src_relative = src_path.file_name().unwrap().to_str().unwrap();
        let header = format!("// AUTO-GENERATED FILE — do not edit manually.\n// Source: {}\n\n", src_relative);

        if entries.is_empty() {
            let dest = pp.join(src_path.file_name().unwrap());
            let content = format!("{}{}", header, stripped);
            if fs::read_to_string(&dest).unwrap_or_default() != content {
                fs::write(&dest, &content).unwrap();
            }
            expected_preprocessed.push(dest.clone());
            result.push((dest, newest_dep));
        } else {
            for ep in &entries {
                let variant = generate_variant(&stripped, &ep.name, &entries);
                let suffix = &ep.name[5..]; // "00" (skip "comp_", "frag_", or "vert_")
                let out_ext = entry_ext(&ep.name);
                let file_name = format!("{}_{}.{}", stem, suffix, out_ext);
                let define = format!("#define {}", ep.name.to_uppercase());
                let dest = pp.join(&file_name);
                let content = format!("{}{}", header, insert_after_directives(&variant, &define));
                if fs::read_to_string(&dest).unwrap_or_default() != content {
                    fs::write(&dest, &content).unwrap();
                }
                expected_preprocessed.push(dest.clone());
                result.push((dest, newest_dep));
            }
        }
    }

    // Remove stale preprocessed files that no longer correspond to any source
    if let Ok(entries) = fs::read_dir(&pp) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() && !expected_preprocessed.contains(&path) {
                let _ = fs::remove_file(&path);
            }
        }
    }

    (result, all_includes)
}

fn find_glsl_shaders(dir: &Path) -> Vec<PathBuf> {
    let mut shaders = Vec::new();
    let extensions = ["vert", "frag", "comp", "geom", "tesc", "tese"];

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(&ext) {
                        shaders.push(path);
                    }
                }
            }
        }
    }

    shaders.sort();
    shaders
}

fn outputs_up_to_date(newest_input: SystemTime, outputs: &[PathBuf]) -> bool {
    for output in outputs {
        let output_time = match fs::metadata(output).and_then(|m| m.modified()) {
            Ok(time) => time,
            Err(_) => return false,
        };

        if output_time <= newest_input {
            return false;
        }
    }

    true
}

fn compile_glsl_to_spirv(input: &Path, output: &Path) {
    let stem = input.file_stem().unwrap().to_str().unwrap();

    let result = Command::new("glslc")
        .arg(input)
        .arg("-o")
        .arg(output)
        .arg("--target-env=vulkan1.0")
        .output();

    if let Ok(output_data) = result {
        if output_data.status.success() {
            println!("cargo:rerun-if-changed={}", input.display());
            println!("cargo:rerun-if-changed={}", output.display());
            return;
        }
        let stderr = String::from_utf8_lossy(&output_data.stderr);
        let stdout = String::from_utf8_lossy(&output_data.stdout);
        eprintln!("glslc failed for {}:\nstdout: {}\nstderr: {}", stem, stdout, stderr);
    }

    let result = Command::new("glslcc")
        .arg("--input")
        .arg(input)
        .arg("--output")
        .arg(output)
        .arg("--profile")
        .arg("spirv")
        .output();

    match result {
        Ok(output_data) => {
            if output_data.status.success() {
                println!("cargo:warning=  -> SPIR-V");
                println!("cargo:rerun-if-changed={}", input.display());
                println!("cargo:rerun-if-changed={}", output.display());
                return;
            }
            let stderr = String::from_utf8_lossy(&output_data.stderr);
            let stdout = String::from_utf8_lossy(&output_data.stdout);
            eprintln!("glslcc failed for {}:\nstdout: {}\nstderr: {}", stem, stdout, stderr);
            panic!("Failed to compile shader {}", stem);
        }
        Err(e) => {
            eprintln!("Failed to execute glslcc: {}", e);
            panic!("glslc and glslcc not found. Please install Vulkan SDK for glslc or glslcc.");
        }
    }
}

fn convert_spirv_to_formats(
    spv_path: &Path,
    dxil_dir: Option<&Path>,
    msl_dir: Option<&Path>,
    json_dir: &Path,
    name: &str,
    stage: &str,
) {
    let dxil_path = dxil_dir.map(|d| d.join(format!("{}.dxil", name)));
    let msl_path = msl_dir.map(|d| d.join(format!("{}.metal", name)));
    let json_path = json_dir.join(format!("{}.json", name));

    // DXIL
    if let Some(ref dxil_path) = dxil_path {
        let result = Command::new("shadercross")
            .arg(spv_path)
            .arg("-s")
            .arg("SPIRV")
            .arg("-d")
            .arg("DXIL")
            .arg("-t")
            .arg(stage)
            .arg("-o")
            .arg(dxil_path)
            .output();
        match result {
            Ok(o) if o.status.success() => {
                println!("cargo:rerun-if-changed={}", dxil_path.display());
            }
            Ok(o) => {
                eprintln!(
                    "DXIL conversion failed for {}:\nstdout: {}\nstderr: {}",
                    name,
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                );
                panic!("Failed to convert {} to DXIL", name);
            }
            Err(e) => {
                eprintln!("Failed to run shadercross for DXIL: {}", e);
                panic!("Failed to convert {} to DXIL", name);
            }
        }
    }

    // MSL
    if let Some(ref msl_path) = msl_path {
        let result = Command::new("shadercross")
            .arg(spv_path)
            .arg("-s")
            .arg("SPIRV")
            .arg("-d")
            .arg("MSL")
            .arg("-t")
            .arg(stage)
            .arg("-o")
            .arg(msl_path)
            .output();
        match result {
            Ok(o) if o.status.success() => {
                println!("cargo:rerun-if-changed={}", msl_path.display());
            }
            Ok(o) => {
                eprintln!(
                    "MSL conversion failed for {}:\nstdout: {}\nstderr: {}",
                    name,
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                );
                panic!("Failed to convert {} to MSL", name);
            }
            Err(e) => {
                eprintln!("Failed to run shadercross for MSL: {}", e);
                panic!("Failed to convert {} to MSL", name);
            }
        }
    }

    // JSON reflection
    let result = Command::new("shadercross")
        .arg(spv_path)
        .arg("-s")
        .arg("SPIRV")
        .arg("-d")
        .arg("JSON")
        .arg("-t")
        .arg(stage)
        .arg("-o")
        .arg(&json_path)
        .output();
    match result {
        Ok(o) if o.status.success() => {
            println!("cargo:rerun-if-changed={}", json_path.display());
        }
        Ok(o) => {
            eprintln!(
                "JSON reflection failed for {}:\nstdout: {}\nstderr: {}",
                name,
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            panic!("Failed to generate reflection JSON for {}", name);
        }
        Err(e) => {
            eprintln!("Failed to run shadercross for JSON: {}", e);
            panic!("Failed to generate reflection JSON for {}", name);
        }
    }
}
