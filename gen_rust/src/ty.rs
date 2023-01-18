use super::*;

impl<'a> Gen<'a> {
    pub fn get_type_tokens(&self, type_index: TypeIndex) -> Result<TokenStream> {
        self.get_type_tokens_with_const(type_index)
            .map(|(tokens, _)| tokens)
    }

    fn get_type_tokens_with_const(&self, type_index: TypeIndex) -> Result<(TokenStream, bool)> {
        Ok(match type_index.tag() {
            TypeSort::QUALIFIED => {
                let qt = self.ifc.type_qualified().entry(type_index.index())?;
                let is_const = qt.qualifiers.contains(Qualifiers::CONST);
                (self.get_type_tokens(qt.unqualified_type)?, is_const)
            }

            TypeSort::FUNDAMENTAL => {
                let fun_ty = self.ifc.type_fundamental().entry(type_index.index())?;
                let is_signed = matches!(fun_ty.sign, TypeSign::SIGNED | TypeSign::PLAIN);

                let fun_tokens = match fun_ty.basis {
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
                            warn!("Floating-point types must have default precision");
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

                    // TODO: obviously bogus
                    TypeBasis::ELLIPSIS => quote!(core::ffi::c_void),

                    _ => todo!("TypeBasis: {:?}", fun_ty.basis),
                };
                (fun_tokens, false)
            }

            TypeSort::POINTER => {
                let pointed_ty = *self.ifc.type_pointer().entry(type_index.index())?;
                let (pointed_ty_tokens, is_const) = self.get_type_tokens_with_const(pointed_ty)?;
                (
                    if is_const {
                        quote! {*const #pointed_ty_tokens}
                    } else {
                        quote! {*mut #pointed_ty_tokens}
                    },
                    false,
                )
            }

            TypeSort::ARRAY => {
                let type_array = self.ifc.type_array().entry(type_index.index())?;
                let (element_tokens, is_const) =
                    self.get_type_tokens_with_const(type_array.element)?;
                if type_array.extent.tag() == ExprSort::VENDOR_EXTENSION {
                    // Unsized array - translate as a pointer.
                    (
                        if is_const {
                            quote! {*const #element_tokens}
                        } else {
                            quote! {*mut #element_tokens}
                        },
                        false,
                    )
                } else {
                    let extent_tokens = self.gen_expr_tokens(None, type_array.extent)?;
                    (
                        quote! {
                            [#element_tokens; #extent_tokens]
                        },
                        is_const,
                    )
                }
            }

            TypeSort::DESIGNATED => {
                let desig_decl = *self.ifc.type_designated().entry(type_index.index())?;
                let desig_name = self
                    .fully_qualified_names
                    .get(&desig_decl)
                    .expect("Any decl used must have had a name generated");
                (desig_name.clone(), false)
            }

            TypeSort::UNALIGNED => {
                // TODO: property handle unaligned
                let unaligned = self.ifc.type_unaligned().entry(type_index.index())?;
                self.get_type_tokens_with_const(*unaligned)?
            }

            TypeSort::LVALUE_REFERENCE => {
                let lvalue_ref = *self.ifc.type_lvalue_reference().entry(type_index.index())?;
                let (tokens, is_const) = self.get_type_tokens_with_const(lvalue_ref)?;
                (
                    if is_const {
                        quote! {*const #tokens}
                    } else {
                        quote! {*mut #tokens}
                    },
                    false,
                )
            }

            TypeSort::RVALUE_REFERENCE => {
                let rvalue_ref = *self.ifc.type_rvalue_reference().entry(type_index.index())?;
                let (tokens, is_const) = self.get_type_tokens_with_const(rvalue_ref)?;
                (
                    if is_const {
                        quote! {*const *const #tokens}
                    } else {
                        quote! {*mut *mut #tokens}
                    },
                    false,
                )
            }

            TypeSort::FUNCTION => (quote!(*const core::ffi::c_void), false),

            _ => todo!("unrecognized type sort: {:?}", type_index),
        })
    }
}
