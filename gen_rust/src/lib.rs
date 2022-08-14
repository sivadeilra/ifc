//! Generates Rust code from IFC modules

#![allow(unused_imports)]
#![forbid(unused_must_use)]

use anyhow::{bail, Result};
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::*;
use ripper::*;
use syn::Ident;
use syn::*;

#[derive(Default, Clone, Debug)]
pub struct Options {}

pub fn gen_rust(ifc: &Ifc, options: &Options) -> Result<TokenStream> {
    let mut output = TokenStream::new();
    output.extend(gen_crate_start(ifc)?);
    output.extend(gen_types(ifc, options)?);
    Ok(output)
}

fn gen_crate_start(_ifc: &Ifc) -> Result<TokenStream> {
    Ok(quote! {
        //! This code was generated by `gen_rust` from C++ definitions, sourced through IFC.
        #![allow(non_camel_case_types)]
        #![allow(non_snake_case)]
        #![allow(non_upper_case_globals)]
    })
}

fn gen_types(ifc: &Ifc, options: &Options) -> Result<TokenStream> {
    gen_types_for_scope(ifc, ifc.file_header().global_scope, 50)
}

/// Recursively walks a scope and generates type definitions for it.
fn gen_types_for_scope(ifc: &Ifc, parent_scope: ScopeIndex, max_depth: u32) -> Result<TokenStream> {
    let mut output = TokenStream::new();

    println!(
        "Scope #{}{}",
        parent_scope,
        if parent_scope == ifc.file_header().global_scope {
            " - Global scope"
        } else {
            ""
        }
    );
    // `scope.descriptor` gives us the start and length of the region in `scope.members` where
    // the members for this scope can be found.
    let scope_descriptor = ifc.scope_desc().entry(parent_scope - 1)?;

    if max_depth == 0 {
        bail!("Max depth exceeded!");
    }
    let max_depth = max_depth - 1;

    println!("scope descriptor = {:?}", scope_descriptor);

    let scope_members = ifc.scope_member();

    for member_index in
        scope_descriptor.start..scope_descriptor.start + scope_descriptor.cardinality
    {
        let member_decl_index: DeclIndex = *scope_members.entry(member_index)?;

        match member_decl_index.tag() {
            DeclSort::ALIAS => {
                let decl_alias = ifc.decl_alias().entry(member_decl_index.index())?;
                let alias_name = ifc.get_string(decl_alias.name)?;
                println!("    alias: {}", alias_name);
                println!("{:#?}", decl_alias);
            }

            DeclSort::FUNCTION => {
                // 8.2.16
                let func_decl = ifc.decl_function().entry(member_decl_index.index())?;
                let func_name = match func_decl.name.tag() {
                    NameSort::IDENTIFIER => ifc.get_string(func_decl.name.index())?.to_string(),
                    _ => format!("{:?}", func_decl.name),
                };
                // println!("function \"{}\" = {:#?}", func_name, func_decl);

                let type_str = ifc.get_type_string(func_decl.type_)?;
                println!("function: \"{}\" = type = {}", func_name, type_str);
            }

            DeclSort::SCOPE => {
                let nested_scope = ifc.decl_scope().entry(member_decl_index.index())?;
                let nested_scope_name = ifc.get_string(nested_scope.name.index())?;

                // What kind of scope is it?
                if ifc.is_type_namespace(nested_scope.ty)? {
                    println!("    namespace {} {{ ... }}", nested_scope_name);
                } else {
                    println!(
                        "    nested scope: name: {:#?} - {:?}",
                        nested_scope_name, nested_scope.ty
                    );
                }

                // println!("{:#?}", nested_scope);

                // if scope.initializer != 0 {
                //     dump_scope(ifc, scope.initializer, max_depth)?;
                // }
            }

            DeclSort::FIELD => {
                let field = ifc.decl_field().entry(member_decl_index.index())?;
                let field_name = ifc.get_string(field.name)?;
                let field_type_string = ifc.get_type_string(field.ty)?;
                println!("    field: {} : {}", field_name, field_type_string);
            }

            DeclSort::METHOD => {}

            DeclSort::ENUMERATION => {
                let en = ifc.decl_enum().entry(member_decl_index.index())?;
                let t = gen_enum(ifc, &en)?;
                output.extend(t);
            }

            _ => {
                nyi!();
                println!("unknown decl: {:?}", member_decl_index);
            }
        }
    }

    Ok(output)
}

fn get_type_tokens(ifc: &Ifc, type_index: TypeIndex) -> Result<TokenStream> {
    Ok(match type_index.tag() {
        TypeSort::FUNDAMENTAL => {
            let fun_ty = ifc.type_fundamental().entry(type_index.index())?;
            let is_signed = matches!(fun_ty.sign, TypeSign::SIGNED | TypeSign::PLAIN);

            match fun_ty.basis {
                TypeBasis::INT => match (fun_ty.precision, is_signed) {
                    (TypePrecision::DEFAULT | TypePrecision::LONG | TypePrecision::BIT32, true) => {
                        quote!(i32)
                    }
                    (
                        TypePrecision::DEFAULT | TypePrecision::LONG | TypePrecision::BIT32,
                        false,
                    ) => quote!(u32),
                    (TypePrecision::BIT8, true) => quote!(i8),
                    (TypePrecision::BIT8, false) => quote!(u8),
                    (TypePrecision::BIT16, true) => quote!(i16),
                    (TypePrecision::BIT16, false) => quote!(u16),
                    (TypePrecision::BIT64, true) => quote!(i64),
                    (TypePrecision::BIT64, false) => quote!(u64),
                    (TypePrecision::BIT128, true) => quote!(i128),
                    (TypePrecision::BIT128, false) => quote!(u128),
                    _ => todo!("unrecognized INT type: {:?}", fun_ty),
                },

                TypeBasis::CHAR => match (fun_ty.precision, is_signed) {
                    (TypePrecision::DEFAULT, true) => quote!(i8),
                    (TypePrecision::DEFAULT, false) => quote!(u8),
                    _ => {
                        todo!("unrecognized CHAR type: {:?}", fun_ty);
                    }
                },

                TypeBasis::FLOAT | TypeBasis::DOUBLE => {
                    if fun_ty.precision != TypePrecision::DEFAULT {
                        bail!("Floating-point types must have default precision");
                    }
                    if fun_ty.sign != TypeSign::PLAIN {
                        bail!("Floating-point types must have default sign");
                    }
                    if fun_ty.basis == TypeBasis::FLOAT {
                        quote!(f32)
                    } else {
                        quote!(f64)
                    }
                }

                _ => todo!(),
            }
        }
        _ => todo!(),
    })
}

// This converts literal expressions into token streams.
fn gen_expr_tokens(ifc: &Ifc, expr: ripper::ExprIndex) -> Result<TokenStream> {
    println!("gen_expr_tokens: expr = {:?}", expr);
    Ok(match expr.tag() {
        ExprSort::LITERAL => {
            let literal = ifc.expr_literal().entry(expr.index())?;
            println!("literal = {:?}", literal);

            if true {
                match literal.value.tag() {
                    LiteralSort::IMMEDIATE => {
                        let value: u32 = literal.value.index();
                        let lit = syn::LitInt::new(&value.to_string(), Span::call_site());
                        quote!(#lit)
                    }
                    LiteralSort::INTEGER => {
                        let value: u64 = *ifc.const_i64().entry(literal.value.index())?;
                        let lit = syn::LitInt::new(&value.to_string(), Span::call_site());
                        quote!(#lit)
                    }
                    LiteralSort::FLOATING_POINT => {
                        todo!("floating point literals")
                    }
                    _ => todo!("unrecognized literal value: {:?}", literal.value),
                }
            } else {
                let value: u32 = literal.value.index();
                quote!(#value + 100000)
            }
        }

        _ => todo!("unsupported expr: {:?}", expr),
    })
}

fn gen_enum(ifc: &Ifc, enum_decl: &DeclEnum) -> Result<TokenStream> {
    // let en = ifc.decl_enum().entry(member_decl_index.index())?;
    let en_name = ifc.get_string(enum_decl.name)?;
    let en_ident = Ident::new(&en_name, Span::call_site());

    println!("enumeration decl:\n{:#?}", enum_decl);

    // Is this an ordinary enum, or an enum class?
    if enum_decl.ty.tag() != TypeSort::FUNDAMENTAL {
        bail!("Expected DeclEnum.ty to be TypeSort::FUNDAMENTAL");
    }
    let en_ty = ifc.type_fundamental().entry(enum_decl.ty.index())?;
    let is_enum_class = match en_ty.basis {
        TypeBasis::CLASS | TypeBasis::STRUCT => true,
        TypeBasis::ENUM => false,
        _ => {
            println!(
                "warning: enum type basis is not recognized: {:?}",
                en_ty.basis
            );
            false
        }
    };

    // Determine the storage type of the enum.  The default is int (i32).
    if enum_decl.base.tag() != TypeSort::FUNDAMENTAL {
        bail!("Expected DeclEnum.base to be TypeSort::FUNDAMENTAL");
    }
    let base_type = ifc.type_fundamental().entry(enum_decl.base.index())?;
    println!("base_type = {:?}", base_type);

    let storage_ty: TokenStream = get_type_tokens(ifc, enum_decl.base)?;

    // Generate the enum type tokens.

    let mut output = quote! {
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct #en_ident(pub #storage_ty);
    };

    let mut variants_tokens = TokenStream::new();

    for var_index in enum_decl.initializer.to_range() {
        let var = ifc.decl_enumerator().entry(var_index)?;
        let var_name_string = ifc.get_string(var.name)?;
        let var_name_ident = Ident::new(&var_name_string, Span::call_site());

        println!("enumerator: {} {:?}", var_name_string, var);

        let initializer = gen_expr_tokens(ifc, var.initializer)?;

        variants_tokens.extend(quote! {
            pub const #var_name_ident: #en_ident = #en_ident(#initializer);
        });
        println!();
    }

    if is_enum_class {
        output.extend(quote! {
            impl #en_ident {
                #variants_tokens
            }
        });
    } else {
        output.extend(variants_tokens);
    }

    parse_check_mod_items(&output)?;
    Ok(output)
}

fn parse_check<T: syn::parse::Parse>(t: &TokenStream) -> Result<()> {
    let tt = t.clone();
    match syn::parse2::<T>(tt) {
        Ok(_) => Ok(()),
        Err(e) => {
            println!("FAILED to parse token stream with expected type.");
            println!("Error: {:?}", e);
            println!("Token stream:\n{}", t);
            Err(e.into())
        }
    }
}

// Check that `t` can be successfully parsed as a sequence of items inside a mod.
fn parse_check_mod_items(t: &TokenStream) -> Result<()> {
    let mod_t = quote! {
        mod foo {
            #t
        }
    };

    parse_check::<syn::ItemMod>(&mod_t)
}
