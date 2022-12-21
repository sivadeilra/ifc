use super::*;

impl<'a> Gen<'a> {
    pub fn gen_variable(
        &self,
        var_index: u32,
        filter: &Option<options::Filter>,
        parent_scope_name: &str,
        outputs: &mut GenOutputs,
    ) -> Result<()> {
        let var = self.ifc.decl_var().entry(var_index)?;

        if var.name.tag() != NameSort::IDENTIFIER {
            info!("Found VARIABLE, but its name is not IDENTIFIER.  Ignoring.");
            return Ok(());
        }
        let var_name = self.ifc.get_string(var.name.index())?;

        if let Some(filter) = filter &&
            !filter.is_allowed_qualified_name(var_name, parent_scope_name) {
                return Ok(())
        }

        let var_ident = Ident::new(var_name, Span::call_site());

        let is_const = var.traits.contains(ObjectTraits::CONSTEXPR) || (self.ifc.is_const_qualified(var.ty)? && self.ifc.is_literal_expr(var.initializer));

        if is_const {
            let ty_tokens = self.get_type_tokens(var.ty)?;
            let init_tokens = self.gen_expr_tokens(var.ty, var.initializer)?;
            outputs.consts.extend(quote! {
                pub const #var_ident: #ty_tokens = #init_tokens;
            });
            // } else if var.specifier.contains(BasicSpecifiers::EXTERNAL) {
        } else {
            // This is a variable declaration, not a definition. We can emit an "extern static" item.
            let ty_tokens = self.get_type_tokens(var.ty)?;

            let mut_kw = if !self.ifc.is_const_qualified(var.ty)? {
                quote!(mut)
            } else {
                quote!()
            };

            outputs.extern_cdecl.extend(quote! {
                pub static #mut_kw #var_ident: #ty_tokens;
            })
        }

        Ok(())
    }
}
