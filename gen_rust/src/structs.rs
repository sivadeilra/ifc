use super::*;

impl<'a> Gen<'a> {
    pub fn gen_struct(
        &self,
        member_decl_index: DeclIndex,
        parent_scope_name: &str,
        filter: options::Filter,
    ) -> Result<Option<(TokenStream, String)>> {
        let nested_scope = self.ifc.decl_scope().entry(member_decl_index.index())?;

        // What kind of scope is it?
        if self.ifc.is_type_namespace(nested_scope.ty)? {
            // We do not yet process namespaces.
            return Ok(None);
        }
        // It's a nested struct/class.

        let nested_scope_name: String;
        let nested_scope_ident: Ident = if let Some(id) = self.renamed_decls.get(&member_decl_index)
        {
            nested_scope_name = id.to_string();
            id.clone()
        } else {
            nested_scope_name = self.ifc.get_string(nested_scope.name.index())?.to_string();
            Ident::new(&nested_scope_name, Span::call_site())
        };

        // Check to see if its filtered.
        if !filter.is_allowed_qualified_name(&nested_scope_name, parent_scope_name) {
            return Ok(None);
        }

        // If the initializer is NULL (not empty, but NULL), then this is a forward declaration
        // with no definition.
        if nested_scope.initializer == 0 {
            // This struct has a forward declaration but no definition,
            // e.g. "struct FOO;".  Not sure what to do about that, yet.
            debug!("struct {} - forward decl", nested_scope_ident);

            let use_extern_types = false;
            if use_extern_types {
                return Ok(Some((
                    quote! {
                        extern "C" {
                            pub type #nested_scope_ident;
                        }
                    },
                    nested_scope_name,
                )));
            } else {
                return Ok(Some((
                    quote! {
                        #[repr(transparent)]
                        pub struct #nested_scope_ident(pub u8);
                    },
                    nested_scope_name,
                )));
            }
        }

        // If the type is defined in a different crate, then do not emit a definition.
        if self.symbol_map.is_symbol_in(&nested_scope_name) {
            debug!("struct {} - defined in external crate", nested_scope_name);
            return Ok(None);
        }

        // Emit the definition for this struct.
        debug!("struct {} - emitting", nested_scope_name);
        let mut struct_contents = TokenStream::new();

        if nested_scope.base.0 != 0 {
            for (base_index, base_ty) in self.ifc.iter_type_tuple(nested_scope.base)?.enumerate() {
                if base_ty.tag() != TypeSort::BASE {
                    bail!("Base type is not a TypeSort::BASE: {:?}", base_ty);
                }

                let base = self.ifc.type_base().entry(base_ty.index())?;
                let base_field_ident = if base_index != 0 {
                    Ident::new(&format!("_base{}", base_index), Span::call_site())
                } else {
                    Ident::new("_base", Span::call_site())
                };

                let base_ty_tokens = self.get_type_tokens(base.ty)?;
                struct_contents.extend(quote! {
                    pub #base_field_ident: #base_ty_tokens,
                });
            }
        }

        // TODO: handle packing
        // TODO: handle alignment

        let mut anon_name_counter: u32 = 0;

        for member_decl in self.ifc.iter_scope(nested_scope.initializer)? {
            match member_decl.tag() {
                DeclSort::FIELD => {
                    let field_decl = self.ifc.decl_field().entry(member_decl.index())?;
                    let mut field_name = self.ifc.get_string(field_decl.name)?.to_string();

                    if field_name == "EntryPointActivationContext" {
                        debug!(
                            "found EntryPointActivationContext:\nField: {:#?}\nTy: {:#?}",
                            field_decl, field_decl.ty
                        );
                        if field_decl.ty.tag() == TypeSort::POINTER {
                            let ptr = self.ifc.type_pointer().entry(field_decl.ty.index())?;
                            debug!("ptr: {:#?}", ptr);
                            if ptr.tag() == TypeSort::DESIGNATED {
                                let desig_decl: DeclIndex =
                                    *self.ifc.type_designated().entry(ptr.index())?;
                                debug!("desig_decl = {:?}", desig_decl);

                                if let DeclSort::SCOPE = desig_decl.tag() {
                                    let scope =
                                        self.ifc.decl_scope().entry(desig_decl.index())?;
                                    let scope_name = self.ifc.get_name_string(scope.name)?;
                                    debug!("... {} {:?} ({:?})", scope_name, desig_decl, scope)
                                }
                            }
                        }
                    }

                    fixup_anon_names(&mut field_name, &mut anon_name_counter);
                    let field_ident = Ident::new(&field_name, Span::call_site());
                    let field_type_tokens = self.get_type_tokens(field_decl.ty)?;
                    struct_contents.extend(quote! { pub #field_ident: #field_type_tokens, });
                }

                DeclSort::BITFIELD => {
                    // TODO: implement bitfields
                    if true {
                        let bitfield = self.ifc.decl_bitfield().entry(member_decl.index())?;
                        let bitfield_name = self.ifc.get_string(bitfield.name)?;
                        if bitfield_name.starts_with('<') {
                            // e.g. "<alignment member>"
                        } else {
                            let bitfield_ident = Ident::new(bitfield_name, Span::call_site());
                            let bitfield_type_tokens = self.get_type_tokens(bitfield.ty)?;
                            let bitfield_width =
                                self.ifc.get_literal_expr_u32(bitfield.width)? as usize;
                            struct_contents.extend(quote! {
                                pub #bitfield_ident: __Bitfield<#bitfield_width, #bitfield_type_tokens>,
                            });
                        }
                    }
                }

                _ => {
                    // Ignore everything else, for now.
                }
            }
        }

        // This is useful, but very verbose.
        // let doc = format!("{:#?}", nested_scope);

        let doc = format!("Scope: {:?}", member_decl_index);

        debug!("emitting struct {}", nested_scope_ident);
        Ok(Some((
            quote! {
                #[doc = #doc]
                #[repr(C)]
                pub struct #nested_scope_ident {
                    #struct_contents
                }
            },
            nested_scope_name,
        )))
    }
}
