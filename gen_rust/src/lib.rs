//! Generates Rust code from IFC modules

#![allow(unused_imports)]
#![forbid(unused_must_use)]

use anyhow::{bail, Result};
use expr::*;
use ifc::*;
use log::warn;
use log::{debug, info, trace};
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::*;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use syn::Ident;
use syn::*;

mod enums;
mod expr;
mod funcs;
mod pp;
mod structs;
mod ty;
mod vars;

#[derive(Clone, Debug)]
pub struct Options {
    /// Derive Debug impls for types, by default
    pub derive_debug: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self { derive_debug: true }
    }
}

// This is an alias into the `refs` table.
type RefIndex = usize;

struct Gen<'a> {
    ifc: &'a Ifc,
    symbol_map: SymbolMap,
    options: &'a Options,
    #[allow(dead_code)]
    wk: WellKnown,
}

#[allow(dead_code)]
struct WellKnown {
    tokens_empty: TokenStream,
    tokens_false: TokenStream,
    tokens_true: TokenStream,
}

pub struct ReferencedIfc {
    pub name: String,
    pub ifc: Arc<Ifc>,
}

#[derive(Default, Clone)]
pub struct SymbolMap {
    pub crates: Vec<String>,
    pub map: HashMap<String, RefIndex>,
}

impl SymbolMap {
    /// Scan through the symbols exported by each of the IFC files.
    /// Build a table that maps from each symbol name to the index of the IFC file that defines that
    /// symbol.
    ///
    /// If a symbol is exported by more than one Ifc, then just pick the one with the lowest index.
    /// This is a terrible idea, but it's good enough to start with.
    ///
    /// For now, we only process the root scope.  So no nested namespaces.  And we ignore preprocessor
    /// definitions.
    pub fn add_ref_ifc(&mut self, ifc_name: &str, ifc: &Ifc) -> Result<RefIndex> {
        let ifc_index = self.crates.len();

        self.crates.push(ifc_name.to_string());

        let mut num_added: u64 = 0;

        let mut add_symbol = |name: &str| {
            if let Some(existing_index) = self.map.get_mut(name) {
                if ifc_index < *existing_index {
                    *existing_index = ifc_index;
                }
            } else {
                // This is the first time we've seen this symbol. Insert using the current IFC index.
                self.map.insert(name.to_string(), ifc_index);
            }
            num_added += 1;
        };

        let scope = ifc.global_scope();
        for member_decl in ifc.iter_scope(scope)? {
            match member_decl.tag() {
                DeclSort::SCOPE => {
                    let nested_scope = ifc.decl_scope().entry(member_decl.index())?;
                    if ifc.is_type_namespace(nested_scope.ty)? {
                        // For now, we ignore namespaces.
                    } else {
                        // It's a nested struct/class.
                        if nested_scope.name.tag() == NameSort::IDENTIFIER {
                            let nested_name = ifc.get_string(nested_scope.name.index())?;
                            add_symbol(nested_name);
                        } else {
                            warn!("ignoring scope member named: {:?}", nested_scope.name);
                        }
                    }
                }

                DeclSort::ALIAS => {
                    let alias = ifc.decl_alias().entry(member_decl.index())?;
                    let alias_name = ifc.get_string(alias.name)?;
                    add_symbol(alias_name);
                }

                DeclSort::ENUMERATION => {
                    let en = ifc.decl_enum().entry(member_decl.index())?;
                    let en_name = ifc.get_string(en.name)?;
                    add_symbol(en_name);
                }

                DeclSort::INTRINSIC => {}
                DeclSort::TEMPLATE => {}
                DeclSort::EXPLICIT_INSTANTIATION => {}
                DeclSort::EXPLICIT_SPECIALIZATION => {}

                DeclSort::FUNCTION => {
                    let func_decl = ifc.decl_function().entry(member_decl.index())?;
                    match func_decl.name.tag() {
                        NameSort::IDENTIFIER => {
                            let func_name = ifc.get_string(func_decl.name.index())?;
                            add_symbol(func_name);
                        }
                        _ => {
                            warn!("ignoring function named: {:?}", func_decl.name);
                        }
                    }
                }

                _ => {
                    warn!("ignoring unrecognized scope member: {:?}", member_decl);
                }
            }
        }

        info!(
            "Number of symbols added for this IFC '{}' (crate #{}): {}",
            ifc_name, ifc_index, num_added
        );
        Ok(ifc_index)
    }

    pub fn is_symbol_in(&self, name: &str) -> bool {
        self.map.contains_key(name)
    }

    pub fn resolve(&self, name: &str) -> Option<&str> {
        let crate_index = *self.map.get(name)?;
        Some(self.crates[crate_index].as_str())
    }
}

impl<'a> Gen<'a> {
    fn new(ifc: &'a Ifc, symbol_map: SymbolMap, options: &'a Options) -> Self {
        Self {
            ifc,
            symbol_map: symbol_map,
            options,
            wk: WellKnown {
                tokens_empty: quote!(),
                tokens_false: quote!(false),
                tokens_true: quote!(true),
            },
        }
    }
}

pub fn gen_rust(ifc: &Ifc, symbol_map: SymbolMap, options: &Options) -> Result<TokenStream> {
    let gen = Gen::new(ifc, symbol_map, options);

    let mut output = TokenStream::new();
    output.extend(gen.gen_crate_start()?);
    output.extend(gen.gen_types()?);
    output.extend(gen.gen_macros()?);
    Ok(output)
}

impl<'a> Gen<'a> {
    fn gen_crate_start(&self) -> Result<TokenStream> {
        Ok(quote! {
            //! This code was generated by `gen_rust` from C++ definitions, sourced through IFC.
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            #![allow(non_upper_case_globals)]
            #![no_std]
        })
    }

    fn gen_types(&self) -> Result<TokenStream> {
        self.gen_types_for_scope(self.ifc.file_header().global_scope, 50)
    }

    /// Recursively walks a scope and generates type definitions for it.
    fn gen_types_for_scope(&self, parent_scope: ScopeIndex, max_depth: u32) -> Result<TokenStream> {
        if parent_scope == 0 {
            bail!("Invalid scope");
        }

        let mut output = TokenStream::new();

        info!(
            "Scope #{}{}",
            parent_scope,
            if parent_scope == self.ifc.file_header().global_scope {
                " - Global scope"
            } else {
                ""
            }
        );

        if max_depth == 0 {
            bail!("Max depth exceeded!");
        }

        let _max_depth = max_depth - 1;

        // Add "extern crate foo;" declarations.
        for name in self.symbol_map.crates.iter() {
            let crate_ident = Ident::new(name, Span::call_site());
            output.extend(quote! {
                extern crate #crate_ident;
            });
        }

        let mut alias_defs = TokenStream::new();
        let mut extern_stdcall_funcs = TokenStream::new(); // CallingConvention::Std
        let mut extern_cdecl_funcs = TokenStream::new(); // CallingConvention::Cdecl
        let mut extern_fastcall_funcs = TokenStream::new(); // CallingConvention::Fast
        let mut struct_defs = TokenStream::new();

        for member_decl_index in self.ifc.iter_scope(parent_scope)? {
            match member_decl_index.tag() {
                DeclSort::ALIAS => {
                    let decl_alias = self.ifc.decl_alias().entry(member_decl_index.index())?;
                    let alias_name = self.ifc.get_string(decl_alias.name)?;

                    if self.symbol_map.is_symbol_in(alias_name) {
                        debug!("alias {} is defined in external crate", alias_name);
                    } else {
                        debug!("alias {} - adding", alias_name);
                        let alias_ident = syn::Ident::new(alias_name, Span::call_site());
                        let aliasee_tokens = self.get_type_tokens(decl_alias.aliasee)?;
                        alias_defs.extend(quote! {
                            pub type #alias_ident = #aliasee_tokens;
                        });
                    }
                }

                DeclSort::FUNCTION => {
                    if let Some((convention, func_tokens)) = self.gen_function(member_decl_index)? {
                        // Write the extern function declaration to the right extern "X" { ... } block.
                        let extern_block = match convention {
                            CallingConvention::Std => &mut extern_stdcall_funcs,
                            CallingConvention::Cdecl => &mut extern_cdecl_funcs,
                            CallingConvention::Fast => &mut extern_fastcall_funcs,
                            _ => bail!(
                                "Function calling convention {:?} is not supported",
                                convention
                            ),
                        };
                        extern_block.extend(func_tokens);
                    }
                }
                DeclSort::METHOD => {}

                DeclSort::SCOPE => {
                    struct_defs.extend(self.gen_struct(member_decl_index)?);
                }

                DeclSort::ENUMERATION => {
                    let en = self.ifc.decl_enum().entry(member_decl_index.index())?;
                    let en_name = self.ifc.get_string(en.name)?;
                    if self.symbol_map.is_symbol_in(en_name) {
                        debug!("enum {} - defined in external crate", en_name);
                    } else {
                        debug!("enum {} - emitting", en_name);
                        let t = self.gen_enum(&en)?;
                        output.extend(t);
                    }
                }

                DeclSort::VARIABLE => {
                    let t = self.gen_variable(member_decl_index.index())?;
                    output.extend(t);
                }

                DeclSort::INTRINSIC
                | DeclSort::TEMPLATE
                | DeclSort::CONCEPT
                | DeclSort::EXPLICIT_INSTANTIATION
                | DeclSort::EXPLICIT_SPECIALIZATION => {}

                _ => {
                    nyi!();
                    info!("unknown decl: {:?}", member_decl_index);
                }
            }
        }

        output.extend(alias_defs);

        if !extern_stdcall_funcs.is_empty() {
            output.extend(quote! {
                extern "stdcall" {
                    #extern_stdcall_funcs
                }
            });
        }
        if !extern_cdecl_funcs.is_empty() {
            output.extend(quote! {
                extern "C" {
                    #extern_cdecl_funcs
                }
            });
        }
        if !extern_fastcall_funcs.is_empty() {
            output.extend(quote! {
                extern "fastcall" {
                    #extern_fastcall_funcs
                }
            });
        }

        output.extend(struct_defs);

        Ok(output)
    }
}

fn parse_check<T: syn::parse::Parse>(t: &TokenStream) -> Result<()> {
    let tt = t.clone();
    match syn::parse2::<T>(tt) {
        Ok(_) => Ok(()),
        Err(e) => {
            info!(
                "FAILED to parse token stream with expected type.\n\
                   Error: {:?}\n\
                   Token stream:\n{}",
                e, t
            );
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
