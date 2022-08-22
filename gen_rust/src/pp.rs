use super::*;
use log::warn;

mod eval;

impl<'a> Gen<'a> {
    /// Generates constants from preprocessor macro definitions, when possible.
    pub fn gen_macros(&self) -> Result<TokenStream> {
        let mut output = TokenStream::new();

        for object in self.ifc.macro_object_like().entries.iter() {
            self.gen_one_object_like_macro(object, &mut output)?;
        }

        Ok(output)
    }

    /// Examines an object-like preprocessor macro and attempts to convert it to a Rust constant.
    ///
    fn gen_one_object_like_macro(
        &self,
        object: &ifc::MacroObjectLike,
        output: &mut TokenStream,
    ) -> Result<()> {
        let name = self.ifc.get_string(object.name)?;

        trace!("processing: #define {} ...", name);

        // It's not possible to convert all #define macros to Rust constants. We look for certain
        // patterns. The simplest macros to convert consist of a single literal constant, or a
        // a single literal constant inside parens.  More complex expressions that use operators
        // and such are more difficult to convert.  Complex expressions that refer to other
        // macros or constants are the hardest.

        let body = self.remove_parens(object.body)?;
        loop {
            match body.tag() {
                FormSort::NUMBER => {
                    let t = self.convert_form_number_to_tokens(name, body)?;
                    output.extend(t);
                    return Ok(());
                }
                FormSort::TUPLE => {
                    if false {
                        let tuple = self.ifc.pp_tuple().entry(body.index())?;
                        if tuple.cardinality != 1 {
                            warn!("tuple cardinality is {}, but we need 1", tuple.cardinality);
                            break;
                        }

                        let element0 = *self.ifc.heap_form().entry(tuple.start)?;
                        let t = self.convert_form_number_to_tokens(name, element0)?;
                        output.extend(t);
                    }
                    return Ok(());
                }
                _ => {}
            }
            break;
        }

        warn!(
            "#define {} - definition did not match any supported pattern: {:?}",
            name, body
        );
        Ok(())
    }

    fn convert_form_number_to_tokens(&self, name: &str, form: FormIndex) -> Result<TokenStream> {
        assert!(form.tag() == FormSort::NUMBER);

        let ident = Ident::new(name, Span::call_site());

        let is_negative = false;

        let num = self.ifc.pp_num().entry(form.index())?;
        let num_str = self.ifc.get_string(num.spelling)?;

        // C++ numeric literals can be complex.
        // * Can have 0x prefix for hex
        // * Can have suffix: U, UL, L, ULL, LL
        // * Can have interior ' for separating parts

        use core::fmt::Write;

        let mut s = num_str;
        let mut is_long = false;
        let mut is_long_long = false;
        let mut is_unsigned = false;
        let mut chars: String = String::new();

        let mut radix = 10;
        if s.starts_with("0x") || s.starts_with("0X") {
            radix = 0x10;
            s = &s[2..];
        }

        for c in s.chars() {
            if c == '\'' {
                continue;
            }

            if c.is_ascii_digit() {
                chars.push(c);
                continue;
            }

            if c.is_ascii_hexdigit() {
                if radix == 0x10 {
                    chars.push(c);
                } else {
                    bail!("Found hex chars but did not have hex prefix: {:?}", num);
                }
            }

            if c == 'u' || c == 'U' {
                is_unsigned = true;
            }

            if c == 'l' || c == 'L' {
                if is_long_long {
                    // Really??
                    bail!("Number literal has too many L: {:?}", num);
                }
                if is_long {
                    is_long = false;
                    is_long_long = true;
                } else {
                    is_long = true;
                }
            }
        }

        // Parse the number.
        let value_u128: u128 = if let Ok(value) = u128::from_str_radix(&chars, radix) {
            value
        } else {
            bail!("Failed to parse number: {:?} chars {:?}", num_str, chars);
        };

        // We might reuse the chars buffer. Clear it.
        chars.clear();

        if is_long_long {
            if is_unsigned {
                let mut value_u64 = value_u128 as u64;
                if is_negative {
                    // Ok, *whatever*.
                    value_u64 = (value_u64 as i64).wrapping_neg() as u64;
                }

                if value_u64 == u64::MAX {
                    return Ok(quote!(pub const #ident: u64 = u64::MAX;));
                }

                write!(chars, "{}", value_u64).unwrap();
                let lit = syn::LitInt::new(&chars, Span::call_site()).to_token_stream();
                return Ok(quote! {
                    pub const #ident: u64 = #lit;
                });
            } else {
                // signed
                let mut value_i64 = value_u128 as i64;
                if is_negative {
                    value_i64 = value_i64.wrapping_neg();
                }

                // Special case for "most negative" value. This value will not change its sign
                // if we negate it, because it oveflows. Such is the price of 2's complement.
                if value_i64 == i64::MIN {
                    return Ok(quote! {
                        pub const #ident: i64 = i64::MIN;
                    });
                }

                if value_i64 == i64::MAX {
                    return Ok(quote! {
                        pub const #ident: i64 = i64::MAX;
                    });
                }

                if is_negative {
                    value_i64 = -value_i64;
                    return Ok(quote! {
                        pub const #ident: i64 = - #value_i64;
                    });
                } else {
                    return Ok(quote! {
                        pub const #ident: i64 = #value_i64;
                    });
                }
            }
        } else {
            // Emit as i32/u32
            if is_unsigned {
                let mut value_u32 = value_u128 as u32;
                if is_negative {
                    // Ok, *whatever*.
                    value_u32 = (value_u32 as i32).wrapping_neg() as u32;
                }
                if value_u32 == u32::MAX {
                    return Ok(quote!(pub const #ident: u32 = u32::MAX;));
                }
                return Ok(quote! {
                    pub const #ident: u64 = #value_u32;
                });
            } else {
                // signed
                let mut value_i32 = value_u128 as i32;
                if is_negative {
                    value_i32 = value_i32.wrapping_neg();
                }

                // Special case for "most negative" value. This value will not change its sign
                // if we negate it, because it oveflows. Such is the price of 2's complement.
                if value_i32 == i32::MIN {
                    return Ok(quote! {
                        pub const #ident: i32 = i32::MIN;
                    });
                }

                if value_i32 == i32::MAX {
                    return Ok(quote! {
                        pub const #ident: i32 = i32::MAX;
                    });
                }

                if is_negative {
                    value_i32 = -value_i32;
                    return Ok(quote! {
                        pub const #ident: i32 = - #value_i32;
                    });
                } else {
                    return Ok(quote! {
                        pub const #ident: i32 = #value_i32;
                    });
                }
            }
        }
    }

    /// Removes any outer-most parens from a form.
    fn remove_parens(&self, mut form: FormIndex) -> Result<FormIndex> {
        while form.tag() == FormSort::PARENTHESIZED {
            let inner = self.ifc.pp_paren().entry(form.index())?;
            form = inner.operand;
        }
        Ok(form)
    }
}
