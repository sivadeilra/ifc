use super::*;
use crate::{fixup_anon_names, options::Filter, SymbolMap};
use anyhow::Result;
use log::info;
use once_cell::unsync::OnceCell;
use std::{borrow::Cow, collections::HashMap};

pub(crate) struct TypeAndCrateInformation<'ifc> {
    /// Scopes in the current crate and their contents that are to be emitted.
    pub scope_to_contents: HashMap<DeclIndex, HashMap<DeclIndex, Cow<'ifc, str>>>,

    /// The fully qualified name (including crate) to use for a given declaration.
    pub fully_qualified_names: HashMap<DeclIndex, TokenStream>,
}

struct FullyQualifiedName<'a> {
    name: Cow<'a, str>,
    container_as_string: String,
    cached_as_string: OnceCell<String>,
    container_as_tokens: TokenStream,
    cached_as_tokens: OnceCell<TokenStream>,
}

impl<'a> FullyQualifiedName<'a> {
    fn global_namespace() -> Self {
        Self {
            name: "".into(),
            container_as_string: "".to_string(),
            cached_as_string: OnceCell::with_value("".to_string()),
            container_as_tokens: TokenStream::new(),
            cached_as_tokens: OnceCell::with_value(TokenStream::new()),
        }
    }

    fn make_child<'child>(
        &self,
        child_name: impl Into<Cow<'child, str>>,
    ) -> FullyQualifiedName<'child> {
        FullyQualifiedName {
            name: child_name.into(),
            container_as_string: self.as_string().to_string(),
            cached_as_string: OnceCell::new(),
            container_as_tokens: self.as_tokens().clone(),
            cached_as_tokens: OnceCell::new(),
        }
    }

    fn name_as_ident(&self) -> Ident {
        Ident::new(&self.name, Span::call_site())
    }

    fn as_string(&self) -> &str {
        self.cached_as_string
            .get_or_init(|| format!("{}::{}", self.container_as_string, self.name))
    }

    fn as_tokens(&self) -> &TokenStream {
        self.cached_as_tokens.get_or_init(|| {
            let ident = self.name_as_ident();
            let container = &self.container_as_tokens;
            quote!(#container :: #ident)
        })
    }

    fn as_tokens_in_current_crate(&self) -> TokenStream {
        let ident = self.name_as_ident();
        let container = &self.container_as_tokens;
        quote!(crate #container :: #ident)
    }

    fn as_tokens_in_extern_crate(&self, extern_crate: &Ident) -> TokenStream {
        let ident = self.name_as_ident();
        let container = &self.container_as_tokens;
        quote!(#extern_crate #container :: #ident)
    }
}

/// Discovers types within a scope and maintains a list of included and skipped
/// types.
pub(crate) struct TypeDiscovery<'ifc, 'opt> {
    ifc: &'ifc Ifc,
    symbol_map: &'ifc SymbolMap,
    renamed_decls: HashMap<DeclIndex, Ident>,

    type_filter: Filter<'opt>,
    function_filter: Filter<'opt>,
    variable_filter: Filter<'opt>,

    /// Scopes in the current crate and their contents that are to be emitted.
    scope_to_contents: HashMap<DeclIndex, HashMap<DeclIndex, Cow<'ifc, str>>>,

    /// Discovered types that were skipped while their containing scope was
    /// being walked.
    skipped_types: HashMap<DeclIndex, FullyQualifiedName<'ifc>>,

    /// The fully qualified name (including crate) to use for a given declaration.
    fully_qualified_names: HashMap<DeclIndex, Option<TokenStream>>,
}

impl<'ifc, 'opt> TypeDiscovery<'ifc, 'opt> {
    /// Creates a new TypeDiscovery object.
    fn new(
        ifc: &'ifc Ifc,
        symbol_map: &'ifc SymbolMap,
        renamed_decls: HashMap<DeclIndex, Ident>,
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
            scope_to_contents: HashMap::new(),
            skipped_types: HashMap::new(),
            fully_qualified_names: HashMap::new(),
        }
    }

    /// Walks the global scope and discovers the set of scopes and their contents to be emitted.
    pub fn walk_global_scope(
        ifc: &'ifc Ifc,
        symbol_map: &'ifc SymbolMap,
        renamed_decls: HashMap<DeclIndex, Ident>,
        type_filter: Filter<'opt>,
        function_filter: Filter<'opt>,
        variable_filter: Filter<'opt>,
    ) -> TypeAndCrateInformation<'ifc> {
        debug!(
            "Walking global scope with filters: Type={:?}, Func={:?}, Vars={:?}",
            type_filter, function_filter, variable_filter
        );

        let mut type_discovery = TypeDiscovery::new(
            ifc,
            symbol_map,
            renamed_decls,
            type_filter,
            function_filter,
            variable_filter,
        );
        type_discovery
            .fully_qualified_names
            .insert(DeclIndex(0), Some(TokenStream::new()));
        log_error! { {
            if let Some(global_scope) = ifc.global_scope() {
                type_discovery
                    .scope_to_contents
                    .insert(
                        DeclIndex(0),
                        HashMap::new(),
                    );
                type_discovery.walk_scope(global_scope, &FullyQualifiedName::global_namespace())
            } else {
                Ok(())
            }
        } -> (), "Walking global scope" };

        TypeAndCrateInformation {
            scope_to_contents: type_discovery.scope_to_contents,
            fully_qualified_names: type_discovery
                .fully_qualified_names
                .into_iter()
                .map(|(k, v)| {
                    (
                        k,
                        v.unwrap_or_else(|| {
                            panic!("{:?}> Must have found names for declarations by now", k)
                        }),
                    )
                })
                .collect(),
        }
    }

    /// Include a type (from this crate or another) that are included (directly or transitively) by a filter.
    fn include_declaration(&mut self, index: DeclIndex) {
        if !self.fully_qualified_names.contains_key(&index) {
            // If this type was previously skipped, then re-walk its declaration to discover new
            // types.
            if let Some(type_name) = self.skipped_types.remove(&index) {
                self.fully_qualified_names
                    .insert(index, Some(type_name.as_tokens_in_current_crate()));
                let fully_qualified_name = type_name.as_string();
                log_error! { {
                    debug!("{:?}> Including previously skipped {}", index, fully_qualified_name);
                    match index.tag() {
                        DeclSort::ALIAS => {
                            let decl_alias = self.ifc.decl_alias().entry(index.index())?;
                            self.add_alias_declaration_to_emit(index, decl_alias, &type_name.name)?;
                        }

                        DeclSort::SCOPE => {
                            let decl_scope= self.ifc.decl_scope().entry(index.index())?;
                            self.add_type_declaration_to_emit(index, decl_scope, &type_name)?;
                        }

                        DeclSort::ENUMERATION => {
                            let decl_enum= self.ifc.decl_enum().entry(index.index())?;
                            self.add_enum_declaration_to_emit(index, decl_enum, &type_name.name)?;
                        }

                        _ => {
                            bail!("Unexpected skipped type");
                        }
                    }

                    Ok(())
                } -> (), format!("Discovering type {}", fully_qualified_name) };
            } else {
                self.fully_qualified_names.insert(index, None);
            }
        }
    }

    /// Add member to the contents of a scope.
    fn add_member(&mut self, container: DeclIndex, member: DeclIndex, name: &Cow<'ifc, str>) {
        self.scope_to_contents
            .entry(container)
            .or_insert_with(|| HashMap::new())
            .insert(member, name.clone())
            .map(|name| panic!("Duplicate member {}", name));
    }

    /// Walks a type expression and includes any declarations required to represent that type.
    fn include_type(&mut self, index: TypeIndex) -> Result<()> {
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
                self.include_type(
                    self.ifc
                        .type_qualified()
                        .entry(index.index())?
                        .unqualified_type,
                )?
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
        name: &Cow<'ifc, str>,
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
        name: &Cow<'ifc, str>,
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
        name: &FullyQualifiedName<'ifc>,
    ) -> Result<()> {
        self.add_member(scope.home_scope, index, &name.name);
        self.include_declaration(index);
        self.include_type(scope.base)?;
        if scope.initializer != 0 {
            self.walk_scope(scope.initializer, name)
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
        self.add_member(container, index, &name);

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
        name: &Cow<'ifc, str>,
    ) -> Result<()> {
        self.add_member(container, index, name);
        self.include_type(variable_type)
    }

    /// Walks a scope and checks if its members need to be included.
    fn walk_scope(
        &mut self,
        container: ScopeIndex,
        container_name: &FullyQualifiedName<'ifc>,
    ) -> Result<()> {
        let mut anon_name_counter: u32 = 0;

        fn panic_on_duplicate_skipped_type(ty: FullyQualifiedName) -> ! {
            panic!("Duplicate skipped type {}", ty.as_string());
        }

        for member_decl_index in self.ifc.iter_scope(container)? {
            log_error! { {
                match member_decl_index.tag() {
                    DeclSort::ALIAS => {
                        let decl_alias = self.ifc.decl_alias().entry(member_decl_index.index())?;
                        let mut alias_name = self.ifc.get_string(decl_alias.name)?.to_string();
                        fixup_anon_names(&mut alias_name, &mut anon_name_counter);
                        let alias_name = container_name.make_child(alias_name);
                        let fully_qualified_name = alias_name.as_string();

                        if let Some(extern_crate) = self.symbol_map.resolve(&fully_qualified_name) {
                            debug!("{:?}> {:?} {} is defined in external crate", member_decl_index, decl_alias.type_, fully_qualified_name);
                            self.fully_qualified_names.insert(member_decl_index, Some(alias_name.as_tokens_in_extern_crate(extern_crate)));
                        } else if self.fully_qualified_names.contains_key(&member_decl_index) || self.type_filter.is_allowed(&fully_qualified_name)
                        {
                            debug!("{:?}> adding {:?} {} to emit list", member_decl_index, decl_alias.type_, fully_qualified_name);
                            self.fully_qualified_names.insert(member_decl_index, Some(alias_name.as_tokens_in_current_crate()));
                            self.add_alias_declaration_to_emit(member_decl_index, decl_alias, &alias_name.name)?;
                        } else {
                            debug!("{:?}> skipping {:?} {}", member_decl_index, decl_alias.type_, fully_qualified_name);
                            self.skipped_types.insert(member_decl_index, alias_name).map(panic_on_duplicate_skipped_type);
                        }
                    }

                    DeclSort::FUNCTION => {
                        let func_decl = self.ifc.decl_function().entry(member_decl_index.index())?;
                        match func_decl.name.tag() {
                            NameSort::IDENTIFIER => {
                                let func_name = self.ifc.get_string(func_decl.name.index())?;

                                // TODO: hack, deal with overloaded function names in nt.h
                                if func_name != "_RTL_CONSTANT_STRING_type_check" {
                                    let func_name = container_name.make_child(func_name);
                                    let fully_qualified_name = func_name.as_string();
                                    if let Some(extern_crate) = self.symbol_map.resolve(&fully_qualified_name) {
                                        debug!("{:?}> function {} is defined in external crate", member_decl_index, fully_qualified_name);
                                        self.fully_qualified_names.insert(member_decl_index, Some(func_name.as_tokens_in_extern_crate(extern_crate)));
                                    } else if self.function_filter.is_allowed(&fully_qualified_name) {
                                        debug!("{:?}> adding function {} to emit list", member_decl_index, fully_qualified_name);
                                        self.fully_qualified_names.insert(member_decl_index, Some(func_name.as_tokens_in_current_crate()));
                                        self.add_function_declaration_to_emit(func_decl.home_scope, member_decl_index, func_decl.type_, func_name.name)?;
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

                                    if self.function_filter.is_allowed_qualified_name(method_name, container_fully_qualified_name) {
                                        debug!("{:?}> adding method {} to emit list", member_decl_index, fully_qualified_name);
                                        self.add_name_in_current_crate(member_decl_index, container_name, &method_name);
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
                        let scope_name = container_name.make_child(scope_name);
                        let fully_qualified_name = scope_name.as_string();

                        if self.ifc.is_type_namespace(nested_scope.ty)? {
                            // Walk the namespace.
                            debug!("{:?}> walking namespace {}", member_decl_index, fully_qualified_name);
                            self.walk_scope(nested_scope.initializer, &scope_name)?;
                            self.fully_qualified_names.insert(member_decl_index, Some(scope_name.as_tokens_in_current_crate()));
                            self.add_member(nested_scope.home_scope, member_decl_index, &scope_name.name);
                        } else if let Some(extern_crate) = self.symbol_map.resolve(&fully_qualified_name) {
                            debug!("{:?}> {:?} {} is defined in external crate", member_decl_index, nested_scope.ty, fully_qualified_name);
                            self.fully_qualified_names.insert(member_decl_index, Some(scope_name.as_tokens_in_extern_crate(extern_crate)));
                        } else if self.fully_qualified_names.contains_key(&member_decl_index) || self.type_filter.is_allowed(&fully_qualified_name) {
                            debug!("{:?}> adding {:?} {} to emit list", member_decl_index, nested_scope.ty, fully_qualified_name);
                            self.fully_qualified_names.insert(member_decl_index, Some(scope_name.as_tokens_in_current_crate()));
                            self.add_type_declaration_to_emit(member_decl_index, nested_scope, &scope_name)?;
                        } else {
                            debug!("{:?}> skipping {:?} {}", member_decl_index, nested_scope.ty, fully_qualified_name);
                            self.skipped_types.insert(member_decl_index, scope_name).map(panic_on_duplicate_skipped_type);
                        }
                    }

                    DeclSort::ENUMERATION => {
                        let en = self.ifc.decl_enum().entry(member_decl_index.index())?;
                        let en_name = container_name.make_child(self.ifc.get_string(en.name)?);
                        let fully_qualified_name = en_name.as_string();

                        if let Some(extern_crate) = self.symbol_map.resolve(&fully_qualified_name) {
                            debug!("{:?}> {:?} {} is defined in external crate", member_decl_index, en.ty, fully_qualified_name);
                            self.fully_qualified_names.insert(member_decl_index, Some(en_name.as_tokens_in_extern_crate(extern_crate)));
                        } else if self.fully_qualified_names.contains_key(&member_decl_index) || self.type_filter.is_allowed(&fully_qualified_name) {
                            debug!("{:?}> adding {:?} {} to emit list", member_decl_index, en.ty, fully_qualified_name);
                            self.fully_qualified_names.insert(member_decl_index, Some(en_name.as_tokens_in_current_crate()));
                            self.add_enum_declaration_to_emit(member_decl_index, en, &en_name.name)?;
                        } else {
                            debug!("{:?}> skipping {:?} {}", member_decl_index, en.ty, fully_qualified_name);
                            self.skipped_types.insert(member_decl_index, en_name).map(panic_on_duplicate_skipped_type);
                        }
                    }

                    DeclSort::VARIABLE => {
                        let var_decl = self.ifc.decl_var().entry(member_decl_index.index())?;
                        match var_decl.name.tag() {
                            NameSort::IDENTIFIER => {
                                let var_name = container_name.make_child(self.ifc.get_string(var_decl.name.index())?);
                                let fully_qualified_name = var_name.as_string();

                                if let Some(extern_crate) = self.symbol_map.resolve(&fully_qualified_name) {
                                    debug!("{:?}> variable {} is defined in external crate", member_decl_index, fully_qualified_name);
                                    self.fully_qualified_names.insert(member_decl_index, Some(var_name.as_tokens_in_extern_crate(extern_crate)));
                                } else if self.variable_filter.is_allowed(&fully_qualified_name) {
                                    debug!("{:?}> adding variable {} to emit list", member_decl_index, fully_qualified_name);
                                    self.fully_qualified_names.insert(member_decl_index, Some(var_name.as_tokens_in_current_crate()));
                                    self.add_variable_declaration(var_decl.home_scope, member_decl_index, var_decl.ty, &var_name.name)?;
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
                        let field_name = container_name.make_child(self.ifc.get_string(field_decl.name)?);
                        debug!("{:?}> adding field {} to emit list", member_decl_index, field_name.as_string());
                        self.add_variable_declaration(field_decl.home_scope, member_decl_index, field_decl.ty, &field_name.name)?;
                        self.fully_qualified_names.insert(member_decl_index, Some(field_name.as_tokens_in_current_crate()));
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
            } -> (), format!("Walking {:?} in {}", member_decl_index, container_name.as_string()) };
            //self.had_errors = true;
        }

        Ok(())
    }
}
