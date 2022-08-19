//! Generates Rust code from IFC modules

#![allow(unused_imports)]
#![forbid(unused_must_use)]

use anyhow::{bail, Result};
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

mod pp;

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
            "Number of symbols added for this IFC '{}': #{} {}",
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
        })
    }

    fn gen_types(&self) -> Result<TokenStream> {
        self.gen_types_for_scope(self.ifc.file_header().global_scope, 50)
    }

    /// Recursively walks a scope and generates type definitions for it.
    fn gen_types_for_scope(&self, parent_scope: ScopeIndex, max_depth: u32) -> Result<TokenStream> {
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
        // `scope.descriptor` gives us the start and length of the region in `scope.members` where
        // the members for this scope can be found.
        let scope_descriptor = self.ifc.scope_desc().entry(parent_scope - 1)?;

        if max_depth == 0 {
            bail!("Max depth exceeded!");
        }

        let _max_depth = max_depth - 1;

        // Add "extern crate foo;" declarations.
        for name in self.symbol_map.crates.iter() {
            let crate_ident = Ident::new(name, Span::call_site());
            output.extend(quote!{
                extern crate #crate_ident;
            });
        }

        let mut alias_defs = TokenStream::new();
        let mut extern_c_funcs = TokenStream::new();
        let mut struct_defs = TokenStream::new();

        let scope_members = self.ifc.scope_member();
        for member_index in
            scope_descriptor.start..scope_descriptor.start + scope_descriptor.cardinality
        {
            let member_decl_index: DeclIndex = *scope_members.entry(member_index)?;

            match member_decl_index.tag() {
                DeclSort::ALIAS => {
                    let decl_alias = self.ifc.decl_alias().entry(member_decl_index.index())?;
                    let alias_name = self.ifc.get_string(decl_alias.name)?;

                    if self.symbol_map.is_symbol_in(alias_name) {
                        debug!("alias {} is defined in external crate", alias_name);
                    } else {
                        debug!("alias {} - adding", alias_name);
                        let alias_ident = syn::Ident::new(alias_name, Span::call_site());
                        alias_defs.extend(quote! {
                            pub type #alias_ident = ();
                        });
                    }
                }

                DeclSort::FUNCTION => {
                    let func_decl = self.ifc.decl_function().entry(member_decl_index.index())?;
                    match func_decl.name.tag() {
                        NameSort::IDENTIFIER => {
                            let func_name = self.ifc.get_string(func_decl.name.index())?;

                            if self.symbol_map.is_symbol_in(func_name) {
                                debug!("function {} - defined in external crate", func_name);
                            } else {
                                let func_ident = syn::Ident::new(func_name, Span::call_site());
                                extern_c_funcs.extend(quote! {
                                    pub fn #func_ident();
                                });
                            }

                            // let _type_str = self.ifc.get_type_string(func_decl.type_)?;
                        }
                        _ => {
                            // For now, we ignore all other kinds of functions.
                            debug!("ignoring function named {:?}", func_decl.name);
                        }
                    };
                }

                DeclSort::SCOPE => {
                    let nested_scope = self.ifc.decl_scope().entry(member_decl_index.index())?;

                    // What kind of scope is it?
                    if self.ifc.is_type_namespace(nested_scope.ty)? {
                        // We do not yet process namespaces.
                    } else {
                        // It's a nested struct/class.
                        let nested_scope_name = self.ifc.get_string(nested_scope.name.index())?;
                        if nested_scope.initializer != 0 {
                            if self.symbol_map.is_symbol_in(nested_scope_name) {
                                debug!("struct {} - defined in external crate", nested_scope_name);
                            } else {
                                // Emit the definition for this struct.

                                debug!("struct {} - emitting", nested_scope_name);
                                let mut struct_contents = TokenStream::new();
                                for member_decl in self.ifc.iter_scope(nested_scope.initializer)? {
                                    match member_decl.tag() {
                                        DeclSort::FIELD => {
                                            let field_decl =
                                                self.ifc.decl_field().entry(member_decl.index())?;
                                            let field_name =
                                                self.ifc.get_string(field_decl.name)?;
                                            let field_ident =
                                                syn::Ident::new(field_name, Span::call_site());
                                            let field_type_tokens =
                                                self.get_type_tokens(field_decl.ty)?;

                                            struct_contents.extend(
                                                quote! { pub #field_ident: #field_type_tokens, },
                                            );
                                        }

                                        _ => {
                                            // Ignore everything else, for now.
                                        }
                                    }
                                }

                                let struct_ident =
                                    syn::Ident::new(nested_scope_name, Span::call_site());

                                struct_defs.extend(quote! {
                                    #[repr(C)]
                                    pub struct #struct_ident {
                                        #struct_contents
                                    }
                                });
                            }
                        } else {
                            // This struct has a forward declaration but no definition,
                            // e.g. "struct FOO;".  Not sure what to do about that, yet.
                            debug!("struct {} - ignoring forward decl", nested_scope_name);
                        }
                    }
                }

                DeclSort::METHOD => {}

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

                _ => {
                    nyi!();
                    info!("unknown decl: {:?}", member_decl_index);
                }
            }
        }

        output.extend(alias_defs);
        output.extend(quote! {
            extern "C" {
                #extern_c_funcs
            }
        });
        output.extend(struct_defs);

        Ok(output)
    }

    fn gen_variable(&self, var_index: u32) -> Result<TokenStream> {
        let var = self.ifc.decl_var().entry(var_index)?;

        if var.name.tag() != NameSort::IDENTIFIER {
            info!("Found VARIABLE, but its name is not IDENTIFIER.  Ignoring.");
            return Ok(quote!());
        }
        let var_name = self.ifc.get_string(var.name.index())?;
        info!("----- var: {} -----", var_name);

        let var_ident = Ident::new(&var_name, Span::call_site());

        info!("VarDecl: {}", var_name);
        info!("{:?}", var);

        let is_const;
        if var.traits.contains(ObjectTraits::CONSTEXPR) {
            is_const = true;
        } else {
            if self.ifc.is_const_qualified(var.ty)? {
                // If it has a literal initializer, it's a constant.
                if self.ifc.is_literal_expr(var.initializer)? {
                    is_const = true;
                } else {
                    is_const = false;
                }
            } else {
                is_const = false;
            }
        }

        if is_const {
            let ty_tokens = self.get_type_tokens(var.ty)?;
            let init_tokens = self.gen_expr_tokens(var.ty, var.initializer)?;
            Ok(quote! {
                pub const #var_ident: #ty_tokens = #init_tokens;
            })
            // } else if var.specifier.contains(BasicSpecifiers::EXTERNAL) {
        } else {
            // This is a variable declaration, not a definition. We can emit an "extern static" item.
            let ty_tokens = self.get_type_tokens(var.ty)?;

            let mut_kw = if self.ifc.is_const_qualified(var.ty)? {
                quote!(mut)
            } else {
                quote!()
            };

            Ok(quote! {
                extern "C" {
                    pub static #mut_kw #var_ident: #ty_tokens;
                }
            })
        }
    }

    fn get_type_tokens(&self, mut type_index: TypeIndex) -> Result<TokenStream> {
        // Remove qualifiers.
        let mut const_qual = false;
        while type_index.tag() == TypeSort::QUALIFIED {
            let qt = self.ifc.type_qualified().entry(type_index.index())?;
            if qt.qualifiers.contains(Qualifiers::CONST) {
                const_qual = true;
            }
            type_index = qt.unqualified_type;
        }

        Ok(match type_index.tag() {
            TypeSort::FUNDAMENTAL => {
                let fun_ty = self.ifc.type_fundamental().entry(type_index.index())?;
                let is_signed = matches!(fun_ty.sign, TypeSign::SIGNED | TypeSign::PLAIN);

                match fun_ty.basis {
                    TypeBasis::INT => match (fun_ty.precision, is_signed) {
                        (
                            TypePrecision::DEFAULT | TypePrecision::LONG | TypePrecision::BIT32,
                            true,
                        ) => {
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
                        (TypePrecision::SHORT, true) => quote!(i16),
                        (TypePrecision::SHORT, false) => quote!(u16),
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

                    TypeBasis::BOOL => quote!(bool),

                    TypeBasis::WCHAR_T => quote!(u16),

                    _ => todo!("TypeBasis: {:?}", fun_ty.basis),
                }
            }

            TypeSort::POINTER => {
                let pointed_ty = *self.ifc.type_pointer().entry(type_index.index())?;
                let pointed_ty_tokens = self.get_type_tokens(pointed_ty)?;
                if const_qual {
                    quote! {*const #pointed_ty_tokens}
                } else {
                    quote! {*mut #pointed_ty_tokens}
                }
            }

            TypeSort::ARRAY => {
                let type_array = self.ifc.type_array().entry(type_index.index())?;
                let element_tokens = self.get_type_tokens(type_array.element)?;
                let extent_tokens = if type_array.extent.tag() == ExprSort::EMPTY {
                    quote!(_)
                } else {
                    quote!(42)
                    // gen_expr_tokens(ifc, type_array.extent)?
                };

                quote! {
                    [#element_tokens; #extent_tokens]
                }
            }

            TypeSort::DESIGNATED => {
                let desig_decl = self.ifc.type_designated().entry(type_index.index())?;
                let desig_name: &str = match desig_decl.tag() {
                    DeclSort::SCOPE => {
                        let scope_decl = self.ifc.decl_scope().entry(desig_decl.index())?;
                        match scope_decl.name.tag() {
                            NameSort::IDENTIFIER => self.ifc.get_string(scope_decl.name.index())?,
                            _ => todo!(
                                "designated type {:?} references unrecognized name {:?}",
                                desig_decl,
                                scope_decl.name
                            ),
                        }
                    }

                    DeclSort::ENUMERATION => {
                        let enum_decl = self.ifc.decl_enum().entry(desig_decl.index())?;
                        self.ifc.get_string(enum_decl.name)?
                    }

                    DeclSort::ALIAS => {
                        let alias_decl = self.ifc.decl_alias().entry(desig_decl.index())?;
                        self.ifc.get_string(alias_decl.name)?
                    }

                    _ => todo!("unrecognized designated type: {:?}", desig_decl),
                };

                if let Some(extern_crate) = self.symbol_map.resolve(desig_name) {
                    // This designated type reference resolves to a name in a dependent crate.
                    trace!("resolved type to external crate: {}", extern_crate);
                    let extern_ident = syn::Ident::new(extern_crate, Span::call_site());
                    let desig_ident = syn::Ident::new(desig_name, Span::call_site());
                    quote! { #extern_ident :: #desig_ident}
                } else {
                    // This designated type references something in this crate.
                    Ident::new(desig_name, Span::call_site()).to_token_stream()
                }
            }

            _ => todo!("unrecognized type sort: {:?}", type_index),
        })
    }

    // This converts literal expressions into token streams.
    fn gen_expr_tokens(&self, ty: ifc::TypeIndex, expr: ifc::ExprIndex) -> Result<TokenStream> {
        let ty = self.ifc.remove_qualifiers(ty)?;

        Ok(match expr.tag() {
            ExprSort::LITERAL => {
                let literal = self.ifc.expr_literal().entry(expr.index())?;
                debug!("literal = {:?}", literal);

                // It appears the "type" field in ExprLiteral is always set to 0, which is
                // VENDOR_EXTENSION.  So we don't actually know the type of the literal.

                if ty.tag() != TypeSort::FUNDAMENTAL {
                    bail!(
                        "gen_expr_tokens: This only works with TypeSort::FUNDAMENTAL, not {:?}",
                        ty
                    );
                }
                let fun_ty = self.ifc.type_fundamental().entry(ty.index())?;
                debug!("gen_expr_tokens: fun_ty {:?}", fun_ty);

                match literal.value.tag() {
                    LiteralSort::IMMEDIATE => {
                        let value: u32 = literal.value.index();
                        trace!("LiteralSort::IMMEDIATE: value = 0x{:x} {}", value, value);
                        if fun_ty.basis == TypeBasis::BOOL {
                            if value != 0 {
                                quote!(true)
                            } else {
                                quote!(false)
                            }
                        } else {
                            let lit = syn::LitInt::new(&value.to_string(), Span::call_site());
                            quote!(#lit)
                        }
                    }
                    LiteralSort::INTEGER => {
                        let value: u64 = *self.ifc.const_i64().entry(literal.value.index())?;
                        trace!("LiteralSort::INTEGER: value = 0x{:x} {}", value, value);
                        if fun_ty.basis == TypeBasis::BOOL {
                            if value != 0 {
                                quote!(true)
                            } else {
                                quote!(false)
                            }
                        } else {
                            if matches!(fun_ty.sign, TypeSign::SIGNED | TypeSign::PLAIN) {
                                let value_i64: i64 = value as i64;
                                if value_i64 < 0 {
                                    if let Some(value_pos) = value_i64.checked_abs() {
                                        let lit = syn::LitInt::new(
                                            &value_pos.to_string(),
                                            Span::call_site(),
                                        );
                                        quote!(-#lit)
                                    } else {
                                        bail!(
                                        "Negative value is -MAX_INT, not sure how to handle that."
                                    );
                                    }
                                } else {
                                    let lit =
                                        syn::LitInt::new(&value.to_string(), Span::call_site());
                                    quote!(#lit)
                                }
                            } else {
                                let lit = syn::LitInt::new(&value.to_string(), Span::call_site());
                                quote!(#lit)
                            }
                        }
                    }
                    LiteralSort::FLOATING_POINT => {
                        todo!("floating point literals")
                    }
                    _ => todo!("unrecognized literal value: {:?}", literal.value),
                }
            }

            ExprSort::DYAD => {
                let dyad = self.ifc.expr_dyad().entry(expr.index())?;
                bail!("ExprSort::DYAD: {:?}", dyad);
            }

            _ => todo!("unsupported expr: {:?}", expr),
        })
    }

    fn get_literal_expr_as_u64(&self, expr: ifc::ExprIndex) -> Result<u64> {
        Ok(match expr.tag() {
            ExprSort::LITERAL => {
                let literal = self.ifc.expr_literal().entry(expr.index())?;
                match literal.value.tag() {
                    LiteralSort::IMMEDIATE => literal.value.index() as u64,
                    LiteralSort::INTEGER => *self.ifc.const_i64().entry(literal.value.index())?,
                    LiteralSort::FLOATING_POINT => {
                        todo!("floating point literals")
                    }
                    _ => todo!("unrecognized literal value: {:?}", literal.value),
                }
            }

            _ => todo!("unsupported expr: {:?}", expr),
        })
    }

    fn gen_enum(&self, enum_decl: &DeclEnum) -> Result<TokenStream> {
        // let en = ifc.decl_enum().entry(member_decl_index.index())?;
        let en_name = self.ifc.get_string(enum_decl.name)?;
        let en_ident = Ident::new(&en_name, Span::call_site());

        // info!("enumeration decl:\n{:#?}", enum_decl);

        // Is this an ordinary enum, or an enum class?
        if enum_decl.ty.tag() != TypeSort::FUNDAMENTAL {
            bail!("Expected DeclEnum.ty to be TypeSort::FUNDAMENTAL");
        }
        let en_ty = self.ifc.type_fundamental().entry(enum_decl.ty.index())?;
        let is_enum_class = match en_ty.basis {
            TypeBasis::CLASS | TypeBasis::STRUCT => true,
            TypeBasis::ENUM => false,
            _ => {
                info!(
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

        let storage_ty: TokenStream = self.get_type_tokens(enum_decl.base)?;

        // Generate the enum type tokens.

        let mut output = quote! {
            #[repr(transparent)]
            #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
            #[cfg_attr(feature = "zerocopy", derive(::zerocopy::AsBytes, ::zerocopy::FromBytes))]
            pub struct #en_ident(pub #storage_ty);
        };

        let mut variants_tokens = TokenStream::new();
        let want_derive_debug = self.options.derive_debug;

        let mut derive_debug_body = TokenStream::new();

        // If an enum has more than one enumerator with the same type, then we need to avoid
        // emitting more than one match arm for that value.
        let mut value_seen: HashSet<u64> = HashSet::new();

        for var_index in enum_decl.initializer.to_range() {
            let var = self.ifc.decl_enumerator().entry(var_index)?;
            let var_name_string = self.ifc.get_string(var.name)?;
            let var_name_ident = Ident::new(&var_name_string, Span::call_site());

            let initializer = self.gen_expr_tokens(enum_decl.base, var.initializer)?;
            variants_tokens.extend(quote! {
                pub const #var_name_ident: #en_ident = #en_ident(#initializer);
            });

            if want_derive_debug {
                let initializer_as_u64 = self.get_literal_expr_as_u64(var.initializer)?;
                if value_seen.insert(initializer_as_u64) {
                    if is_enum_class {
                        derive_debug_body.extend(quote! {
                            Self::#var_name_ident => #var_name_string,
                        });
                    } else {
                        derive_debug_body.extend(quote! {
                            #var_name_ident => #var_name_string,
                        });
                    }
                }
            }
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

        if want_derive_debug {
            output.extend(quote! {
                impl core::fmt::Debug for #en_ident {
                    // If the enum defines an enumerator for every possible value, then we don't
                    // want to report an error.
                    #[allow(unreachable_patterns)]
                    fn fmt(&self, fmt: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                        let s: &str = match *self {
                            #derive_debug_body
                            _ => {
                                return write!(fmt, "({})", self.0);
                            }
                        };
                        fmt.write_str(s)
                    }
                }
            });
        }

        parse_check_mod_items(&output)?;
        Ok(output)
    }
}

fn parse_check<T: syn::parse::Parse>(t: &TokenStream) -> Result<()> {
    let tt = t.clone();
    match syn::parse2::<T>(tt) {
        Ok(_) => Ok(()),
        Err(e) => {
            info!("FAILED to parse token stream with expected type.");
            info!("Error: {:?}", e);
            info!("Token stream:\n{}", t);
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
