extern crate proc_macro;

use crate::proc_macro::TokenStream;
use quote::quote; //quote_spanned;
use syn;

#[proc_macro_derive(DrawableLayoutElement)]
pub fn drawable_derive(input: TokenStream) -> TokenStream {
    // Construct a representation of Rust code as a syntax tree that we can manipulate.
    let ast = syn::parse(input).unwrap();

    // Build the trait implementation.
    impl_drawable_macro(&ast)
}

fn impl_drawable_macro(ast: &syn::DeriveInput) -> TokenStream {
    let name = &ast.ident;
    let variants = match &ast.data {
        syn::Data::Enum(data) => &data.variants,
        _ => panic!("derive(DrawableLayoutElement) only works on enums!"),
    };

    let traverse_draw = variants.iter().map(|f| {
        let variant_name = &f.ident;
        // TODO: quote_spanned, condense.
        quote! {
            #name::#variant_name(ref __self_0) => __self_0.draw(hook, offset, parent_rect, window)
        }
    });

    let traverse_predict = variants.iter().map(|f| {
        let variant_name = &f.ident;
        quote! {
            #name::#variant_name(ref mut __self_0) => __self_0.predict_rect_and_init(hook, offset, parent_rect, window)
        }
    });

    let traverse_update = variants.iter().map(|f| {
        let variant_name = &f.ident;
        quote! {
            #name::#variant_name(ref mut __self_0) => __self_0.update(delta_time, window)
        }
    });

    let traverse_clicked = variants.iter().map(|f| {
        let variant_name = &f.ident;
        quote! {
            #name::#variant_name(ref mut __self_0) => __self_0.clicked(window)
        }
    });

    let traverse_hovered = variants.iter().map(|f| {
        let variant_name = &f.ident;
        quote! {
            #name::#variant_name(ref mut __self_0) => __self_0.hovered(entered, window)
        }
    });


    let gen = quote! {
        impl DrawableLayoutElement for #name {
            fn draw(&self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
                window.context.save();
                let rect = match self {
                    #(#traverse_draw),*
                };
                window.context.restore();

                rect
            }

            fn predict_rect_and_init(&mut self, hook: &Hook, offset: &Vec2, parent_rect: &Rect, window: &NotifyWindow) -> Rect {
                window.context.save();
                let rect = match self {
                    #(#traverse_predict),*
                };
                window.context.restore();

                rect
            }

            fn update(&mut self, delta_time: Duration, window: &NotifyWindow) -> bool { 
                match self {
                    #(#traverse_update),*
                }
            }

            fn clicked(&mut self, window: &NotifyWindow) -> bool { 
                match self {
                    #(#traverse_clicked),*
                }
            }

            fn hovered(&mut self, entered: bool, window: &NotifyWindow) -> bool { 
                match self {
                    #(#traverse_hovered),*
                }
            }
        }
    };

    gen.into()
}


