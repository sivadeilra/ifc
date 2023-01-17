use super::*;
use crate::{fixup_anon_names, options::Filter, SymbolMap};
use anyhow::Result;
use log::info;
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
};

struct SkippedType<'ifc> {
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
    included_declarations: HashSet<DeclIndex>,

    /// Scopes in the current crate and their contents that are to be emitted.
    scope_to_contents: HashMap<DeclIndex, HashMap<DeclIndex, Cow<'ifc, str>>>,

    /// Discovered types that were skipped while their containing scope was
    /// being walked.
    skipped_types: HashMap<DeclIndex, SkippedType<'ifc>>,
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
            included_declarations: HashSet::new(),
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
        debug!("Walking global scope with filters: Type={:?}, Func={:?}, Vars={:?}", type_filter, function_filter, variable_filter);

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
    fn include_declaration(&mut self, index: DeclIndex) {
        if self.included_declarations.insert(index) {
            // If this type was previously skipped, then re-walk its declaration to discover new
            // types.
            if let Some(SkippedType {
                name,
                fully_qualified_name,
            }) = self.skipped_types.remove(&index)
            {
                log_error! { {
                    debug!("{:?}> Including previously skipped {}", index, fully_qualified_name);
                    match index.tag() {
                        DeclSort::ALIAS => {
                            let decl_alias = self.ifc.decl_alias().entry(index.index())?;
                            self.add_alias_declaration_to_emit(index, decl_alias, name)?;
                        }

                        DeclSort::SCOPE => {
                            let decl_scope= self.ifc.decl_scope().entry(index.index())?;
                            self.add_type_declaration_to_emit(index, decl_scope, &fully_qualified_name, name)?;
                        }

                        DeclSort::ENUMERATION => {
                            let decl_enum= self.ifc.decl_enum().entry(index.index())?;
                            self.add_enum_declaration_to_emit(index, decl_enum, name)?;
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
            .insert(member, name)
            .map(|name| panic!("Duplicate member {}", name));
    }

    /// Walks a type expression and includes any declarations required to represent that type.
    fn include_type(&mut self, index: TypeIndex) -> Result<()> {
        debug!("{:?}", index.tag());
        match index.tag() {
            TypeSort::ARRAY => {
                // Include the element type.
                self.include_type(self.ifc.type_array().entry(index.index())?.element)?
            }

            TypeSort::BASE => {
                // Include the base type.
                self.include_type(self.ifc.type_base().entry(index.index())?.ty)?
            }

            TypeSort::DESIGNATED => {
                // Include the declaration for this type.
                self.include_declaration(*self.ifc.type_designated().entry(index.index())?)
            }

            TypeSort::FUNDAMENTAL => {
                // Fundamental types already exist, so nothing to do here.
            }

            TypeSort::LVALUE_REFERENCE => {
                // Include the referenced type.
                self.include_type(*self.ifc.type_lvalue_reference().entry(index.index())?)?
            }

            TypeSort::POINTER => {
                // Include the pointed-at type.
                self.include_type(*self.ifc.type_pointer().entry(index.index())?)?
            }

            TypeSort::QUALIFIED => {
                // Include the unqualified type.
                self.include_type(self.ifc.type_qualified().entry(index.index())?.unqualified_type)?
            }

            TypeSort::RVALUE_REFERENCE => {
                // Include the referenced type.
                self.include_type(*self.ifc.type_rvalue_reference().entry(index.index())?)?
            }

            TypeSort::TUPLE => {
                // Include each of the component types.
                let tuple = self.ifc.type_tuple().entry(index.index())?;
                for i in tuple.start..tuple.start + tuple.cardinality {
                    let current_ty = *self.ifc.heap_type().entry(i)?;
                    self.include_type(current_ty)?;
                }
            }

            TypeSort::VENDOR_EXTENSION => {
                warn!("Dropping Vendor Extension type");
            }

            _ => panic!("Don't know how to handle {:?}", index),
        }

        Ok(())
    }

    /// Adds a type alias declaration to the set to emit, and include its dependencies.
    fn add_alias_declaration_to_emit(
        &mut self,
        index: DeclIndex,
        alias: &DeclAlias,
        name: Cow<'ifc, str>,
    ) -> Result<()> {
        self.add_member(alias.home_scope, index, name);
        self.include_declaration(index);
        self.include_type(alias.aliasee)
    }

    /// Adds an enum alias declaration to the set to emit, and include its dependencies.
    fn add_enum_declaration_to_emit(
        &mut self,
        index: DeclIndex,
        en: &DeclEnum,
        name: Cow<'ifc, str>,
    ) -> Result<()> {
        self.add_member(en.home_scope, index, name);
        self.include_declaration(index);
        self.include_type(en.base)
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
        self.include_declaration(index);
        self.include_type(scope.base)?;
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
            self.include_type(func_ty.target)?;
        }

        // Add depenency on the parameter types.
        if func_ty.source.0 != 0 {
            self.include_type(func_ty.source)?;
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
    ) -> Result<()> {
        self.add_member(container, index, name);
        self.include_type(variable_type)
    }

    /// Walks a scope and checks if its members need to be included.
    fn walk_scope(&mut self, container: ScopeIndex, container_fully_qualified_name: &str) -> Result<()> {
        let mut anon_name_counter: u32 = 0;

        fn panic_on_duplicate_skipped_type(ty: SkippedType) -> ! {
            panic!("Duplicate skipped type {}", ty.fully_qualified_name);
        }

        for member_decl_index in self.ifc.iter_scope(container)? {
            log_error! { {
                match member_decl_index.tag() {
                    DeclSort::ALIAS => {
                        let decl_alias = self.ifc.decl_alias().entry(member_decl_index.index())?;
                        let mut alias_name = self.ifc.get_string(decl_alias.name)?.to_string();
                        fixup_anon_names(&mut alias_name, &mut anon_name_counter);
                        let fully_qualified_name = format!("{}::{}", container_fully_qualified_name, alias_name);

                        if self.symbol_map.is_symbol_in(&alias_name) {
                            debug!("{:?}> {:?} {} is defined in external crate", member_decl_index, decl_alias.type_, fully_qualified_name);
                            self.included_declarations.insert(member_decl_index);
                        } else if self.included_declarations.contains(&member_decl_index) || self.type_filter.is_allowed(&fully_qualified_name)
                        {
                            debug!("{:?}> adding {:?} {} to emit list", member_decl_index, decl_alias.type_, fully_qualified_name);
                            self.add_alias_declaration_to_emit(member_decl_index, decl_alias, alias_name.into())?;
                        } else {
                            debug!("{:?}> skipping {:?} {}", member_decl_index, decl_alias.type_, fully_qualified_name);
                            self.skipped_types.insert(member_decl_index, SkippedType { name: alias_name.into(), fully_qualified_name }).map(panic_on_duplicate_skipped_type);
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
                                        debug!("{:?}> function {}::{} is defined in external crate", member_decl_index, container_fully_qualified_name, func_name);
                                    } else if self.function_filter.is_allowed_qualified_name(func_name, container_fully_qualified_name) {
                                        debug!("{:?}> adding function {}::{} to emit list", member_decl_index, container_fully_qualified_name, func_name);
                                        self.add_function_declaration_to_emit(func_decl.home_scope, member_decl_index, func_decl.type_, func_name.into())?;
                                    }
                                }
                            }
                            _ => {
                                // For now, we ignore all other kinds of functions.
                                debug!("{:?}> ignoring function named {:?}: incompatible name type", member_decl_index, func_decl.name.tag());
                            }
                        }
                    }

                    DeclSort::METHOD => {
                        let method_decl = self.ifc.decl_method().entry(member_decl_index.index())?;
                        match method_decl.name.tag() {
                            NameSort::IDENTIFIER => {
                                #[cfg(todo)]
                                {
                                    let method_name = self.ifc.get_string(method_decl.name.index())?;

                                    if self.function_filter.is_allowed_qualified_name(method_name, fully_qualified_name) {
                                        debug!("{:?}> adding method {}::{} to emit list", member_decl_index, container_fully_qualified_name, func_name);
                                        self.add_function_declaration_to_emit(method_decl.home_scope, member_decl_index, method_decl.type_, method_name.into())?;
                                    }
                                }
                            }
                            _ => {
                                // For now, we ignore all other kinds of methods.
                                debug!("{:?}> ignoring method named {:?}: incompatible name type", member_decl_index, method_decl.name.tag());
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
                        let fully_qualified_name = format!("{}::{}", container_fully_qualified_name, scope_name);

                        if self.ifc.is_type_namespace(nested_scope.ty)? {
                            // Walk the namespace.
                            debug!("{:?}> walking namespace {}", member_decl_index, scope_name);
                            self.walk_scope(nested_scope.initializer, &fully_qualified_name)?;
                        } else if self.symbol_map.is_symbol_in(&scope_name) {
                            debug!("{:?}> {:?} {} is defined in external crate", member_decl_index, nested_scope.ty, fully_qualified_name);
                            self.included_declarations.insert(member_decl_index);
                        } else if self.included_declarations.contains(&member_decl_index) || self.type_filter.is_allowed(&fully_qualified_name) {
                            debug!("{:?}> adding {:?} {} to emit list", member_decl_index, nested_scope.ty, fully_qualified_name);
                            self.add_type_declaration_to_emit(member_decl_index, nested_scope, &fully_qualified_name, scope_name.into())?;
                        } else {
                            debug!("{:?}> skipping {:?} {}", member_decl_index, nested_scope.ty, fully_qualified_name);
                            self.skipped_types.insert(member_decl_index, SkippedType { name: scope_name.into(), fully_qualified_name }).map(panic_on_duplicate_skipped_type);
                        }
                    }

                    DeclSort::ENUMERATION => {
                        let en = self.ifc.decl_enum().entry(member_decl_index.index())?;
                        let en_name = self.ifc.get_string(en.name)?;
                        let fully_qualified_name = format!("{}::{}", container_fully_qualified_name, en_name);

                        if self.symbol_map.is_symbol_in(&en_name) {
                            debug!("{:?}> {:?} {} is defined in external crate", member_decl_index, en.ty, fully_qualified_name);
                            self.included_declarations.insert(member_decl_index);
                        } else if self.included_declarations.contains(&member_decl_index) || self.type_filter.is_allowed(&fully_qualified_name) {
                            debug!("{:?}> adding {:?} {} to emit list", member_decl_index, en.ty, fully_qualified_name);
                            self.add_enum_declaration_to_emit(member_decl_index, en, en_name.into())?;
                        } else {
                            debug!("{:?}> skipping {:?} {}", member_decl_index, en.ty, fully_qualified_name);
                            self.skipped_types.insert(member_decl_index, SkippedType { name: en_name.into(), fully_qualified_name }).map(panic_on_duplicate_skipped_type);
                        }
                    }

                    DeclSort::VARIABLE => {
                        let var_decl = self.ifc.decl_var().entry(member_decl_index.index())?;
                        match var_decl.name.tag() {
                            NameSort::IDENTIFIER => {
                                let var_name = self.ifc.get_string(var_decl.name.index())?;

                                if self.symbol_map.is_symbol_in(&var_name) {
                                    debug!("{:?}> variable {}::{} is defined in external crate", member_decl_index, container_fully_qualified_name, var_name);
                                } else if self.variable_filter.is_allowed_qualified_name(var_name, container_fully_qualified_name) {
                                    debug!("{:?}> adding variable {}::{} to emit list", member_decl_index, container_fully_qualified_name, var_name);
                                    self.add_variable_declaration(var_decl.home_scope, member_decl_index, var_decl.ty, var_name.into())?;
                                }
                            }
                            _ => {
                                // For now, we ignore all other kinds of variables.
                                debug!("{:?}> ignoring variable named {:?}: incompatible name type", member_decl_index, var_decl.name.tag());
                            }
                        }
                    }

                    DeclSort::FIELD => {
                        let field_decl = self.ifc.decl_field().entry(member_decl_index.index())?;
                        let field_name = self.ifc.get_string(field_decl.name)?;
                        debug!("{:?}> adding field {}::{} to emit list", member_decl_index, container_fully_qualified_name, field_name);
                        self.add_variable_declaration(field_decl.home_scope, member_decl_index, field_decl.ty, field_name.into())?;
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
            } -> (), format!("Walking {:?} in {}", member_decl_index, container_fully_qualified_name) };
            //self.had_errors = true;
        }

        Ok(())
    }
}
