//! The shadercross command line tool, ported from src/cli.c.

use clap::{Parser, ValueEnum};
use shadercross::{
    compile_dxil_from_hlsl, compile_dxil_from_spirv, compile_spirv_from_hlsl,
    reflect_compute_spirv, reflect_graphics_spirv, transpile_hlsl_from_spirv,
    transpile_msl_from_spirv, Error, HlslDefine, HlslInfo, Options, ShaderStage, SpirvInfo,
};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;

#[derive(ValueEnum, Clone, Copy)]
enum SourceFormat {
    Spirv,
    Hlsl,
}

#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
enum DestinationFormat {
    Dxbc,
    Dxil,
    Msl,
    Spirv,
    Hlsl,
    Json,
}

#[derive(ValueEnum, Clone, Copy, PartialEq, Eq)]
enum StageFormat {
    Vertex,
    Fragment,
    Compute,
}

impl From<StageFormat> for ShaderStage {
    fn from(stage: StageFormat) -> Self {
        match stage {
            StageFormat::Vertex => ShaderStage::Vertex,
            StageFormat::Fragment => ShaderStage::Fragment,
            StageFormat::Compute => ShaderStage::Compute,
        }
    }
}

fn print_help() {
    let column_width = 32;
    let required: [(&str, &str); 5] = [
        (
            "-s | --source <value>",
            "Source language format. May be inferred from the filename. Values: [SPIRV, HLSL]",
        ),
        (
            "-d | --dest <value>",
            "Destination format. May be inferred from the filename. Values: [DXBC, DXIL, MSL, SPIRV, HLSL, JSON]",
        ),
        (
            "-t | --stage <value>",
            "Shader stage. May be inferred from the filename. Values: [vertex, fragment, compute]",
        ),
        (
            "-e | --entrypoint <value>",
            "Entrypoint function name. Default: \"main\".",
        ),
        ("-o | --output <value>", "Output file."),
    ];
    let optional: [(&str, &str); 8] = [
        (
            "-I | --include <value>",
            "HLSL include directory. Only used with HLSL source.",
        ),
        (
            "-D<name>[=<value>]",
            "HLSL define. Only used with HLSL source. Can be repeated.",
        ),
        (
            "",
            "If =<value> is omitted the define will be treated as equal to 1.",
        ),
        (
            "--msl-version <value>",
            "Target MSL version. Only used when transpiling to MSL. The default is 1.2.0.",
        ),
        (
            "-c | --cull",
            "Allow the compiler to cull unused resource bindings. This may lead to surprising binding behavior so be careful when enabling this!",
        ),
        (
            "-g | --debug",
            "Generate debug information when possible. Shaders are valid only when graphics debuggers are attached.",
        ),
        (
            "-p | --pssl",
            "Generate PSSL-compatible shader. Destination format should be HLSL.",
        ),
        ("-h | --help", "Print help."),
    ];

    println!("Usage: shadercross <input> [options]");
    println!("Required options:\n");
    for (flag, description) in required {
        println!("  {:<width$} {}", flag, description, width = column_width);
    }
    println!("\n");
    println!("Optional options:\n");
    for (flag, description) in optional {
        println!("  {:<width$} {}", flag, description, width = column_width);
    }
}

fn fail(message: &str) -> ! {
    eprintln!("{message}");
    print_help();
    exit(1);
}

fn write_all(output_file: &mut File, buffer: &[u8]) -> Result<(), String> {
    output_file
        .write_all(buffer)
        .map_err(|error| error.to_string())
}

fn spirv_info<'a>(
    bytecode: &'a [u8],
    entrypoint: &'a str,
    shader_stage: ShaderStage,
    options: &'a Options,
) -> SpirvInfo<'a> {
    SpirvInfo {
        bytecode,
        entrypoint,
        shader_stage,
        options,
    }
}

#[derive(Parser)]
#[command(
    name = "shadercross",
    disable_help_flag = true,
    disable_version_flag = true
)]
struct Cli {
    /// Print help.
    #[arg(short = 'h', long = "help")]
    help: bool,

    /// Source language format. May be inferred from the filename.
    #[arg(short = 's', long = "source", value_enum, ignore_case = true)]
    source: Option<SourceFormat>,

    /// Destination format. May be inferred from the filename.
    #[arg(short = 'd', long = "dest", value_enum, ignore_case = true)]
    dest: Option<DestinationFormat>,

    /// Shader stage. May be inferred from the filename.
    #[arg(short = 't', long = "stage", value_enum, ignore_case = true)]
    stage: Option<StageFormat>,

    /// Entrypoint function name.
    #[arg(short = 'e', long = "entrypoint", default_value = "main")]
    entrypoint: String,

    /// Output file.
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// HLSL include directory. Only used with HLSL source.
    #[arg(short = 'I', long = "include")]
    include: Option<PathBuf>,

    /// HLSL define. Only used with HLSL source. Can be repeated. If =<value>
    /// is omitted the define will be treated as equal to 1.
    #[arg(short = 'D', value_name = "NAME[=VALUE]")]
    defines: Vec<String>,

    /// Target MSL version. Only used when transpiling to MSL.
    #[arg(long = "msl-version")]
    msl_version: Option<String>,

    /// Allow the compiler to cull unused resource bindings.
    #[arg(short = 'c', long = "cull")]
    cull: bool,

    /// Generate debug information when possible.
    #[arg(short = 'g', long = "debug")]
    debug: bool,

    /// Generate PSSL-compatible shader.
    #[arg(short = 'p', long = "pssl")]
    pssl: bool,

    /// Input file.
    input: Option<PathBuf>,
}

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            eprintln!("{error}");
            print_help();
            exit(1);
        }
    };

    if cli.help {
        print_help();
        exit(0);
    }

    let input_filename = match &cli.input {
        Some(path) => path,
        None => fail("shadercross: missing input path"),
    };
    let output_filename = match &cli.output {
        Some(path) => path,
        None => fail("shadercross: missing output path"),
    };

    let file_data = match std::fs::read(input_filename) {
        Ok(data) => data,
        Err(error) => fail(&format!("Invalid file ({error})")),
    };

    let spirv_source = match cli.source {
        Some(SourceFormat::Spirv) => true,
        Some(SourceFormat::Hlsl) => false,
        None => {
            let name = input_filename.to_string_lossy();
            if name.contains(".spv") {
                true
            } else if name.contains(".hlsl") {
                false
            } else {
                fail("Could not infer source format!");
            }
        }
    };

    let destination_format = match cli.dest {
        Some(format) => format,
        None => {
            let name = output_filename.to_string_lossy();
            if name.contains(".dxbc") {
                DestinationFormat::Dxbc
            } else if name.contains(".dxil") {
                DestinationFormat::Dxil
            } else if name.contains(".msl") {
                DestinationFormat::Msl
            } else if name.contains(".spv") {
                DestinationFormat::Spirv
            } else if name.contains(".hlsl") {
                DestinationFormat::Hlsl
            } else if name.contains(".json") {
                DestinationFormat::Json
            } else {
                fail("Could not infer destination format!");
            }
        }
    };

    let shader_stage = match cli.stage {
        Some(stage) => stage,
        None => {
            let name = input_filename.to_string_lossy().to_lowercase();
            if name.contains(".vert") {
                StageFormat::Vertex
            } else if name.contains(".frag") {
                StageFormat::Fragment
            } else if name.contains(".comp") {
                StageFormat::Compute
            } else {
                fail("Could not infer shader stage from filename!");
            }
        }
    };

    let defines: Vec<HlslDefine> = cli
        .defines
        .iter()
        .map(|define| {
            if let Some((name, value)) = define.split_once('=') {
                HlslDefine {
                    name: name.to_owned(),
                    value: Some(value.to_owned()),
                }
            } else {
                // no '=' was found, the define is treated as equal to 1
                HlslDefine {
                    name: define.clone(),
                    value: None,
                }
            }
        })
        .collect();

    let options = Options {
        debug: cli.debug,
        debug_name: cli.debug.then(|| input_filename.display().to_string()),
        cull_unused_bindings: cli.cull,
        pssl_compatibility: cli.pssl,
        msl_version: cli
            .msl_version
            .clone()
            .unwrap_or_else(|| "1.2.0".to_owned()),
        skip_spirv_roundtrip: false,
    };

    // The C tool creates the output file before compiling.
    let mut output_file = match File::create(output_filename) {
        Ok(file) => file,
        Err(error) => fail(&format!("{error}")),
    };

    let unsupported = Error::UnsupportedDestination.to_string();
    let stage = shader_stage.into();

    let result: Result<(), String> = if spirv_source {
        let info = SpirvInfo {
            bytecode: &file_data,
            entrypoint: &cli.entrypoint,
            shader_stage: stage,
            options: &options,
        };

        match destination_format {
            DestinationFormat::Dxbc => Err(unsupported),
            DestinationFormat::Dxil => match compile_dxil_from_spirv(&info) {
                Ok(buffer) => write_all(&mut output_file, &buffer),
                Err(error) => Err(format!("Failed to compile DXIL from SPIR-V: {error}")),
            },
            DestinationFormat::Msl => match transpile_msl_from_spirv(&info) {
                Ok(buffer) => write_all(&mut output_file, buffer.as_bytes()),
                Err(error) => Err(format!("Failed to transpile MSL from SPIR-V: {error}")),
            },
            DestinationFormat::Hlsl => match transpile_hlsl_from_spirv(&info) {
                Ok(buffer) => write_all(&mut output_file, buffer.as_bytes()),
                Err(error) => Err(format!("Failed to transpile HLSL from SPIRV: {error}")),
            },
            DestinationFormat::Spirv => {
                fail("Input and output are both SPIRV. Did you mean to do that?")
            }
            DestinationFormat::Json => {
                let json = if shader_stage == StageFormat::Compute {
                    reflect_compute_spirv(&file_data)
                        .map(|info| shadercross::json::write_compute_reflect_json(&info))
                } else {
                    reflect_graphics_spirv(&file_data)
                        .map(|info| shadercross::json::write_graphics_reflect_json(&info))
                };
                match json {
                    Ok(json) => write_all(&mut output_file, json.as_bytes()),
                    Err(error) => Err(format!("Failed to reflect SPIRV: {error}")),
                }
            }
        }
    } else {
        let source = match std::str::from_utf8(&file_data) {
            Ok(source) => source,
            Err(error) => fail(&format!("Input file is not valid UTF-8: {error}")),
        };
        let info = HlslInfo {
            source,
            entrypoint: &cli.entrypoint,
            include_dir: cli.include.as_deref(),
            defines: &defines,
            shader_stage: stage,
            options: &options,
        };

        let stage = shader_stage.into();

        match destination_format {
            DestinationFormat::Dxbc => Err(unsupported),
            DestinationFormat::Dxil => match compile_dxil_from_hlsl(&info) {
                Ok(buffer) => write_all(&mut output_file, &buffer),
                Err(error) => Err(format!("Failed to compile DXIL from HLSL: {error}")),
            },
            // TODO: Should we have TranspileMSLFromHLSL?
            DestinationFormat::Msl => match compile_spirv_from_hlsl(&info) {
                Err(error) => Err(format!("Failed to transpile MSL from HLSL: {error}")),
                Ok(spirv) => {
                    match transpile_msl_from_spirv(&spirv_info(
                        &spirv,
                        &cli.entrypoint,
                        stage,
                        &options,
                    )) {
                        Ok(buffer) => write_all(&mut output_file, buffer.as_bytes()),
                        Err(error) => Err(format!("Failed to transpile MSL from HLSL: {error}")),
                    }
                }
            },
            DestinationFormat::Spirv => match compile_spirv_from_hlsl(&info) {
                Ok(buffer) => write_all(&mut output_file, &buffer),
                Err(error) => Err(format!("Failed to compile SPIR-V From HLSL: {error}")),
            },
            DestinationFormat::Hlsl => match compile_spirv_from_hlsl(&info) {
                Err(error) => Err(format!("Failed to compile HLSL to SPIRV: {error}")),
                Ok(spirv) => {
                    match transpile_hlsl_from_spirv(&spirv_info(
                        &spirv,
                        &cli.entrypoint,
                        stage,
                        &options,
                    )) {
                        Ok(buffer) => write_all(&mut output_file, buffer.as_bytes()),
                        Err(error) => Err(format!("Failed to transpile HLSL from SPIRV: {error}")),
                    }
                }
            },
            DestinationFormat::Json => match compile_spirv_from_hlsl(&info) {
                Err(error) => Err(format!("Failed to compile HLSL to SPIRV: {error}")),
                Ok(spirv) => {
                    let json = if shader_stage == StageFormat::Compute {
                        reflect_compute_spirv(&spirv)
                            .map(|info| shadercross::json::write_compute_reflect_json(&info))
                    } else {
                        reflect_graphics_spirv(&spirv)
                            .map(|info| shadercross::json::write_graphics_reflect_json(&info))
                    };
                    match json {
                        Ok(json) => write_all(&mut output_file, json.as_bytes()),
                        Err(error) => Err(format!("Failed to reflect SPIRV: {error}")),
                    }
                }
            },
        }
    };

    match result {
        Ok(()) => exit(0),
        Err(error) => {
            eprintln!("{error}");
            exit(1);
        }
    }
}
