use super::*;
use crate::{fixup_anon_names, options::Filter, SymbolMap};
use anyhow::Result;
use log::info;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

struct SkippedType<'ifc> {
    index: DeclIndex,
    name: Cow<'ifc, str>,
    fully_qualified_name: String,
}

/// Discovers types within a scope and maintains a list of included and skipped
/// types.
pub(crate) struct TypeDiscovery<'ifc, 'opt> {
    ifc: &'ifc Ifc,
    symbol_map: &'ifc SymbolMap,
    renamed_decls: &'ifc HashMap<DeclIndex, Ident>,

    type_filter: Filter<'opt>,
    function_filter: Filter<'opt>,
    variable_filter: Filter<'opt>,

    /// Discovered types (from any crate) that are to be included.
    included_types: HashSet<TypeIndex>,

    /// Scopes in the current crate and their contents that are to be emitted.
    scope_to_contents: HashMap<DeclIndex, HashMap<DeclIndex, Cow<'ifc, str>>>,

    /// Discovered types that were skipped while their containing scope was
    /// being walked.
    skipped_types: HashMap<TypeIndex, SkippedType<'ifc>>,
}

impl<'ifc, 'opt> TypeDiscovery<'ifc, 'opt> {
    /// Creates a new TypeDiscovery object.
    fn new(
        ifc: &'ifc Ifc,
        symbol_map: &'ifc SymbolMap,
        renamed_decls: &'ifc HashMap<DeclIndex, Ident>,
        type_filter: Filter<'opt>,
        function_filter: Filter<'opt>,
        variable_filter: Filter<'opt>,
    ) -> Self {
        Self {
            ifc,
            symbol_map,
            renamed_decls,
            type_filter,
            function_filter,
            variable_filter,
            included_types: HashSet::new(),
            scope_to_contents: HashMap::new(),
            skipped_types: HashMap::new(),
        }
    }

    /// Walks the global scope and discovers the set of scopes and their contents to be emitted.
    pub fn walk_global_scope(
        ifc: &'ifc Ifc,
        symbol_map: &'ifc SymbolMap,
        renamed_decls: &'ifc HashMap<DeclIndex, Ident>,
        type_filter: Filter<'opt>,
        function_filter: Filter<'opt>,
        variable_filter: Filter<'opt>,
    ) -> HashMap<DeclIndex, HashMap<DeclIndex, Cow<'ifc, str>>> {
        let mut type_discovery = TypeDiscovery::new(
            ifc,
            symbol_map,
            renamed_decls,
            type_filter,
            function_filter,
            variable_filter,
        );
        log_error! { {
            if let Some(global_scope) = ifc.global_scope() {
                type_discovery
                    .scope_to_contents
                    .insert(
                        DeclIndex(0),
                        HashMap::new(),
                    );
                type_discovery.walk_scope(global_scope, "")
            } else {
                Ok(())
            }
        } -> (), "Walking global scope" };

        type_discovery.scope_to_contents
    }

    /// Include a type (from this crate or another) that are included (directly or transitively) by a filter.
    fn include_type(&mut self, type_index: TypeIndex) {
        if !self.included_types.insert(type_index) {
            // If this type was previously skipped, then re-walk its declaration to discover new
            // types.
            if let Some(SkippedType {
                index,
                name,
                fully_qualified_name,
            }) = self.skipped_types.remove(&type_index)
            {
                log_error! { {
                    match index.tag() {
                        DeclSort::ALIAS => {
                            let decl_alias = self.ifc.decl_alias().entry(index.index())?;
                            self.add_alias_declaration_to_emit(index, decl_alias, name);
                        }

                        DeclSort::SCOPE => {
                            let decl_scope= self.ifc.decl_scope().entry(index.index())?;
                            self.add_type_declaration_to_emit(index, decl_scope, &fully_qualified_name, name)?;
                        }

                        DeclSort::ENUMERATION => {
                            let decl_enum= self.ifc.decl_enum().entry(index.index())?;
                            self.add_enum_declaration_to_emit(index, decl_enum, name);
                        }

                        _ => {
                            bail!("Unexpected skipped type");
                        }
                    }

                    Ok(())
                } -> (), format!("Discovering type {}", &fully_qualified_name) };
            }
        }
    }

    /// Add member to the contents of a scope.
    fn add_member(&mut self, container: DeclIndex, member: DeclIndex, name: Cow<'ifc, str>) {
        self.scope_to_contents
            .entry(container)
            .or_insert_with(|| HashMap::new())
            .insert(member, name);
    }

    /// Adds a type alias declaration to the set to emit, and include its dependencies.
    fn add_alias_declaration_to_emit(
        &mut self,
        index: DeclIndex,
        alias: &DeclAlias,
        name: Cow<'ifc, str>,
    ) {
        self.add_member(alias.home_scope, index, name);
        self.include_type(alias.type_);
        self.include_type(alias.aliasee);
    }

    /// Adds an enum alias declaration to the set to emit, and include its dependencies.
    fn add_enum_declaration_to_emit(
        &mut self,
        index: DeclIndex,
        en: &DeclEnum,
        name: Cow<'ifc, str>,
    ) {
        self.add_member(en.home_scope, index, name);
        self.include_type(en.ty);
        self.include_type(en.base);
    }

    /// Adds a type (struct/class) declaration to the set to emit, and walk its contents.
    fn add_type_declaration_to_emit(
        &mut self,
        index: DeclIndex,
        scope: &DeclScope,
        fully_qualified_name: &str,
        name: Cow<'ifc, str>,
    ) -> Result<()> {
        self.add_member(scope.home_scope, index, name);
        self.include_type(scope.ty);
        self.include_type(scope.base);
        if scope.initializer != 0 {
            self.walk_scope(scope.initializer, fully_qualified_name)
        } else {
            Ok(())
        }
    }

    /// Adds a function declaration to the set to emit, and include its dependencies.
    fn add_function_declaration_to_emit(
        &mut self,
        container: DeclIndex,
        index: DeclIndex,
        function_type: TypeIndex,
        name: Cow<'ifc, str>,
    ) -> Result<()> {
        self.add_member(container, index, name);

        let func_ty = self.ifc.type_function().entry(function_type.index())?;

        // Add dependency on the return type.
        if func_ty.target.0 != 0 && !self.ifc.is_void_type(func_ty.target)? {
            self.include_type(func_ty.target);
        }

        // Add depenency on the parameter types.
        if func_ty.source.0 != 0 {
            // More than one paramter is stored as a tuple.
            if func_ty.source.tag() == TypeSort::TUPLE {
                let args_tuple = self.ifc.type_tuple().entry(func_ty.source.index())?;
                for i in args_tuple.start..args_tuple.start + args_tuple.cardinality {
                    let arg_ty = *self.ifc.heap_type().entry(i)?;
                    self.include_type(arg_ty);
                }
            } else {
                self.include_type(func_ty.source);
            }
        }

        Ok(())
    }

    /// Adds a variable/field declaration to the set to emit, and include its dependencies.
    fn add_variable_declaration(
        &mut self,
        container: DeclIndex,
        index: DeclIndex,
        variable_type: TypeIndex,
        name: Cow<'ifc, str>,
    ) {
        self.add_member(container, index, name);
        self.include_type(variable_type);
    }

    /// Walks a scope and checks if its members need to be included.
    fn walk_scope(&mut self, scope: ScopeIndex, fully_qualified_name: &str) -> Result<()> {
        let mut anon_name_counter: u32 = 0;

        for member_decl_index in self.ifc.iter_scope(scope)? {
            log_error! { {
                match member_decl_index.tag() {
                    DeclSort::ALIAS => {
                        let decl_alias = self.ifc.decl_alias().entry(member_decl_index.index())?;
                        let mut alias_name = self.ifc.get_string(decl_alias.name)?.to_string();
                        fixup_anon_names(&mut alias_name, &mut anon_name_counter);
                        let fully_qualified_name = format!("{}::{}", fully_qualified_name, alias_name);

                        if self.symbol_map.is_symbol_in(&alias_name) {
                            debug!("alias {} is defined in external crate", alias_name);
                            self.included_types.insert(decl_alias.type_);
                        } else if self.included_types.contains(&decl_alias.type_) || self.type_filter.is_allowed(&fully_qualified_name)
                        {
                            self.add_alias_declaration_to_emit(member_decl_index, decl_alias, alias_name.into());
                        } else {
                            self.skipped_types.insert(decl_alias.type_, SkippedType { index: member_decl_index, name: alias_name.into(), fully_qualified_name }).expect("Duplicate skipsped type");
                        }
                    }

                    DeclSort::FUNCTION => {
                        let func_decl = self.ifc.decl_function().entry(member_decl_index.index())?;
                        match func_decl.name.tag() {
                            NameSort::IDENTIFIER => {
                                let func_name = self.ifc.get_string(func_decl.name.index())?;

                                // TODO: hack, deal with overloaded function names in nt.h
                                if func_name != "_RTL_CONSTANT_STRING_type_check" {
                                    // TODO: Only for non-member functions?
                                    if self.symbol_map.is_symbol_in(&func_name) {
                                        debug!("alias {} is defined in external crate", func_name);
                                    } else if self.function_filter.is_allowed_qualified_name(func_name, fully_qualified_name) {
                                        self.add_function_declaration_to_emit(func_decl.home_scope, member_decl_index, func_decl.type_, func_name.into())?;
                                    }
                                }
                            }
                            _ => {
                                // For now, we ignore all other kinds of functions.
                                debug!("ignoring function named {:?}", func_decl.name);
                            }
                        }
                    }

                    DeclSort::METHOD => {
                        let method_decl = self.ifc.decl_method().entry(member_decl_index.index())?;
                        match method_decl.name.tag() {
                            #[cfg(todo)]
                            NameSort::IDENTIFIER => {
                                let method_name = self.ifc.get_string(method_decl.name.index())?;

                                if self.function_filter.is_allowed_qualified_name(method_name, fully_qualified_name) {
                                    self.add_function_declaration_to_emit(method_decl.home_scope, member_decl_index, method_decl.type_, method_name.into())?;
                                }
                            }
                            _ => {
                                // For now, we ignore all other kinds of methods.
                                debug!("ignoring method named {:?}", method_decl.name);
                            }
                        }
                    }

                    DeclSort::SCOPE => {
                        let nested_scope = self.ifc.decl_scope().entry(member_decl_index.index())?;
                        let scope_name = if let Some(id) = self.renamed_decls.get(&member_decl_index) {
                            id.to_string()
                        } else {
                            self.ifc.get_string(nested_scope.name.index())?.to_string()
                        };
                        let fully_qualified_name = format!("{}::{}", fully_qualified_name, scope_name);

                        if self.ifc.is_type_namespace(nested_scope.ty)? {
                            // Walk the namespace.
                            self.walk_scope(nested_scope.initializer, &fully_qualified_name)?;
                        } else if self.symbol_map.is_symbol_in(&scope_name) {
                            debug!("type {} is defined in external crate", scope_name);
                            self.included_types.insert(nested_scope.ty);
                        } else if self.included_types.contains(&nested_scope.ty) || self.type_filter.is_allowed(&fully_qualified_name) {
                            self.add_type_declaration_to_emit(member_decl_index, nested_scope, &fully_qualified_name, scope_name.into())?;
                        } else {
                            self.skipped_types.insert(nested_scope.ty, SkippedType { index: member_decl_index, name: scope_name.into(), fully_qualified_name }).expect("Duplicate skipped type");
                        }
                    }

                    DeclSort::ENUMERATION => {
                        let en = self.ifc.decl_enum().entry(member_decl_index.index())?;
                        let en_name = self.ifc.get_string(en.name)?;
                        let fully_qualified_name = format!("{}::{}", fully_qualified_name, en_name);

                        if self.symbol_map.is_symbol_in(&en_name) {
                            debug!("alias {} is defined in external crate", en_name);
                            self.included_types.insert(en.ty);
                        } else if self.included_types.contains(&en.ty) || self.type_filter.is_allowed(&fully_qualified_name) {
                            self.add_enum_declaration_to_emit(member_decl_index, en, en_name.into());
                        } else {
                            self.skipped_types.insert(en.ty, SkippedType { index: member_decl_index, name: en_name.into(), fully_qualified_name }).expect("Duplicate skipped type");
                        }
                    }

                    DeclSort::VARIABLE => {
                        let var_decl = self.ifc.decl_var().entry(member_decl_index.index())?;
                        match var_decl.name.tag() {
                            NameSort::IDENTIFIER => {
                                let var_name = self.ifc.get_string(var_decl.name.index())?;

                                if self.symbol_map.is_symbol_in(&var_name) {
                                    debug!("alias {} is defined in external crate", var_name);
                                } else if self.variable_filter.is_allowed_qualified_name(var_name, fully_qualified_name) {
                                    self.add_variable_declaration(var_decl.home_scope, member_decl_index, var_decl.ty, var_name.into());
                                }
                            }
                            _ => {
                                // For now, we ignore all other kinds of variables.
                                debug!("ignoring variable named {:?}", var_decl.name);
                            }
                        }
                    }

                    DeclSort::FIELD => {
                        let field_decl = self.ifc.decl_field().entry(member_decl_index.index())?;
                        self.add_variable_declaration(field_decl.home_scope, member_decl_index, field_decl.ty, self.ifc.get_string(field_decl.name)?.into());
                    }

                    DeclSort::INTRINSIC
                    | DeclSort::TEMPLATE
                    | DeclSort::CONCEPT
                    | DeclSort::EXPLICIT_INSTANTIATION
                    | DeclSort::BITFIELD
                    | DeclSort::USING_DECLARATION
                    | DeclSort::PARTIAL_SPECIALIZATION
                    | DeclSort::EXPLICIT_SPECIALIZATION => {}

                    _ => {
                        nyi!();
                        info!("unknown decl: {:?}", member_decl_index);
                    }
                }
                Ok(())
            } -> (), format!("Walking {:?} with id {} in {}", member_decl_index.tag(), member_decl_index.index(), fully_qualified_name) };
            //self.had_errors = true;
        }

        Ok(())
    }
}
