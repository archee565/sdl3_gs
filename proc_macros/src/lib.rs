use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(SDLVertexDesc)]
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
        .enumerate()
        .map(|(idx, field)| {
            let field_name = field.ident.as_ref().unwrap();
            let field_type = &field.ty;
            let location = idx as u32;
            let format = get_sdl_vertex_format(field_type);

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

fn get_sdl_vertex_format(ty: &syn::Type) -> proc_macro2::TokenStream {
    let type_str = quote!(#ty).to_string();

    match type_str.as_str() {
        "CVec2" | "Vec2" | "[f32 ; 2]" | "[f32; 2]" => quote!(SDL_GPUVertexElementFormat::FLOAT2),
        "CVec3" | "Vec3" | "[f32 ; 3]" | "[f32; 3]" => quote!(SDL_GPUVertexElementFormat::FLOAT3),
        "CVec4" | "Vec4" | "[f32 ; 4]" | "[f32; 4]" | "Quat" => quote!(SDL_GPUVertexElementFormat::FLOAT4),
        "[i8 ; 4]" | "[i8; 4]" => quote!(SDL_GPUVertexElementFormat::BYTE4),
        "[i32 ; 4]" | "[i32; 4]" => quote!(SDL_GPUVertexElementFormat::INT4),
        "f32" => quote!(SDL_GPUVertexElementFormat::FLOAT),
        "u32" => quote!(SDL_GPUVertexElementFormat::UBYTE4_NORM),
        "i32" => quote!(SDL_GPUVertexElementFormat::INT),
        _ => panic!("Unsupported SDL vertex attribute type: {}", type_str),
    }
}
