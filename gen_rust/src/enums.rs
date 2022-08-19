use super::*;

impl<'a> Gen<'a> {
    pub fn gen_enum(&self, enum_decl: &DeclEnum) -> Result<TokenStream> {
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
