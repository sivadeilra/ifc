use super::*;

impl<'a> Gen<'a> {
    pub fn gen_variable(&self, var_index: u32, name: &str) -> Result<TokenStream> {
        let var = self.ifc.decl_var().entry(var_index)?;

        let var_ident = Ident::new(name, Span::call_site());

        let is_const = var.traits.contains(ObjectTraits::CONSTEXPR)
            || (self.ifc.is_const_qualified(var.ty)? && self.ifc.is_literal_expr(var.initializer));

        if is_const {
            trace!("var {} is a definition", name);

            let ty_tokens = self.get_type_tokens(var.ty)?;
            let init_tokens = self.gen_expr_tokens(Some(var.ty), var.initializer)?;
            Ok(quote! {
                pub const #var_ident: #ty_tokens = #init_tokens;
            })
            // } else if var.specifier.contains(BasicSpecifiers::EXTERNAL) {
        } else {
            trace!("var {} is a declaration", name);

            // This is a variable declaration, not a definition. We can emit an "extern static" item.
            let ty_tokens = self.get_type_tokens(var.ty)?;

            let mut_kw = if !self.ifc.is_const_qualified(var.ty)? {
                quote!(mut)
            } else {
                quote!()
            };

            Ok(quote! {
                extern {
                    pub static #mut_kw #var_ident: #ty_tokens;
                }
            })
        }
    }
}
