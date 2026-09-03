use proc_macro::TokenStream;
use quote::quote;
use std::path::Path;
use syn::parse::Parse;
use syn::{Data, DeriveInput, Fields, LitStr, parse_macro_input};

// ---------------------------------------------------------------------------
// stored_shaders!: embed sdl3_gs's offline shader pipeline output
// ---------------------------------------------------------------------------

/// Embeds the shader artifacts produced by sdl3_gs's offline pipeline
/// (`sdl3_gs::shader_build::compile_shaders`) into the calling crate.
///
/// Usage: `static STORED: LazyLock<StoredShaders> = sdl3_gs::stored_shaders!("target/shaders");`
///
/// The argument is the output root the caller's build script passed to
/// `compile_shaders`, resolved against the caller's manifest directory. The
/// result initializes on first use; per platform the reflection JSON plus the
/// bytecode directories that platform can consume are embedded — Windows:
/// `obj_dxil/` + `obj_spirv/`, Apple: `obj_msl/`, elsewhere: `obj_spirv/`.
#[proc_macro]
pub fn stored_shaders(input: TokenStream) -> TokenStream {
    match expand_stored_shaders(input) {
        Ok(tokens) => tokens,
        Err(e) => e.to_compile_error().into(),
    }
}

struct ShaderRoot(LitStr);

impl Parse for ShaderRoot {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(ShaderRoot(input.parse::<LitStr>()?))
    }
}

fn expand_stored_shaders(input: TokenStream) -> syn::Result<TokenStream> {
    let root = syn::parse::<ShaderRoot>(input)?.0;
    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        syn::Error::new(
            root.span(),
            "CARGO_MANIFEST_DIR is not set; run through cargo",
        )
    })?;
    let base = Path::new(&manifest).join(root.value());
    if !base.is_dir() {
        return Err(syn::Error::new(
            root.span(),
            format!(
                "shader artifact root `{}` does not exist; run \
                 sdl3_gs::shader_build::compile_shaders(.., {:?}) from build.rs first",
                base.display(),
                root.value()
            ),
        ));
    }

    // Reflection JSON always travels with the binary; backend directories
    // are embedded under their platform cfg so unselected platforms are
    // dropped by the compiler.
    let json = embed_dir(&base.join("obj_json"), "obj_json");

    // Per platform: a Vec of `(shader_type, TokenStream)` tuples — one per
    // backend that platform can consume, in preference order. The emitted
    // init closure declares each tuple's tokens as an `EmbeddedDir` const
    // and pushes it.
    let windows_backends: Vec<(ShaderKind, proc_macro2::TokenStream)> = vec![
        (
            ShaderKind::Dxil,
            embed_dir(&base.join("obj_dxil"), "obj_dxil"),
        ),
        (
            ShaderKind::Spv,
            embed_dir(&base.join("obj_spirv"), "obj_spirv"),
        ),
    ];

    let apple_backends: Vec<(ShaderKind, proc_macro2::TokenStream)> =
        vec![(ShaderKind::Msl, embed_dir(&base.join("obj_msl"), "obj_msl"))];

    let other_backends: Vec<(ShaderKind, proc_macro2::TokenStream)> = vec![(
        ShaderKind::Spv,
        embed_dir(&base.join("obj_spirv"), "obj_spirv"),
    )];

    let platforms: [(&str, &[(ShaderKind, proc_macro2::TokenStream)]); 3] = [
        ("target_os = \"windows\"", &windows_backends),
        ("all(unix, target_vendor = \"apple\")", &apple_backends),
        (
            "not(any(target_os = \"windows\", all(unix, target_vendor = \"apple\")))",
            &other_backends,
        ),
    ];

    let mut platform_blocks = Vec::new();
    for (cfg_predicate, backends) in platforms {
        let cfg_expr: proc_macro2::TokenStream = syn::parse_str(cfg_predicate)
            .map_err(|e| syn::Error::new(root.span(), format!("bad cfg predicate: {e}")))?;
        let cfg_attr = quote! { #[cfg(#cfg_expr)] };
        let entries = backends.iter().map(|(kind, dir)| {
            let const_ident = syn::Ident::new(kind.const_name(), proc_macro2::Span::call_site());
            let format_const: syn::Path =
                syn::parse_str(kind.format_path()).expect("valid format path");
            quote! {
                const #const_ident: ::sdl3_gs::shader_assets::EmbeddedDir = #dir;
                shaders.push((
                    &#const_ident,
                    #format_const,
                ));
            }
        });
        platform_blocks.push(quote! {
            #cfg_attr
            {
                #(#entries)*
            }
        });
    }

    // The result is a `LazyLock<StoredShaders>`: first access runs the init
    // closure, which pushes one entry per embedded backend the target
    // platform can consume.
    let expanded = quote! {
        ::std::sync::LazyLock::new(|| {
            const __SDL_JSON_DIR: ::sdl3_gs::shader_assets::EmbeddedDir = #json;

            let mut shaders = ::std::vec::Vec::new();

            #(#platform_blocks)*

            ::sdl3_gs::shader_assets::StoredShaders {
                json: &__SDL_JSON_DIR,
                shaders,
            }
        })
    };
    if std::env::var("SDL3_GS_DUMP_EXPANSION").is_ok() {
        eprintln!("=== stored_shaders expansion ===\n{expanded}");
    }
    Ok(expanded.into())
}

/// The shader format carried by an embedded backend directory.
#[derive(Clone, Copy)]
enum ShaderKind {
    Dxil,
    Msl,
    Spv,
}

impl ShaderKind {
    /// Name of the `EmbeddedDir` const declared for this backend inside the
    /// init closure.
    fn const_name(self) -> &'static str {
        match self {
            ShaderKind::Dxil => "__SDL_DXIL_DIR",
            ShaderKind::Msl => "__SDL_METAL_DIR",
            ShaderKind::Spv => "__SDL_SPV_DIR",
        }
    }

    /// Path to the matching `SDL_GPUShaderFormat` constant.
    fn format_path(self) -> &'static str {
        match self {
            ShaderKind::Dxil => "::sdl3_gs::device::SDL_GPUShaderFormat::DXIL",
            ShaderKind::Msl => "::sdl3_gs::device::SDL_GPUShaderFormat::MSL",
            ShaderKind::Spv => "::sdl3_gs::device::SDL_GPUShaderFormat::SPIRV",
        }
    }
}

/// Emits an `EmbeddedDir { .. }` expression for everything under `dir`.
/// A missing directory embeds as empty (e.g. shadercross unavailable -> no
/// `.dxil` / `.metal`).
fn embed_dir(dir: &Path, name: &str) -> proc_macro2::TokenStream {
    let mut files: Vec<(String, Vec<u8>)> = Vec::new();
    let _ = walk_files(dir, dir, &mut files);
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let entries = files.iter().map(|(path, bytes)| {
        let data = proc_macro2::Literal::byte_string(bytes);
        quote! {
            ::sdl3_gs::shader_assets::EmbeddedFile {
                path: #path,
                contents: #data,
            },
        }
    });

    quote! {
        ::sdl3_gs::shader_assets::EmbeddedDir {
            name: #name,
            files: &[#(#entries)*],
        }
    }
}

fn walk_files(root: &Path, dir: &Path, out: &mut Vec<(String, Vec<u8>)>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            walk_files(root, &path, out)?;
        } else {
            let rel = path
                .strip_prefix(root)
                .expect("walk stays below root")
                .to_string_lossy()
                .replace('\\', "/");
            let contents = std::fs::read(&path)?;
            out.push((rel, contents));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SDLVertexDesc derive
// ---------------------------------------------------------------------------

#[proc_macro_derive(SDLVertexDesc, attributes(sdl_vertex_desc))]
pub fn derive_sdl_vertex_desc(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let has_repr_c = input.attrs.iter().any(|attr| {
        if attr.path().is_ident("repr") {
            let mut found = false;
            let _ = attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("C") {
                    found = true;
                }
                Ok(())
            });
            found
        } else {
            false
        }
    });
    if !has_repr_c {
        panic!("SdlVertexDesc requires #[repr(C)] on the struct");
    }

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("SdlVertexDesc only supports structs with named fields"),
        },
        _ => panic!("SdlVertexDesc only supports structs"),
    };

    let attributes: Vec<_> = fields
        .iter()
        .filter_map(|field| {
            let (skip, format_override) = field_options(field);
            if skip {
                return None;
            }

            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;
            let format = format_override.unwrap_or_else(|| get_sdl_vertex_format(field_type));

            Some((field_name, format))
        })
        .enumerate()
        .map(|(location, (field_name, format))| {
            let location = location as u32;
            quote! {
                SDL_GPUVertexAttribute {
                    location: #location,
                    buffer_slot: 0,
                    format: #format,
                    offset: std::mem::offset_of!(#name, #field_name) as u32,
                }
            }
        })
        .collect();

    let expanded = quote! {
        impl #name {
            pub fn vertex_desc() -> (Vec<SDL_GPUVertexAttribute>, Vec<SDL_GPUVertexBufferDescription>) {
                let attrs = vec![
                    #(#attributes),*
                ];
                let bufs = vec![
                    SDL_GPUVertexBufferDescription {
                        slot: 0,
                        pitch: std::mem::size_of::<#name>() as u32,
                        input_rate: SDL_GPUVertexInputRate::VERTEX,
                        instance_step_rate: 0,
                    },
                ];
                (attrs, bufs)
            }
        }
    };

    TokenStream::from(expanded)
}

fn field_options(field: &syn::Field) -> (bool, Option<proc_macro2::TokenStream>) {
    let mut skip = false;
    let mut format = None;

    for attr in &field.attrs {
        if !attr.path().is_ident("sdl_vertex_desc") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("skip") {
                skip = true;
            } else if meta.path.is_ident("format") {
                let value = meta.value()?;
                let format_name: syn::Ident = value.parse()?;
                format = Some(quote!(SDL_GPUVertexElementFormat::#format_name));
            } else {
                return Err(meta.error("expected `skip` or `format = FORMAT`"));
            }
            Ok(())
        })
        .unwrap_or_else(|error| panic!("invalid sdl_vertex_desc attribute: {error}"));
    }

    (skip, format)
}

fn get_sdl_vertex_format(ty: &syn::Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string();

    match type_str.as_str() {
        "CVec2" | "Vec2" | "[f32 ; 2]" | "[f32; 2]" => quote!(SDL_GPUVertexElementFormat::FLOAT2),
        "CVec3" | "Vec3" | "[f32 ; 3]" | "[f32; 3]" => quote!(SDL_GPUVertexElementFormat::FLOAT3),
        "CVec4" | "Vec4" | "[f32 ; 4]" | "[f32; 4]" | "Quat" => {
            quote!(SDL_GPUVertexElementFormat::FLOAT4)
        }
        "[i8 ; 4]" | "[i8; 4]" => quote!(SDL_GPUVertexElementFormat::BYTE4),
        "[i32 ; 4]" | "[i32; 4]" => quote!(SDL_GPUVertexElementFormat::INT4),
        "f32" => quote!(SDL_GPUVertexElementFormat::FLOAT),
        "u32" => quote!(SDL_GPUVertexElementFormat::UBYTE4_NORM),
        "i32" => quote!(SDL_GPUVertexElementFormat::INT),
        _ => panic!("Unsupported SDL vertex attribute type: {}", type_str),
    }
}
