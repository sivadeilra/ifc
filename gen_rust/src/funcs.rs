use super::*;

impl<'a> Gen<'a> {
    pub fn gen_function(
        &self,
        member_decl_index: DeclIndex,
        names_map: &mut HashMap<Ident, u32>,
        func_name: &str,
    ) -> Result<TokenStream> {
        let func_decl = self.ifc.decl_function().entry(member_decl_index.index())?;
        Ok(match func_decl.name.tag() {
            NameSort::IDENTIFIER => {
                if func_decl.type_.tag() != TypeSort::FUNCTION {
                    bail!("Function has wrong type: {:?}", func_decl.type_);
                }

                let original_func_ident = syn::Ident::new(func_name, Span::call_site());
                let mut func_ident = original_func_ident.clone();

                if names_map.contains_key(&original_func_ident) {
                    loop {
                        let overload_counter = names_map.get_mut(&original_func_ident).unwrap();
                        *overload_counter += 1;

                        func_ident = Ident::new(
                            &format!("{}_{}", func_name, *overload_counter),
                            Span::call_site(),
                        );

                        if names_map.contains_key(&func_ident) {
                            // loop again
                        } else {
                            // we found a somewhat-unique one
                            break;
                        }
                    }
                } else {
                    // This is the first time we've seen this symbol. Hopefully, it's the
                    // last time.
                    names_map.insert(func_ident.clone(), 0);
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

                let convention = match func_ty.convention {
                    CallingConvention::Std => quote!("stdcall"),
                    CallingConvention::Cdecl => quote!("C"),
                    CallingConvention::Fast => quote!("fastcall"),
                    _ => bail!(
                        "Function calling convention {:?} is not supported",
                        func_ty.convention
                    ),
                };

                quote! {
                    extern #convention {
                        pub fn #func_ident(
                            #args
                        ) #return_type_tokens;
                    }
                }
            }
            _ => {
                // For now, we ignore all other kinds of functions.
                debug!("ignoring function named {:?}", func_decl.name);
                TokenStream::new()
            }
        })
    }
}
