use super::*;

impl<'a> Gen<'a> {
    pub fn get_type_tokens(&self, mut type_index: TypeIndex) -> Result<TokenStream> {
        // Remove qualifiers.
        let mut const_qual = false;
        while type_index.tag() == TypeSort::QUALIFIED {
            let qt = self.ifc.type_qualified().entry(type_index.index())?;
            if qt.qualifiers.contains(Qualifiers::CONST) {
                const_qual = true;
            }
            type_index = qt.unqualified_type;
        }

        let mut anon_name_counter: u32 = 0;

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

                    TypeBasis::VOID => quote!(core::ffi::c_void),

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
                let desig_decl = *self.ifc.type_designated().entry(type_index.index())?;

                if let Some(id) = self.renamed_decls.get(&desig_decl) {
                    debug!("found renamed decl: {:?} -> {}", desig_decl, id);
                    return Ok(id.to_token_stream());
                }

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


                let mut desig_name = desig_name.to_string();
                fixup_anon_names(&mut desig_name, &mut anon_name_counter);

                if let Some(extern_crate) = self.symbol_map.resolve(&desig_name) {
                    // This designated type reference resolves to a name in a dependent crate.
                    trace!("resolved type to external crate: {}", extern_crate);
                    let extern_ident = syn::Ident::new(extern_crate, Span::call_site());
                    let desig_ident = syn::Ident::new(&desig_name, Span::call_site());
                    quote! { #extern_ident :: #desig_ident}
                } else {
                    // This designated type references something in this crate.
                    Ident::new(&desig_name, Span::call_site()).to_token_stream()
                }
            }

            TypeSort::UNALIGNED => {
                // TODO: property handle unaligned
                let unaligned = self.ifc.type_unaligned().entry(type_index.index())?;
                self.get_type_tokens(*unaligned)?
            }

            TypeSort::LVALUE_REFERENCE => {
                let lvalue_ref = *self.ifc.type_lvalue_reference().entry(type_index.index())?;
                let tokens = self.get_type_tokens(lvalue_ref)?;
                quote!(*mut #tokens)
            }

            TypeSort::RVALUE_REFERENCE => {
                let rvalue_ref = *self.ifc.type_rvalue_reference().entry(type_index.index())?;
                let tokens = self.get_type_tokens(rvalue_ref)?;
                quote!(*mut #tokens)
            }

            TypeSort::FUNCTION => {
                quote!(*const core::ffi::c_void)
            }

            _ => todo!("unrecognized type sort: {:?}", type_index),
        })
    }
}
