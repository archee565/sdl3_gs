//! Locates the spirv-cross C API shared library (as installed by the Vulkan SDK)
//! and emits the cargo directives to link against it.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Candidate file names for the shared library, per platform.
fn library_filenames() -> &'static [&'static str] {
    if cfg!(windows) {
        &["spirv-cross-c-shared.lib", "libspirv-cross-c-shared.dll.a"]
    } else if cfg!(target_os = "macos") {
        &[
            "libspirv-cross-c-shared.dylib",
            "libspirv-cross-c-shared.0.dylib",
        ]
    } else {
        &["libspirv-cross-c-shared.so", "libspirv-cross-c-shared.so.0"]
    }
}

fn find_in_dir(dir: &Path) -> Option<PathBuf> {
    if !dir.is_dir() {
        return None;
    }
    for name in library_filenames() {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn sdk_lib_dirs(sdk_root: &Path) -> Vec<PathBuf> {
    ["lib", "lib64", "x86_64", "x86_64/lib", "arm64", "arm64/lib"]
        .iter()
        .map(|sub| sdk_root.join(sub))
        .collect()
}

/// Multiarch library directories, e.g. /usr/lib/x86_64-linux-gnu.
fn multiarch_lib_dirs(prefix: &str) -> Vec<PathBuf> {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let triplet = match arch.as_str() {
        "x86_64" => "x86_64-linux-gnu",
        "aarch64" => "aarch64-linux-gnu",
        "arm" => "arm-linux-gnueabihf",
        "x86" => "i386-linux-gnu",
        "riscv64" => "riscv64-linux-gnu",
        "powerpc64" => "powerpc64le-linux-gnu",
        _ => return Vec::new(),
    };
    vec![PathBuf::from(format!("{prefix}/lib/{triplet}"))]
}

/// Ask pkg-config where the library lives, if a .pc file is installed.
fn pkg_config_libdirs() -> Vec<PathBuf> {
    let Ok(output) = Command::new("pkg-config")
        .args(["--libs", "spirv_cross_c_shared"])
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .filter_map(|flag| flag.strip_prefix("-L"))
        .map(PathBuf::from)
        .collect()
}

fn main() {
    println!("cargo:rerun-if-env-changed=VULKAN_SDK");
    println!("cargo:rerun-if-env-changed=SHADERCROSS_SPIRV_CROSS_DIR");

    let mut searched: Vec<String> = Vec::new();
    let mut candidates: Vec<PathBuf> = Vec::new();

    // Explicit override wins.
    if let Ok(dir) = env::var("SHADERCROSS_SPIRV_CROSS_DIR") {
        candidates.push(PathBuf::from(&dir));
        searched.push(format!("SHADERCROSS_SPIRV_CROSS_DIR: {dir}"));
    }

    // Vulkan SDK layout.
    if let Ok(sdk) = env::var("VULKAN_SDK") {
        candidates.extend(sdk_lib_dirs(Path::new(&sdk)));
        searched.push(format!("VULKAN_SDK: {sdk}"));
    }

    // pkg-config metadata (Linux distro / Homebrew installs).
    let pkg_dirs = pkg_config_libdirs();
    for dir in &pkg_dirs {
        searched.push(format!("pkg-config: {}", dir.display()));
    }
    candidates.extend(pkg_dirs);

    // Default linker search paths (system packages).
    let mut system_dirs: Vec<PathBuf> = ["/usr/lib", "/usr/local/lib"]
        .iter()
        .map(PathBuf::from)
        .collect();
    system_dirs.extend(multiarch_lib_dirs("/usr"));
    system_dirs.extend(multiarch_lib_dirs("/usr/local"));
    for dir in &system_dirs {
        searched.push(format!("system: {}", dir.display()));
    }
    candidates.extend(system_dirs);

    // If nothing was found in the candidate dirs, the linker may still resolve
    // the library from its default search path; only fail when we are certain.
    let found = candidates.iter().find_map(|dir| find_in_dir(dir));

    match found {
        Some(lib) => {
            let lib_dir = lib.parent().unwrap().to_path_buf();
            println!("cargo:rustc-link-search=native={}", lib_dir.display());
            println!("cargo:rustc-link-lib=dylib=spirv-cross-c-shared");
            // Make the binary find the shared library at runtime even when the
            // library lives outside the default loader path (e.g. ~/VulkanSDK).
            println!("cargo:rustc-link-arg-bins=-Wl,-rpath,{}", lib_dir.display());
        }
        None => {
            let searched = searched.join("\n  ");
            panic!(
                "could not locate the spirv-cross C API shared library \
                 (libspirv-cross-c-shared).\n\
                 Install the Vulkan SDK, or set VULKAN_SDK / SHADERCROSS_SPIRV_CROSS_DIR \
                 to the directory containing the library.\n\
                 Searched:\n  {searched}"
            );
        }
    }
}
