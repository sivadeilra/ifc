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

mod config;
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

    renamed_decls: HashMap<DeclIndex, Ident>,
}

#[derive(Default)]
struct GenOutputs {
    // module-wide attribute
    top: TokenStream,

    types: TokenStream,
    consts: TokenStream,
    statics: TokenStream,
    macros: TokenStream,
    aliases: TokenStream,
    extern_cdecl: TokenStream,
    extern_stdcall: TokenStream,
    extern_fastcall: TokenStream,
}

impl GenOutputs {
    fn finish(self) -> TokenStream {
        let mut output = self.top;
        output.extend(self.types);
        output.extend(self.macros);
        output.extend(self.consts);
        output.extend(self.statics);

        let extern_cdecl = self.extern_cdecl;
        let extern_stdcall = self.extern_stdcall;
        let extern_fastcall = self.extern_fastcall;

        if !extern_cdecl.is_empty() {
            output.extend(quote! {
                extern "C" {
                    #extern_cdecl
                }
            });
        }

        if !extern_stdcall.is_empty() {
            output.extend(quote! {
                extern "stdcall" {
                    #extern_stdcall
                }
            });
        }

        if !extern_fastcall.is_empty() {
            output.extend(quote! {
                extern "fastcall" {
                    #extern_fastcall
                }
            });
        }

        let aliases = self.aliases;
        output.extend(quote!{
            pub mod __typedefs {
                use super::*;
                #aliases
            }
            pub use __typedefs::*;
        });

        output
    }
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
    /// Maps symbol names that are in the global namespace scope to the IFC which define them.
    /// For example, `"_GUID"` to some index.
    pub map: HashMap<String, RefIndex>,
}

#[derive(Default, Clone)]
pub struct Session {
    pub symbols: SymbolMap,
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
            renamed_decls: Default::default(),
        }
    }
}

pub fn gen_rust(ifc: &Ifc, symbol_map: SymbolMap, options: &Options) -> Result<TokenStream> {
    info!("Global scope: {}", ifc.global_scope());

    let mut gen = Gen::new(ifc, symbol_map, options);

    let mut outputs = GenOutputs::default();

    let renamed_decls = gen.rename_decls(ifc.global_scope())?;
    gen.renamed_decls = renamed_decls;

    outputs.top.extend(gen.gen_crate_start()?);
    outputs.macros.extend(gen.gen_macros()?);
    gen.gen_types(&mut outputs)?;
    gen.find_orphans(&mut outputs)?;
    gen.gen_functions(&mut outputs, ifc.global_scope())?;
    Ok(outputs.finish())
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum ScopeKind {
    Namespace,
    Type,
}

impl<'a> Gen<'a> {
    fn gen_crate_start(&self) -> Result<TokenStream> {
        let extern_crates = self.gen_extern_crate()?;

        Ok(quote! {
            //! This code was generated by `gen_rust` from C++ definitions, sourced through IFC.
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            #![allow(non_upper_case_globals)]
            #![no_std]

            #extern_crates

            #[repr(C)]
            pub struct __Bitfield<const W: usize, T> {
                pub _fake: core::marker::PhantomData<[T; W]>,
                pub value: u32,
            }
        })
    }

    #[inline(never)]
    fn gen_types(&self, outputs: &mut GenOutputs) -> Result<()> {
        self.gen_types_for_scope(outputs, self.ifc.global_scope(), 50, ScopeKind::Namespace)
    }

    /// Walk all scopes and find types that appear to be compiler-generated.
    /// Also, for nested types, we need "hoist" these to global scope.
    /// We do so by building a renaming table, which maps DeclIndex to String.
    #[inline(never)]
    fn rename_decls(&self, parent_scope: ScopeIndex) -> Result<HashMap<DeclIndex, Ident>> {
        let mut decl_names: HashMap<DeclIndex, Ident> = HashMap::new();
        debug!("rename_decls: start");
        self.rename_decls_rec(parent_scope, None, &mut decl_names)?;
        debug!("rename_decls: end, num renamed = {}", decl_names.len());
        Ok(decl_names)
    }

    #[inline(never)]
    fn rename_decls_rec(
        &self,
        parent_scope: ScopeIndex,
        name_basis_opt: Option<&str>,
        decl_names: &mut HashMap<DeclIndex, Ident>,
    ) -> Result<()> {
        let mut new_name: String = String::with_capacity(200);
        let mut fixed_name: String = String::with_capacity(100);

        let mut anon_name_counter: u32 = 0;
        // let mut overloaded_func_name_counter: u32 = 0;

        for member_decl in self.ifc.iter_scope(parent_scope)? {
            new_name.clear();
            fixed_name.clear();

            match member_decl.tag() {
                DeclSort::SCOPE => {
                    let nested_scope = self.ifc.decl_scope().entry(member_decl.index())?;
                    if self.ifc.is_type_namespace(nested_scope.ty)? {
                        // ignore namespaces for now
                    } else {
                        // It's a nested type.  Is the name compiler-generated?
                        let scope_name = self.ifc.get_name_string(nested_scope.name)?;
                        let is_gen = is_name_compiler_generated(scope_name);
                        let this_name: &str = if is_gen {
                            fixed_name.clear();
                            fixed_name.push_str(scope_name);
                            fixup_anon_names(&mut fixed_name, &mut anon_name_counter);
                            debug!(
                                "fixed compiler-generated name: {} -> {}",
                                scope_name, fixed_name
                            );
                            &fixed_name
                        } else {
                            trace!("ordinary scope name: {}", scope_name);
                            scope_name
                        };

                        new_name.clear();

                        if let Some(name_basis) = name_basis_opt {
                            // We are in a nested scope. We will need to rename this type, no
                            // matter what.
                            new_name.push_str(name_basis);
                            new_name.push_str("__");
                            new_name.push_str(this_name);
                            debug!("renaming type into global scope: {}", new_name);
                        } else {
                            // We are not in a nested scope. We only need to rename this type if
                            // it is compiler-generated.
                            new_name.push_str(this_name);
                        }

                        if name_basis_opt.is_some() || is_gen {
                            decl_names
                                .insert(member_decl, Ident::new(&new_name, Span::call_site()));
                        }

                        // Recursively evaluate nested scope.
                        if nested_scope.initializer != 0 {
                            self.rename_decls_rec(
                                nested_scope.initializer,
                                Some(&new_name),
                                decl_names,
                            )?;
                        }
                    }
                }

                DeclSort::FUNCTION => {
                    // TODO
                    /*
                    // Functions can be overloaded.
                    let func = self.ifc.decl_function().entry(member_decl.index())?;
                    let func_name = self.ifc.get_name_string(func.name)?;
                    if let Some(existing) = decl_names.get() {
                        let new_func_name = format!("{}_{:04}", overloaded_func_name_counter);
                        info!("uh oh!  overloaded functions!  {} -> {}", func_name, new_func_name);

                        overloaded_func_name_counter += 1;
                        let new_func_ident = Ident::new(&new_func_name, Span::call_site());
                        decl_names.insert
                    }
                    */
                }

                DeclSort::ENUMERATION => {
                    // TODO
                }

                _ => {
                    // ignore
                }
            }
        }

        Ok(())
    }

    fn gen_extern_crate(&self) -> Result<TokenStream> {
        // Add "extern crate foo;" declarations.
        let mut output = TokenStream::new();
        for name in self.symbol_map.crates.iter() {
            let crate_ident = Ident::new(name, Span::call_site());
            output.extend(quote! {
                extern crate #crate_ident;
            });
        }
        Ok(output)
    }

    /// Recursively walks a scope and generates type definitions for it.
    #[inline(never)]
    fn gen_types_for_scope(
        &self,
        outputs: &mut GenOutputs,
        parent_scope: ScopeIndex,
        max_depth: u32,
        _scope_kind: ScopeKind,
    ) -> Result<()> {
        debug!(
            "Scope #{}{}",
            parent_scope,
            if parent_scope + 1 == self.ifc.file_header().global_scope {
                " - Global scope"
            } else {
                ""
            }
        );

        if max_depth == 0 {
            bail!("Max depth exceeded!");
        }

        let _max_depth = max_depth - 1;

        let mut counter: u32 = 0;

        for member_decl_index in self.ifc.iter_scope(parent_scope)? {
            match member_decl_index.tag() {
                DeclSort::ALIAS => {
                    if false {
                        let decl_alias = self.ifc.decl_alias().entry(member_decl_index.index())?;
                        let mut alias_name = self.ifc.get_string(decl_alias.name)?.to_string();
                        fixup_anon_names(&mut alias_name, &mut counter);

                        if self.symbol_map.is_symbol_in(&alias_name) {
                            debug!("alias {} is defined in external crate", alias_name);
                        } else {
                            debug!("alias {} - adding", alias_name);
                            let alias_ident = syn::Ident::new(&alias_name, Span::call_site());
                            let aliasee_tokens = self.get_type_tokens(decl_alias.aliasee)?;

                            outputs.aliases.extend(quote! {
                                pub type #alias_ident = #aliasee_tokens;
                            });
                        }
                    }
                }

                DeclSort::FUNCTION => {
                    // Functions are processed in a later pass.
                }

                DeclSort::METHOD => {}

                DeclSort::SCOPE => {
                    let nested_scope = self.ifc.decl_scope().entry(member_decl_index.index())?;

                    // What kind of scope is it?
                    if self.ifc.is_type_namespace(nested_scope.ty)? {
                        // We do not yet process nested namespaces.
                    } else {
                        // It's a nested struct/class.
                        outputs.types.extend(self.gen_struct(member_decl_index)?);

                        if nested_scope.initializer != 0 {
                            debug!(
                                "gen_types_for_scope: recursing, to scope #{} (zero-based)",
                                nested_scope.initializer
                            );
                            self.gen_types_for_scope(
                                outputs,
                                nested_scope.initializer,
                                max_depth - 1,
                                ScopeKind::Type,
                            )?;
                        } else {
                            debug!("gen_types_for_scope: not recursing, because this is a forward decl only");
                        }
                    }
                }

                DeclSort::ENUMERATION => {
                    let en = self.ifc.decl_enum().entry(member_decl_index.index())?;
                    let en_name = self.ifc.get_string(en.name)?;
                    if self.symbol_map.is_symbol_in(en_name) {
                        debug!("enum {} - defined in external crate", en_name);
                    } else {
                        debug!("enum {} - emitting", en_name);
                        let t = self.gen_enum(&en)?;
                        outputs.types.extend(t);
                    }
                }

                DeclSort::VARIABLE => {
                    self.gen_variable(member_decl_index.index(), outputs)?;
                }

                DeclSort::INTRINSIC
                | DeclSort::TEMPLATE
                | DeclSort::CONCEPT
                | DeclSort::EXPLICIT_INSTANTIATION
                | DeclSort::FIELD
                | DeclSort::BITFIELD
                | DeclSort::EXPLICIT_SPECIALIZATION => {}

                _ => {
                    nyi!();
                    info!("unknown decl: {:?}", member_decl_index);
                }
            }
        }

        Ok(())
    }

    /// Recursively walks a scope and generates type definitions for it.
    #[inline(never)]
    fn gen_functions(&self, outputs: &mut GenOutputs, parent_scope: ScopeIndex) -> Result<()> {
        let mut names_map: HashMap<Ident, u32> = HashMap::new();

        let mut num_errors: u32 = 0;

        for member in self.ifc.iter_scope(parent_scope)? {
            match member.tag() {
                DeclSort::FUNCTION => {
                    match self.gen_function(member, &mut names_map) {
                        Ok(Some((convention, func_tokens))) => {
                            // Write the extern function declaration to the right extern "X" { ... } block.
                            let extern_block = match convention {
                                CallingConvention::Std => &mut outputs.extern_stdcall,
                                CallingConvention::Cdecl => &mut outputs.extern_cdecl,
                                CallingConvention::Fast => &mut outputs.extern_fastcall,
                                _ => bail!(
                                    "Function calling convention {:?} is not supported",
                                    convention
                                ),
                            };
                            extern_block.extend(func_tokens);
                        }
                        Ok(None) => {}
                        Err(_) => {
                            num_errors += 1;
                        }
                    }
                }

                _ => {
                    // Ignore all other decls.
                }
            }
        }

        Ok(())
    }

    /// Emits a forward declaration for a struct that has no definition at all, e.g. `struct Foo;`
    fn emit_forward_decl_scope(
        &self,
        member_decl_index: DeclIndex,
        outputs: &mut GenOutputs,
    ) -> Result<()> {
        let decl_scope = self.ifc.decl_scope().entry(member_decl_index.index())?;
        if decl_scope.initializer != 0 {
            // Nah, it's not a forward decl. Ignore it.
            return Ok(());
        }

        let t = self.gen_struct(member_decl_index)?;
        outputs.types.extend(t);
        Ok(())
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

pub fn is_name_compiler_generated(s: &str) -> bool {
    s.starts_with('<')
}

// <unnamed-tag>
// <unnamed-type-Foo>           // used for fields
// <unnamed-enum-Foo>           // used for fields
pub fn fixup_anon_names(s: &mut String, counter: &mut u32) {
    if s == "<unnamed-tag>" {
        *counter += 1;
        *s = format!("tag{:04}", *counter);
        return;
    }

    if s == "type" {
        *s = "type_".to_string();
        return;
    }

    if s == "Self" {
        *s = "Self_".to_string();
        return;
    }

    if s == "self" {
        *s = "self_".to_string();
        return;
    }

    if !s.starts_with('<') {
        return;
    }

    let mut ss: &str = s;
    ss = ss.strip_prefix("<unnamed-type-").unwrap_or(ss);
    s.strip_prefix('<').unwrap_or(ss);
    ss = ss.strip_suffix('>').unwrap_or(ss);

    let mut out = String::with_capacity(ss.len() + 20);
    for c in ss.chars() {
        match c {
            '<' => out.push_str("__lt"),
            '>' => out.push_str("__gt"),
            '-' => out.push_str("_"),
            c => out.push(c),
        }
    }

    *s = out;
}

pub struct SymbolRemaps {
    /// If an entry is present in this table, then it represents a new name that we have computed
    /// for a given type.
    pub decl_name_remap: HashMap<DeclIndex, String>,
}

impl<'a> Gen<'a> {
    /// Walk all the scopes, see if there are declarations that are not part of any scope.
    fn find_orphans(&self, outputs: &mut GenOutputs) -> Result<()> {
        let ifc = self.ifc;

        struct State {
            decl_scope_found: Vec<bool>,
        }

        let mut state = State {
            decl_scope_found: vec![false; ifc.decl_scope().entries.len()],
        };

        fn search_scope(ifc: &Ifc, state: &mut State, parent_scope: ScopeIndex) -> Result<()> {
            for member_decl in ifc.iter_scope(parent_scope)? {
                match member_decl.tag() {
                    DeclSort::FUNCTION
                    | DeclSort::FIELD
                    | DeclSort::ALIAS
                    | DeclSort::TEMPLATE
                    | DeclSort::EXPLICIT_INSTANTIATION
                    | DeclSort::EXPLICIT_SPECIALIZATION
                    | DeclSort::ENUMERATOR
                    | DeclSort::ENUMERATION
                    | DeclSort::VARIABLE
                    | DeclSort::CONSTRUCTOR
                    | DeclSort::METHOD
                    | DeclSort::DESTRUCTOR
                    | DeclSort::INTRINSIC
                    | DeclSort::BITFIELD => {}

                    DeclSort::SCOPE => {
                        let nested_scope = ifc.decl_scope().entry(member_decl.index())?;
                        if state.decl_scope_found[member_decl.index() as usize] {
                            warn!("found scope twice!  {:?}", member_decl);
                        } else {
                            state.decl_scope_found[member_decl.index() as usize] = true;
                            if nested_scope.initializer != 0 {
                                search_scope(ifc, state, nested_scope.initializer)?;
                            }
                        }
                    }

                    _ => todo!("unrecognized member decl: {:?}", member_decl),
                }
            }

            Ok(())
        }

        search_scope(ifc, &mut state, ifc.global_scope())?;

        for (i, value) in state.decl_scope_found.iter().enumerate() {
            if !*value {
                let nested_scope = ifc.decl_scope().entry(i as u32)?;
                let nested_scope_name = ifc.get_name_string(nested_scope.name)?;
                debug!(
                    "scope #{} not found:  {}   forward decl? {}",
                    i,
                    nested_scope_name,
                    nested_scope.initializer == 0
                );

                // If it looks like a forward declaration, then let's emit a type for it.
                if nested_scope.initializer == 0 {
                    self.emit_forward_decl_scope(
                        DeclIndex::new(DeclSort::SCOPE, i as u32),
                        outputs,
                    )?;
                }
            }
        }

        Ok(())
    }
}
