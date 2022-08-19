use super::*;

impl<'a> Gen<'a> {
    pub fn gen_struct(&self, member_decl_index: DeclIndex) -> Result<TokenStream> {
        let nested_scope = self.ifc.decl_scope().entry(member_decl_index.index())?;

        // What kind of scope is it?
        if self.ifc.is_type_namespace(nested_scope.ty)? {
            // We do not yet process namespaces.
            return Ok(quote!());
        }
        // It's a nested struct/class.

        let nested_scope_name = self.ifc.get_string(nested_scope.name.index())?;

        // If the initializer is NULL (not empty, but NULL), then this is a forward declaration
        // with no definition. We don't do anything for those, yet.
        if nested_scope.initializer == 0 {
            // This struct has a forward declaration but no definition,
            // e.g. "struct FOO;".  Not sure what to do about that, yet.
            debug!("struct {} - ignoring forward decl", nested_scope_name);
            return Ok(quote!());
        }

        // If the type is defined in a different crate, then do not emit a definition.
        if self.symbol_map.is_symbol_in(nested_scope_name) {
            debug!("struct {} - defined in external crate", nested_scope_name);
            return Ok(quote!());
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

        for member_decl in self.ifc.iter_scope(nested_scope.initializer)? {
            match member_decl.tag() {
                DeclSort::FIELD => {
                    let field_decl = self.ifc.decl_field().entry(member_decl.index())?;
                    let field_name = self.ifc.get_string(field_decl.name)?;
                    let field_ident = Ident::new(field_name, Span::call_site());
                    let field_type_tokens = self.get_type_tokens(field_decl.ty)?;
                    struct_contents.extend(quote! { pub #field_ident: #field_type_tokens, });
                }

                DeclSort::BITFIELD => {
                    // TODO: implement bitfields
                    let bitfield = self.ifc.decl_bitfield().entry(member_decl_index.index())?;
                    let bitfield_name = self.ifc.get_string(bitfield.name)?;
                    let bitfield_ident = Ident::new(bitfield_name, Span::call_site());
                    let _bitfield_type_string = self.ifc.get_type_string(bitfield.ty)?;
                    let _bitfield_width = self.ifc.get_literal_expr_u32(bitfield.width)?;
                    struct_contents.extend(quote! {
                        pub #bitfield_ident: (),
                    });
                }

                _ => {
                    // Ignore everything else, for now.
                }
            }
        }

        let struct_ident = syn::Ident::new(nested_scope_name, Span::call_site());

        let doc = format!("{:#?}", nested_scope);

        Ok(quote! {
            #[doc = #doc]
            #[repr(C)]
            pub struct #struct_ident {
                #struct_contents
            }
        })
    }
}
