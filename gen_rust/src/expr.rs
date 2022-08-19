use super::*;

impl<'a> Gen<'a> {
    // This converts literal expressions into token streams.
    pub fn gen_expr_tokens(&self, ty: ifc::TypeIndex, expr: ifc::ExprIndex) -> Result<TokenStream> {
        let ty = self.ifc.remove_qualifiers(ty)?;

        Ok(match expr.tag() {
            ExprSort::LITERAL => {
                let literal = self.ifc.expr_literal().entry(expr.index())?;
                debug!("literal = {:?}", literal);

                // It appears the "type" field in ExprLiteral is always set to 0, which is
                // VENDOR_EXTENSION.  So we don't actually know the type of the literal.

                if ty.tag() != TypeSort::FUNDAMENTAL {
                    bail!(
                        "gen_expr_tokens: This only works with TypeSort::FUNDAMENTAL, not {:?}",
                        ty
                    );
                }
                let fun_ty = self.ifc.type_fundamental().entry(ty.index())?;
                debug!("gen_expr_tokens: fun_ty {:?}", fun_ty);

                match literal.value.tag() {
                    LiteralSort::IMMEDIATE => {
                        let value: u32 = literal.value.index();
                        trace!("LiteralSort::IMMEDIATE: value = 0x{:x} {}", value, value);
                        if fun_ty.basis == TypeBasis::BOOL {
                            if value != 0 {
                                quote!(true)
                            } else {
                                quote!(false)
                            }
                        } else {
                            let lit = syn::LitInt::new(&value.to_string(), Span::call_site());
                            quote!(#lit)
                        }
                    }
                    LiteralSort::INTEGER => {
                        let value: u64 = *self.ifc.const_i64().entry(literal.value.index())?;
                        trace!("LiteralSort::INTEGER: value = 0x{:x} {}", value, value);
                        if fun_ty.basis == TypeBasis::BOOL {
                            if value != 0 {
                                quote!(true)
                            } else {
                                quote!(false)
                            }
                        } else {
                            if matches!(fun_ty.sign, TypeSign::SIGNED | TypeSign::PLAIN) {
                                let value_i64: i64 = value as i64;
                                if value_i64 < 0 {
                                    if let Some(value_pos) = value_i64.checked_abs() {
                                        let lit = syn::LitInt::new(
                                            &value_pos.to_string(),
                                            Span::call_site(),
                                        );
                                        quote!(-#lit)
                                    } else {
                                        bail!(
                                        "Negative value is -MAX_INT, not sure how to handle that."
                                    );
                                    }
                                } else {
                                    let lit =
                                        syn::LitInt::new(&value.to_string(), Span::call_site());
                                    quote!(#lit)
                                }
                            } else {
                                let lit = syn::LitInt::new(&value.to_string(), Span::call_site());
                                quote!(#lit)
                            }
                        }
                    }
                    LiteralSort::FLOATING_POINT => {
                        todo!("floating point literals")
                    }
                    _ => todo!("unrecognized literal value: {:?}", literal.value),
                }
            }

            ExprSort::DYAD => {
                let dyad = self.ifc.expr_dyad().entry(expr.index())?;
                bail!("ExprSort::DYAD: {:?}", dyad);
            }

            _ => todo!("unsupported expr: {:?}", expr),
        })
    }

    pub fn get_literal_expr_as_u64(&self, expr: ifc::ExprIndex) -> Result<u64> {
        Ok(match expr.tag() {
            ExprSort::LITERAL => {
                let literal = self.ifc.expr_literal().entry(expr.index())?;
                match literal.value.tag() {
                    LiteralSort::IMMEDIATE => literal.value.index() as u64,
                    LiteralSort::INTEGER => *self.ifc.const_i64().entry(literal.value.index())?,
                    LiteralSort::FLOATING_POINT => {
                        todo!("floating point literals")
                    }
                    _ => todo!("unrecognized literal value: {:?}", literal.value),
                }
            }

            _ => todo!("unsupported expr: {:?}", expr),
        })
    }
}
