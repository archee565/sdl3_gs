//! HLSL compilation by invoking the `dxc` command line tool.
//!
//! This replaces the COM-based `IDxcCompiler3` path of the C library; the
//! argument construction mirrors `SDL_ShaderCross_INTERNAL_CompileUsingDXC`.
//! Note that SPIR-V output requires a dxc build with SPIR-V codegen enabled
//! (such as the one shipped with the Vulkan SDK).

use crate::error::{Error, Result};
use crate::{HlslInfo, ShaderStage};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The dxc binary to invoke. Defaults to `dxc` on `PATH`.
pub fn dxc_binary() -> PathBuf {
    std::env::var_os("SHADERCROSS_DXC")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("dxc"))
}

fn profile(shader_stage: ShaderStage) -> &'static str {
    match shader_stage {
        ShaderStage::Vertex => "vs_6_0",
        ShaderStage::Fragment => "ps_6_0",
        ShaderStage::Compute => "cs_6_0",
    }
}

/// Compiles HLSL source to SPIR-V (`spirv = true`) or DXIL (`spirv = false`).
pub(crate) fn compile_using_dxc(info: &HlslInfo, spirv: bool) -> Result<Vec<u8>> {
    let temp_dir = tempfile::tempdir()?;

    // dxc embeds the input file path into debug information. The C library
    // passes the debug name as a bare argument; here we name the temporary
    // input file after it instead.
    let input_name = match &info.options.debug_name {
        Some(name) => Path::new(name)
            .file_name()
            .map(|s| s.to_os_string())
            .unwrap_or_else(|| std::ffi::OsStr::new(name).to_os_string()),
        None => std::ffi::OsString::from("input.hlsl"),
    };
    let input_path = temp_dir.path().join(input_name);
    let mut input_file = std::fs::File::create(&input_path)?;
    input_file.write_all(info.source.as_bytes())?;
    input_file.flush()?;

    let output_path = temp_dir.path().join("output.bin");

    let mut args: Vec<String> = Vec::new();

    for define in info.defines {
        match &define.value {
            Some(value) => args.push(format!("-D{}={}", define.name, value)),
            None => args.push(format!("-D{}=1", define.name)),
        }
    }

    args.push("-E".into());
    args.push(info.entrypoint.into());

    if let Some(include_dir) = info.include_dir {
        args.push("-I".into());
        args.push(include_dir.display().to_string());
    }

    args.push("-T".into());
    args.push(profile(info.shader_stage).into());

    if spirv {
        args.push("-spirv".into());
        args.push("-fspv-flatten-resource-arrays".into());

        if !info.options.cull_unused_bindings {
            args.push("-fspv-preserve-bindings".into());
        }

        args.push("-fspv-preserve-interface".into());
    }

    if info.options.debug {
        if spirv {
            // https://github.com/microsoft/DirectXShaderCompiler/blob/main/docs/SPIR-V.rst#debugging
            args.push("-fspv-debug=vulkan-with-source".into());
        } else {
            // https://github.com/microsoft/DirectXShaderCompiler/blob/main/docs/SourceLevelDebuggingHLSL.rst#command-line-options
            args.push("-Zi".into());
        }
    }

    args.push("-Fo".into());
    args.push(output_path.display().to_string());

    // The input file is dxc's positional argument and comes last.
    args.push(input_path.display().to_string());

    run_dxc(&args, &output_path)
}

fn run_dxc(args: &[String], output_path: &Path) -> Result<Vec<u8>> {
    let dxc = dxc_binary();
    let output = Command::new(&dxc)
        .args(args)
        .output()
        .map_err(|source| Error::DxcLaunch {
            path: dxc.clone(),
            source,
        })?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !output.status.success() {
        let message = if stderr.trim().is_empty() {
            format!("{} exited with {}", dxc.display(), output.status)
        } else {
            format!("HLSL compilation failed: {}", stderr.trim_end())
        };
        return Err(Error::Dxc { message });
    }

    // If compilation succeeded but there is diagnostic output, those are warnings.
    if !stderr.trim().is_empty() {
        eprintln!("HLSL compiled with warnings: {}", stderr.trim_end());
    }

    match std::fs::read(output_path) {
        Ok(bytes) => Ok(bytes),
        Err(_) => Err(Error::Dxc {
            message: "HLSL compilation failed with unknown error".into(),
        }),
    }
}
