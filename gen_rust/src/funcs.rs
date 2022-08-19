use super::*;

impl<'a> Gen<'a> {
    pub fn gen_function(
        &self,
        member_decl_index: DeclIndex,
    ) -> Result<Option<(CallingConvention, TokenStream)>> {
        let func_decl = self.ifc.decl_function().entry(member_decl_index.index())?;
        Ok(match func_decl.name.tag() {
            NameSort::IDENTIFIER => {
                let func_name = self.ifc.get_string(func_decl.name.index())?;

                if self.symbol_map.is_symbol_in(func_name) {
                    debug!("function {} - defined in external crate", func_name);
                    None
                } else {
                    if func_decl.type_.tag() != TypeSort::FUNCTION {
                        bail!("Function has wrong type: {:?}", func_decl.type_);
                    }
                    let func_ty = self.ifc.type_function().entry(func_decl.type_.index())?;

                    let mut return_type_tokens = TokenStream::new();
                    // "Target" just means the return type.
                    if func_ty.target.0 != 0 && !self.ifc.is_void_type(func_ty.target)? {
                        return_type_tokens.extend(quote!(->));
                        return_type_tokens.extend(self.get_type_tokens(func_ty.target));
                    }

                    let mut args = TokenStream::new();

                    if func_ty.source.0 != 0 {
                        let mut args_tys: Vec<TypeIndex> = Vec::new();
                        if func_ty.source.tag() == TypeSort::TUPLE {
                            let args_tuple = self.ifc.type_tuple().entry(func_ty.source.index())?;
                            for i in args_tuple.start..args_tuple.start + args_tuple.cardinality {
                                let arg_ty = *self.ifc.heap_type().entry(i)?;
                                args_tys.push(arg_ty);
                            }
                        } else {
                            args_tys = vec![func_ty.source];
                        }

                        for &arg_ty in args_tys.iter() {
                            let arg_ty_tokens = self.get_type_tokens(arg_ty)?;
                            args.extend(quote! {
                                _: #arg_ty_tokens,
                            });
                        }
                    }

                    let func_ident = syn::Ident::new(func_name, Span::call_site());
                    Some((
                        func_ty.convention,
                        quote! {
                            pub fn #func_ident(
                                #args
                            ) #return_type_tokens;
                        },
                    ))
                }
            }
            _ => {
                // For now, we ignore all other kinds of functions.
                debug!("ignoring function named {:?}", func_decl.name);
                None
            }
        })
    }
}
