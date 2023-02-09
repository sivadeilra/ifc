//! Generates Rust code from IFC modules

#![forbid(unused_must_use)]
#![allow(clippy::too_many_arguments)]

use anyhow::{bail, Result};
use ifc::*;
use log::{debug, info, trace, warn};
pub use options::*;
use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::HashSet;
use std::sync::Arc;
use syn::Ident;
use type_discovery::TypeDiscovery;

mod config;
mod enums;
mod expr;
mod funcs;
mod options;
mod pp;
mod structs;
mod ty;
mod type_discovery;
mod vars;

#[macro_export]
macro_rules! log_error {
    ($code:block -> $result_ty:ty, $context:expr) => {
        match (|| -> Result<$result_ty> { $code })() {
            Ok(value) => Some(value),
            Err(err) => {
                log::error!("{:#}", err.context($context));
                None
            }
        }
    };
}

// This is an alias into the `refs` table.
type RefIndex = usize;

struct Gen<'a> {
    ifc: &'a Ifc,
    options: &'a Options,
    #[allow(dead_code)]
    wk: WellKnown,

    scope_to_contents: HashMap<DeclIndex, HashMap<DeclIndex, Cow<'a, str>>>,
    fully_qualified_names: HashMap<DeclIndex, TokenStream>,
}

#[derive(Default)]
struct GenOutputs {
    // module-wide attribute
    top: TokenStream,

    // NOTE: We use HashMap instead of BTreeMap since it is faster in general
    // and we only care about sorting by key when we generate the final stream.
    scopes: Vec<TokenStream>,
    macros: Vec<TokenStream>,
}

impl GenOutputs {
    fn finish(self) -> TokenStream {
        let mut output = self.top;
        output.extend(self.scopes);
        output.extend(self.macros);
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
    pub crates: Vec<TokenStream>,
    /// Maps symbol names that are in the global namespace scope to the IFC which defines them.
    /// For example, `"_GUID"` to some index.
    pub global_symbols: HashMap<String, RefIndex>,
    /// Maps object-like macro names to the IFC which defines them.
    pub object_like_macros: HashMap<String, RefIndex>,
    /// Maps function-like macro names to the IFC which defines them.
    pub function_like_macros: HashMap<String, RefIndex>,
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
    /// For now, we only process the root scope.  So no nested namespaces.
    pub fn add_ref_ifc(&mut self, crate_name: TokenStream, ifc: &Ifc) -> Result<RefIndex> {
        let ifc_index = self.crates.len();

        let ifc_name = crate_name.to_string();
        self.crates.push(crate_name);

        let mut num_added: u64 = 0;

        let mut add_symbol = |name: String, map: &mut HashMap<String, RefIndex>| {
            if let Some(existing_index) = map.get_mut(&name) {
                if ifc_index < *existing_index {
                    *existing_index = ifc_index;
                }
            } else {
                // This is the first time we've seen this symbol. Insert using the current IFC index.
                map.insert(name, ifc_index);
            }
            num_added += 1;
        };

        if let Some(scope) = ifc.global_scope() {
            self.add_scope(ifc, &mut add_symbol, scope, "")?;
        }

        // Add object-like macros that pass the filter.
        for object in ifc.macro_object_like().entries.iter() {
            add_symbol(
                ifc.get_string(object.name)?.to_string(),
                &mut self.object_like_macros,
            );
        }

        // Add function-like macros that pass the filter.
        for func_like in ifc.macro_function_like().entries.iter() {
            add_symbol(
                ifc.get_string(func_like.name)?.to_string(),
                &mut self.function_like_macros,
            );
        }

        info!(
            "Number of symbols added for this IFC '{}' (crate #{}): {}",
            ifc_name, ifc_index, num_added
        );
        Ok(ifc_index)
    }

    fn add_scope(
        &mut self,
        ifc: &Ifc,
        add_symbol: &mut impl FnMut(String, &mut HashMap<String, RefIndex>),
        container: ScopeIndex,
        container_fully_qualified_name: &str,
    ) -> Result<()> {
        for member_decl in ifc.iter_scope(container)? {
            match member_decl.tag() {
                DeclSort::SCOPE => {
                    let nested_scope = ifc.decl_scope().entry(member_decl.index())?;
                    if nested_scope.name.tag() == NameSort::IDENTIFIER {
                        let nested_name = format!(
                            "{}::{}",
                            container_fully_qualified_name,
                            ifc.get_string(nested_scope.name.index())?
                        );
                        if ifc.is_type_namespace(nested_scope.ty)? {
                            self.add_scope(
                                ifc,
                                add_symbol,
                                nested_scope.initializer,
                                &nested_name,
                            )?;
                        } else {
                            // It's a nested struct/class.
                            add_symbol(nested_name, &mut self.global_symbols);
                        }
                    } else {
                        warn!("ignoring scope member named: {:?}", nested_scope.name);
                    }
                }

                DeclSort::ALIAS => {
                    let alias = ifc.decl_alias().entry(member_decl.index())?;
                    let alias_name = format!(
                        "{}::{}",
                        container_fully_qualified_name,
                        ifc.get_string(alias.name)?
                    );
                    add_symbol(alias_name, &mut self.global_symbols);
                }

                DeclSort::ENUMERATION => {
                    let en = ifc.decl_enum().entry(member_decl.index())?;
                    let en_name = format!(
                        "{}::{}",
                        container_fully_qualified_name,
                        ifc.get_string(en.name)?
                    );
                    add_symbol(en_name, &mut self.global_symbols);
                }

                DeclSort::INTRINSIC => {}
                DeclSort::TEMPLATE => {}
                DeclSort::EXPLICIT_INSTANTIATION => {}
                DeclSort::EXPLICIT_SPECIALIZATION => {}

                DeclSort::FUNCTION => {
                    let func_decl = ifc.decl_function().entry(member_decl.index())?;
                    match func_decl.name.tag() {
                        NameSort::IDENTIFIER => {
                            let func_name = format!(
                                "{}::{}",
                                container_fully_qualified_name,
                                ifc.get_string(func_decl.name.index())?
                            );
                            add_symbol(func_name, &mut self.global_symbols);
                        }
                        _ => {
                            warn!("ignoring function named: {:?}", func_decl.name);
                        }
                    }
                }

                DeclSort::VARIABLE => {
                    let var_decl = ifc.decl_var().entry(member_decl.index())?;
                    match var_decl.name.tag() {
                        NameSort::IDENTIFIER => {
                            let var_name = format!(
                                "{}::{}",
                                container_fully_qualified_name,
                                ifc.get_string(var_decl.name.index())?
                            );
                            add_symbol(var_name, &mut self.global_symbols);
                        }
                        _ => {
                            warn!("ignoring var named: {:?}", var_decl.name);
                        }
                    }
                }

                _ => {
                    warn!("ignoring unrecognized scope member: {:?}", member_decl);
                }
            }
        }

        Ok(())
    }

    pub fn is_object_like_macro_in(&self, name: &str) -> bool {
        self.object_like_macros.contains_key(name)
    }

    pub fn is_function_like_macro_in(&self, name: &str) -> bool {
        self.function_like_macros.contains_key(name)
    }

    pub fn resolve(&self, name: &str) -> Option<&TokenStream> {
        let crate_index = *self.global_symbols.get(name)?;
        Some(&self.crates[crate_index])
    }

    pub fn resolve_object_like_macro(&self, name: &str) -> Option<&TokenStream> {
        let crate_index = *self.object_like_macros.get(name)?;
        Some(&self.crates[crate_index])
    }

    pub fn resolve_function_like_macro(&self, name: &str) -> Option<&TokenStream> {
        let crate_index = *self.function_like_macros.get(name)?;
        Some(&self.crates[crate_index])
    }
}

impl<'a> Gen<'a> {
    fn new(
        ifc: &'a Ifc,
        options: &'a Options,
        scope_to_contents: HashMap<DeclIndex, HashMap<DeclIndex, Cow<'a, str>>>,
        fully_qualified_names: HashMap<DeclIndex, TokenStream>,
    ) -> Self {
        Self {
            ifc,
            options,
            wk: WellKnown {
                tokens_empty: quote!(),
                tokens_false: quote!(false),
                tokens_true: quote!(true),
            },
            scope_to_contents,
            fully_qualified_names,
        }
    }
}

pub fn gen_rust(ifc: &Ifc, symbol_map: SymbolMap, options: &Options) -> Result<TokenStream> {
    let renamed_decls = ifc
        .global_scope()
        .map(|global_scope| Gen::rename_decls(ifc, global_scope))
        .transpose()?
        .unwrap_or_default();

    let type_and_crate_info = TypeDiscovery::walk_global_scope(
        ifc,
        &symbol_map,
        renamed_decls,
        options.type_filter(),
        options.function_filter(),
        options.variable_filter(),
        &options.rust_mod_name,
    );

    let gen = Gen::new(
        ifc,
        options,
        type_and_crate_info.scope_to_contents,
        type_and_crate_info.fully_qualified_names,
    );

    let mut outputs = GenOutputs::default();

    if options.standalone {
        outputs.top.extend(gen.gen_standalone_crate_header()?);
    }
    outputs
        .top
        .extend(gen.gen_common_module_header(&symbol_map.crates)?);
    gen.gen_macros(&symbol_map, options.macro_filter(), &mut outputs.macros)?;

    gen.gen_types(&mut outputs.scopes)?;
    gen.emit_orphans(type_and_crate_info.orphans, &mut outputs.scopes)?;
    Ok(outputs.finish())
}

impl<'a> Gen<'a> {
    fn gen_standalone_crate_header(&self) -> Result<TokenStream> {
        Ok(quote! {
            //! This code was generated by `gen_rust` from C++ definitions, sourced through IFC.
            #![allow(non_camel_case_types)]
            #![allow(non_snake_case)]
            #![allow(non_upper_case_globals)]
            #![allow(dead_code)]
            #![allow(unused_imports)]
            #![allow(improper_ctypes)]
            #![no_std]
        })
    }

    fn gen_common_module_header(&self, extern_crates: &Vec<TokenStream>) -> Result<TokenStream> {
        let extern_crates = self.gen_extern_crate(extern_crates)?;

        Ok(quote! {
            #extern_crates

            #[repr(C)]
            pub struct __Bitfield<const W: usize, T> {
                pub _fake: core::marker::PhantomData<[T; W]>,
                pub value: u32,
            }
        })
    }

    #[inline(never)]
    fn gen_types(&self, outputs: &mut Vec<TokenStream>) -> Result<()> {
        if let Some(global_scope) = self.ifc.global_scope() {
            self.gen_members_for_scope(
                self.scope_to_contents
                    .get(&DeclIndex(0))
                    .expect("Must have an entry for the global scope"),
                outputs,
                global_scope,
            )
        } else {
            Ok(())
        }
    }

    /// Walk all scopes and find types that appear to be compiler-generated.
    /// Also, for nested types, we need "hoist" these to global scope.
    /// We do so by building a renaming table, which maps DeclIndex to String.
    #[inline(never)]
    fn rename_decls(ifc: &Ifc, parent_scope: ScopeIndex) -> Result<HashMap<DeclIndex, Ident>> {
        let mut decl_names: HashMap<DeclIndex, Ident> = HashMap::new();
        debug!("rename_decls: start");
        Gen::rename_decls_rec(ifc, parent_scope, None, &mut decl_names)?;
        debug!("rename_decls: end, num renamed = {}", decl_names.len());
        Ok(decl_names)
    }

    #[inline(never)]
    fn rename_decls_rec(
        ifc: &Ifc,
        parent_scope: ScopeIndex,
        name_basis_opt: Option<&str>,
        decl_names: &mut HashMap<DeclIndex, Ident>,
    ) -> Result<()> {
        let mut new_name: String = String::with_capacity(200);
        let mut fixed_name: String = String::with_capacity(100);

        let mut anon_name_counter: u32 = 0;
        // let mut overloaded_func_name_counter: u32 = 0;

        for member_decl in ifc.iter_scope(parent_scope)? {
            new_name.clear();
            fixed_name.clear();

            match member_decl.tag() {
                DeclSort::SCOPE => {
                    let nested_scope = ifc.decl_scope().entry(member_decl.index())?;
                    if ifc.is_type_namespace(nested_scope.ty)? {
                        // ignore namespaces for now
                    } else {
                        // It's a nested type.  Is the name compiler-generated?
                        let scope_name = ifc.get_name_string(nested_scope.name)?;
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
                            Gen::rename_decls_rec(
                                ifc,
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

    fn gen_extern_crate(&self, extern_crates: &Vec<TokenStream>) -> Result<TokenStream> {
        // Gather unique external crate names
        let mut extern_crates = extern_crates
            .iter()
            .filter_map(|qualified_name| {
                let mut crate_name = qualified_name.to_string();
                if let Some(qualifier_index) = crate_name.find("::") {
                    crate_name.truncate(qualifier_index);
                }
                crate_name.truncate(crate_name.trim_end().len());
                (crate_name != "crate").then(|| crate_name)
            })
            .collect::<Vec<_>>();
        extern_crates.sort();
        extern_crates.dedup();

        // Add "extern crate foo;" declarations.
        let mut output = TokenStream::new();
        for crate_name in extern_crates {
            let ident = Ident::new(&crate_name, Span::call_site());
            output.extend(quote! {
                extern crate #ident;
            });
        }
        Ok(output)
    }

    /// Recursively walks a scope and generates member definitions for it.
    #[inline(never)]
    fn gen_members_for_scope(
        &self,
        filtered_contents: &HashMap<DeclIndex, Cow<'_, str>>,
        outputs: &mut Vec<TokenStream>,
        parent_scope: ScopeIndex,
    ) -> Result<()> {
        let mut names_map = HashMap::new();

        for (name, member_decl_index) in self
            .ifc
            .iter_scope(parent_scope)?
            .filter_map(|item| filtered_contents.get(&item).zip(Some(item)))
        {
            debug!(
                "{:?}> emitting {} ({})",
                member_decl_index,
                name,
                self.fully_qualified_names.get(&member_decl_index).unwrap()
            );

            log_error! { {
                match member_decl_index.tag() {
                    DeclSort::ALIAS => {
                        let decl_alias = self.ifc.decl_alias().entry(member_decl_index.index())?;

                        let alias_ident = syn::Ident::new(&name, Span::call_site());
                        let aliasee_tokens = self.get_type_tokens(decl_alias.aliasee)?;

                        outputs.push(quote! {
                            pub type #alias_ident = #aliasee_tokens;
                        });
                    }

                    DeclSort::FUNCTION => {
                        outputs.push(self.gen_function(member_decl_index, &mut names_map, name)?);
                    }

                    DeclSort::SCOPE => {
                        let nested_scope = self.ifc.decl_scope().entry(member_decl_index.index())?;

                        // What kind of scope is it?
                        if self.ifc.is_type_namespace(nested_scope.ty)? {
                            if let Some(filtered_contents) = self.scope_to_contents.get(&member_decl_index) {
                                let mut members = Vec::new();
                                let ident = Ident::new(name, Span::call_site());
                                self.gen_members_for_scope(
                                    filtered_contents,
                                    &mut members,
                                    nested_scope.initializer,
                                )?;
                                if !members.is_empty() {
                                    outputs.push(
                                        quote!{
                                            pub mod #ident {
                                                #(#members
                                                )*
                                            }
                                        }
                                    );
                                }
                            }
                        } else {
                            // It's a nested struct/class.
                            outputs.push(self.gen_struct(member_decl_index, name)?);
                        }
                    }

                    DeclSort::ENUMERATION => {
                        let en = self.ifc.decl_enum().entry(member_decl_index.index())?;
                        debug!("enum {} - emitting", name);
                        outputs.push(self.gen_enum(en)?);
                    }

                    DeclSort::VARIABLE => {
                        outputs.push(self.gen_variable(
                            member_decl_index.index(),
                            name,
                        )?);
                    }

                    DeclSort::METHOD
                    | DeclSort::FIELD
                    | DeclSort::BITFIELD => {
                        panic!("Methods and fields must only be in types");
                    }

                    DeclSort::INTRINSIC
                    | DeclSort::TEMPLATE
                    | DeclSort::CONCEPT
                    | DeclSort::EXPLICIT_INSTANTIATION
                    | DeclSort::USING_DECLARATION
                    | DeclSort::PARTIAL_SPECIALIZATION
                    | DeclSort::EXPLICIT_SPECIALIZATION => {}

                    _ => {
                        nyi!();
                        info!("unknown decl: {:?}", member_decl_index);
                    }
                }
                Ok(())
            } -> (), format!("Generating member {:?} {}", member_decl_index.tag(), name) };
        }

        Ok(())
    }

    /// Emits a forward declaration for a struct that has no definition at all, e.g. `struct Foo;`
    fn emit_forward_decl_scope(
        &self,
        member_decl_index: DeclIndex,
        name: &str,
        outputs: &mut Vec<TokenStream>,
    ) -> Result<()> {
        let decl_scope = self.ifc.decl_scope().entry(member_decl_index.index())?;
        if decl_scope.initializer != 0 {
            // Nah, it's not a forward decl. Ignore it.
            return Ok(());
        }

        outputs.push(self.gen_struct(member_decl_index, name)?);

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
            '-' => out.push('_'),
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
    /// Emit declarations that are not part of any scope.
    fn emit_orphans(
        &self,
        orphans: Vec<(DeclIndex, Cow<'_, str>)>,
        outputs: &mut Vec<TokenStream>,
    ) -> Result<()> {
        for (decl_index, name) in orphans {
            self.emit_forward_decl_scope(decl_index, &name, outputs)?;
        }

        Ok(())
    }
}
