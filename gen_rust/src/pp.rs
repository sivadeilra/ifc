use super::*;
use log::warn;
use proc_macro2::Punct;

mod eval;

#[derive(Copy, Clone)]
enum MacroObjectOrFunction<'a> {
    Object(&'a MacroObjectLike),
    Function(&'a MacroFunctionLike),
}

struct MacroGen<'ifc, 'gen> {
    ifc: &'ifc Ifc,
    output_macros: HashMap<&'ifc str, Result<TokenStream>>,
    symbol_map: &'gen SymbolMap,
    is_object_like: HashMap<&'ifc str, bool>,
    work_queue: Vec<(&'ifc str, MacroObjectOrFunction<'ifc>)>,
}

impl<'ifc, 'gen> MacroGen<'ifc, 'gen> {
    /// Adds an object-like macro into the output set.
    fn add_object_like_macro(&mut self, object: &MacroObjectLike, name: &'ifc str) {
        let result = || -> Result<TokenStream> {
            trace!("processing: #define {} ...", name);
            let body = self.remove_parens(object.body)?;
            let mut one_output = TokenStream::new();
            self.gen_macro_body(body, name, &mut one_output)?;
            let ident = Ident::new(name, Span::mixed_site());
            Ok(quote_spanned! {Span::mixed_site()=>
                #[macro_export]
                macro_rules! #ident {
                    () => {
                        #one_output
                    };
                }
            })
        }();

        assert!(self.output_macros.insert(name, result).is_none());
        assert!(self.is_object_like.insert(name, true).is_none());
    }

    /// Adds an function-like macro into the output set.
    fn add_function_like_macro(&mut self, func_like: &MacroFunctionLike, name: &'ifc str) {
        let result = || -> Result<TokenStream> {
            trace!("processing: #define {} ...", name);
            if func_like.is_variadic() {
                bail!("variadic macros not supported");
            }

            let body = self.remove_parens(func_like.body)?;
            let mut one_output = TokenStream::new();
            self.gen_macro_body(body, name, &mut one_output)?;

            let ident = Ident::new(name, Span::mixed_site());

            assert!(func_like.arity() == 0 || func_like.parameters.tag() == FormSort::TUPLE);
            let params_tuple = self.ifc.pp_tuple().entry(func_like.parameters.index())?;
            let parameters = (0..params_tuple.cardinality)
                .map(|i| {
                    let param_form = *self.ifc.heap_form().entry(params_tuple.start + i as u32)?;
                    assert_eq!(param_form.tag(), FormSort::PARAMETER);
                    let parameter = self.ifc.pp_param().entry(param_form.index())?;
                    Ok(Ident::new(
                        self.ifc.get_string(parameter.spelling)?,
                        Span::mixed_site(),
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(quote_spanned! {Span::mixed_site()=>
                #[macro_export]
                macro_rules! #ident {
                    (#($#parameters:expr),*) => {
                        #one_output
                    };
                }
            })
        }();

        assert!(self.output_macros.insert(name, result).is_none());
        assert!(self.is_object_like.insert(name, false).is_none());
    }

    /// Examines an C++ macro and attempts to convert it to a Rust macro.
    fn gen_macro_body(
        &mut self,
        body: FormIndex,
        name: &str,
        output: &mut TokenStream,
    ) -> Result<()> {
        // It's not possible to convert all #define macros to Rust constants. We look for certain
        // patterns. The simplest macros to convert consist of a single literal constant, or a
        // a single literal constant inside parens.  More complex expressions that use operators
        // and such are more difficult to convert.  Complex expressions that refer to other
        // macros or constants are the hardest.

        match body.tag() {
            FormSort::IDENTIFIER => {
                self.append_identifier(body, output)?;
            }
            FormSort::NUMBER => {
                output.extend(self.convert_form_number_to_tokens(body)?);
            }
            FormSort::OPERATOR => {
                let op = self.ifc.pp_op().entry(body.index())?;
                match op.operator.value() {
                    PreProcessingOpOrPunc::Punctuator(WordSortPunctuator::LeftParenthesis) => {
                        output.append(Punct::new('(', proc_macro2::Spacing::Alone));
                    }
                    PreProcessingOpOrPunc::Punctuator(WordSortPunctuator::RightParenthesis) => {
                        output.append(Punct::new(')', proc_macro2::Spacing::Alone));
                    }
                    PreProcessingOpOrPunc::Operator(WordSortOperator::Bar) => {
                        output.append(Punct::new('|', proc_macro2::Spacing::Alone));
                    }
                    PreProcessingOpOrPunc::Operator(WordSortOperator::Comma) => {
                        output.append(Punct::new(',', proc_macro2::Spacing::Alone));
                    }
                    PreProcessingOpOrPunc::Operator(WordSortOperator::Dash) => {
                        output.append(Punct::new('-', proc_macro2::Spacing::Alone));
                    }
                    PreProcessingOpOrPunc::Operator(WordSortOperator::LeftChevron) => {
                        output.append(Punct::new('<', proc_macro2::Spacing::Joint));
                        output.append(Punct::new('<', proc_macro2::Spacing::Alone));
                    }
                    PreProcessingOpOrPunc::Operator(WordSortOperator::Plus) => {
                        output.append(Punct::new('+', proc_macro2::Spacing::Alone));
                    }
                    PreProcessingOpOrPunc::Operator(WordSortOperator::RightChevron) => {
                        output.append(Punct::new('>', proc_macro2::Spacing::Joint));
                        output.append(Punct::new('>', proc_macro2::Spacing::Alone));
                    }
                    _ => {
                        bail!("Unsupported op or punc: {:?}", op.operator.value())
                    }
                }
            }
            FormSort::PARAMETER => {
                let parameter = self.ifc.pp_param().entry(body.index())?;
                let ident =
                    Ident::new(self.ifc.get_string(parameter.spelling)?, Span::mixed_site());
                output.extend(quote_spanned!(Span::mixed_site()=>$#ident));
            }
            FormSort::TUPLE => {
                let tuple = self.ifc.pp_tuple().entry(body.index())?;

                let is_operator = |i: u32, op_or_punct: PreProcessingOpOrPunc| -> Result<bool> {
                    let element = *self.ifc.heap_form().entry(tuple.start + i)?;
                    if element.tag() == FormSort::OPERATOR {
                        let op = self.ifc.pp_op().entry(element.index())?;
                        Ok(op.operator.value() == op_or_punct)
                    } else {
                        Ok(false)
                    }
                };

                let is_identifier = |i: u32, target_spellings: &[&str]| -> Result<bool> {
                    let element = *self.ifc.heap_form().entry(tuple.start + i)?;
                    if element.tag() == FormSort::IDENTIFIER {
                        if !target_spellings.is_empty() {
                            let op = self.ifc.pp_ident().entry(element.index())?;
                            Ok(target_spellings.contains(&self.ifc.get_string(op.spelling)?))
                        } else {
                            Ok(true)
                        }
                    } else {
                        Ok(false)
                    }
                };

                let mut i = 0;
                loop {
                    // Look for C-style cast:
                    // ( IDENTIFIER )
                    if i + 2 < tuple.cardinality
                        && is_operator(
                            i,
                            PreProcessingOpOrPunc::Punctuator(WordSortPunctuator::LeftParenthesis),
                        )?
                        && is_identifier(i + 1, &[])?
                        && is_operator(
                            i + 2,
                            PreProcessingOpOrPunc::Punctuator(WordSortPunctuator::RightParenthesis),
                        )?
                    {
                        // TODO: Should we cast?
                        warn!("#define {}: dropping possible cast", name);
                        i += 3;
                        continue;
                    }

                    // Look for C-style cast:
                    // ( [signed|unsigned] IDENTIFIER )
                    if i + 4 < tuple.cardinality
                        && is_operator(
                            i,
                            PreProcessingOpOrPunc::Punctuator(WordSortPunctuator::LeftParenthesis),
                        )?
                        && is_identifier(i + 1, &["signed", "unsigned"])?
                        && is_identifier(i + 2, &[])?
                        && is_operator(
                            i + 3,
                            PreProcessingOpOrPunc::Punctuator(WordSortPunctuator::RightParenthesis),
                        )?
                    {
                        // TODO: Should we cast?
                        warn!("#define {}: dropping possible cast", name);
                        i += 4;
                        continue;
                    }

                    self.gen_macro_body(
                        *self.ifc.heap_form().entry(tuple.start + i)?,
                        name,
                        output,
                    )?;
                    i += 1;
                    if i == tuple.cardinality {
                        break;
                    }
                }
            }
            _ => {
                bail!("definition did not match any supported pattern: {:?}", body);
            }
        }

        Ok(())
    }

    /// Appends an identifier to the output.
    fn append_identifier(&mut self, form: FormIndex, output: &mut TokenStream) -> Result<()> {
        assert_eq!(form.tag(), FormSort::IDENTIFIER);

        let identifier = self.ifc.pp_ident().entry(form.index())?;
        let name = self.ifc.get_string(identifier.spelling)?;
        let is_object_like = self
            .is_object_like
            .get(name)
            .map(|&b| (None, b))
            .or_else(|| {
                if let Some(ifc_name) = self.symbol_map.resolve_object_like_macro(name) {
                    Some((Some(ifc_name), true))
                } else if let Some(ifc_name) = self.symbol_map.resolve_function_like_macro(name) {
                    Some((Some(ifc_name), false))
                } else if let Some(macro_obj_or_func) = self.try_find_macro_in_current_ifc(name) {
                    self.work_queue.push((name, macro_obj_or_func));
                    match macro_obj_or_func {
                        MacroObjectOrFunction::Object(_) => Some((None, true)),
                        MacroObjectOrFunction::Function(_) => Some((None, false)),
                    }
                } else {
                    None
                }
            });
        let ident = Ident::new(name, Span::mixed_site());
        match is_object_like {
            Some((Some(ifc_ident), true)) => {
                output.extend(quote_spanned!(Span::mixed_site()=>#ifc_ident::#ident!()))
            }
            Some((Some(ifc_ident), false)) => {
                output.extend(quote_spanned!(Span::mixed_site()=>#ifc_ident::#ident!))
            }
            Some((None, true)) => {
                output.extend(quote_spanned!(Span::mixed_site()=>$crate::#ident!()))
            }
            Some((None, false)) => {
                output.extend(quote_spanned!(Span::mixed_site()=>$crate::#ident!))
            }
            None => output.append(ident),
        }
        Ok(())
    }

    /// Attempts to find a macro with the given name.
    fn try_find_macro_in_current_ifc(
        &self,
        name: &'ifc str,
    ) -> Option<MacroObjectOrFunction<'ifc>> {
        self.ifc
            .macro_object_like()
            .entries
            .iter()
            .find(|object| {
                self.ifc
                    .get_string(object.name)
                    .map_or(false, |obj_name| obj_name == name)
            })
            .map(MacroObjectOrFunction::Object)
            .or_else(|| {
                self.ifc
                    .macro_function_like()
                    .entries
                    .iter()
                    .find(|object| {
                        self.ifc
                            .get_string(object.name)
                            .map_or(false, |obj_name| obj_name == name)
                    })
                    .map(MacroObjectOrFunction::Function)
            })
    }

    /// Converts a C++ macro number into a Rust number.
    fn convert_form_number_to_tokens(&self, form: FormIndex) -> Result<TokenStream> {
        assert_eq!(form.tag(), FormSort::NUMBER);

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
                    Ok(quote_spanned!(Span::mixed_site()=>u64::MAX))
                } else {
                    write!(chars, "{}", value_u64).unwrap();
                    let lit = syn::LitInt::new(&chars, Span::call_site()).to_token_stream();
                    Ok(quote_spanned!(Span::mixed_site()=>#lit as u64))
                }
            } else {
                // signed
                let mut value_i64 = value_u128 as i64;
                if is_negative {
                    value_i64 = value_i64.wrapping_neg();
                }

                // Special case for "most negative" value. This value will not change its sign
                // if we negate it, because it oveflows. Such is the price of 2's complement.
                if value_i64 == i64::MIN {
                    Ok(quote_spanned!(Span::mixed_site()=>i64::MIN))
                } else if value_i64 == i64::MAX {
                    Ok(quote_spanned!(Span::mixed_site()=>i64::MAX))
                } else if is_negative {
                    value_i64 = -value_i64;
                    Ok(quote_spanned!(Span::mixed_site()=> - #value_i64))
                } else {
                    Ok(quote_spanned!(Span::mixed_site()=>#value_i64))
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
                    Ok(quote_spanned!(Span::mixed_site()=>u32::MAX))
                } else {
                    Ok(quote_spanned!(Span::mixed_site()=>#value_u32))
                }
            } else {
                // signed
                let mut value_i32 = value_u128 as i32;
                if is_negative {
                    value_i32 = value_i32.wrapping_neg();
                }

                // Special case for "most negative" value. This value will not change its sign
                // if we negate it, because it oveflows. Such is the price of 2's complement.
                if value_i32 == i32::MIN {
                    Ok(quote_spanned!(Span::mixed_site()=>i32::MIN))
                } else if value_i32 == i32::MAX {
                    Ok(quote_spanned!(Span::mixed_site()=>i32::MAX))
                } else if is_negative {
                    value_i32 = -value_i32;
                    Ok(quote_spanned!(Span::mixed_site()=> - #value_i32))
                } else {
                    Ok(quote_spanned!(Span::mixed_site()=> #value_i32))
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

impl<'a> Gen<'a> {
    /// Generates Rust macros from C++ macros.
    pub fn gen_macros(
        &self,
        symbol_map: &SymbolMap,
        filter: options::Filter,
        output_macros: &mut Vec<TokenStream>,
    ) -> Result<()> {
        let mut macro_gen = MacroGen {
            ifc: self.ifc,
            output_macros: HashMap::new(),
            symbol_map,
            is_object_like: HashMap::new(),
            work_queue: Vec::new(),
        };

        // Add object-like macros that pass the filter.
        for object in self.ifc.macro_object_like().entries.iter() {
            let name = self.ifc.get_string(object.name)?;
            if filter.is_allowed(name) && !symbol_map.is_object_like_macro_in(name) {
                macro_gen.add_object_like_macro(object, name);
            }
        }

        // Add function-like macros that pass the filter.
        for func_like in self.ifc.macro_function_like().entries.iter() {
            let name = self.ifc.get_string(func_like.name)?;
            if filter.is_allowed(name) && !symbol_map.is_function_like_macro_in(name) {
                macro_gen.add_function_like_macro(func_like, name);
            }
        }

        // Process the work queue.
        while let Some((name, macro_obj_or_func)) = macro_gen.work_queue.pop() {
            // Check if already added.
            if !macro_gen.output_macros.contains_key(name) {
                match macro_obj_or_func {
                    MacroObjectOrFunction::Object(object) => {
                        macro_gen.add_object_like_macro(object, name)
                    }
                    MacroObjectOrFunction::Function(func_like) => {
                        macro_gen.add_function_like_macro(func_like, name)
                    }
                }
            }
        }

        let mut macros = macro_gen.output_macros.into_iter().collect::<Vec<_>>();
        macros.sort_by_key(|(k, _)| *k);
        output_macros.extend(
            macros
                .into_iter()
                .filter_map(|(name, v)| log_error! { { v } -> TokenStream, format!("Generating macro #define {}", name) }),
        );

        Ok(())
    }
}
