use proc_macro2::*;
use quote::{quote, ToTokens};
use syn::parse::*;
use syn::parse_macro_input;
use syn::punctuated::*;
use syn::spanned::Spanned;
use syn::ItemEnum;
use syn::*;

struct CEnumAttrs {
    settings: Punctuated<Meta, Token![,]>,
}

impl Parse for CEnumAttrs {
    fn parse(stream: ParseStream) -> Result<Self> {
        Ok(Self {
            settings: Punctuated::parse_terminated(stream)?,
        })
    }
}

fn attrs_to_outer_tokens(attrs: &[Attribute]) -> TokenStream {
    let mut tokens = TokenStream::new();
    for attr in attrs.iter() {
        let attr_path = &attr.path;
        let attr_tokens = &attr.tokens;
        let q = quote! {
            #[ #attr_path #attr_tokens ]
        };
        tokens.extend(q);
    }
    tokens
}

#[proc_macro_attribute]
pub fn c_enum(
    attr: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut errors: Vec<Error> = Vec::new();

    let macro_args: CEnumAttrs = parse_macro_input!(attr as CEnumAttrs);

    let kw_storage = Ident::new("storage", Span::call_site());
    let kw_old_name = Ident::new("old_name", Span::call_site());
    let kw_com = Ident::new("com", Span::call_site());
    let kw_flags = Ident::new("flags", Span::call_site());
    let kw_unscoped = Ident::new("unscoped", Span::call_site());
    let kw_unscoped_prefix = Ident::new("unscoped_prefix", Span::call_site());

    let mut old_name: Option<Ident> = None;
    let mut is_com_compatible = false;
    let mut storage: Option<Ident> = None;
    let mut is_flags = false;
    let mut is_unscoped = false;
    let mut unscoped_prefix: Option<String> = None;

    for arg in macro_args.settings.iter() {
        match arg {
            Meta::NameValue(nv) => {
                if nv.path.is_ident(&kw_old_name) {
                    match &nv.lit {
                        Lit::Str(s) => {
                            old_name = Some(Ident::new(s.value().as_str(), nv.lit.span()));
                            continue;
                        }
                        _ => {}
                    }
                } else if nv.path.is_ident(&kw_storage) {
                    if storage.is_none() {
                        match &nv.lit {
                            Lit::Str(s) => {
                                storage = Some(Ident::new(s.value().as_str(), nv.lit.span()));
                                continue;
                            }
                            _ => {}
                        }
                    } else {
                        errors.push(Error::new(
                            arg.span(),
                            "cannot specify storage type more than once.",
                        ));
                    }
                } else if nv.path.is_ident(&kw_unscoped_prefix) {
                    match &nv.lit {
                        Lit::Str(s) => {
                            is_unscoped = true;
                            if unscoped_prefix.is_some() {
                                errors.push(Error::new(
                                    arg.span(),
                                    "cannot specify `unscoped_prefix` more than once.",
                                ));
                            } else {
                                unscoped_prefix = Some(s.value());
                            }
                            continue;
                        }
                        _ => {}
                    }
                }
            }
            Meta::Path(p) => {
                if p.is_ident(&kw_com) {
                    is_com_compatible = true;
                    continue;
                } else if p.is_ident(&kw_flags) {
                    is_flags = true;
                    continue;
                } else if p.is_ident(&kw_unscoped) {
                    is_unscoped = true;
                    continue;
                }
            }
            _ => {}
        }

        errors.push(Error::new(arg.span(), "Option is not recognized."));
    }

    let storage: Ident = if let Some(storage) = storage {
        storage
    } else {
        // Use i32 as the default.
        Ident::new("i32", Span::call_site())
    };

    let en: ItemEnum = parse_macro_input!(input as ItemEnum);
    let en_vis = &en.vis;
    let en_ident = &en.ident;
    let en_attrs_tokens = attrs_to_outer_tokens(&en.attrs);

    // If this is Some(Error), then the previous discriminant could not
    // be parsed as an integer literal.
    let mut last_value: Option<Ident> = None; // ident of last variant

    let const_defs = en
        .variants
        .iter()
        .map(|var| {
            let var_ident = &var.ident;
            let var_value: TokenStream;
            if let Some((_eq, discriminant)) = &var.discriminant {
                var_value = discriminant.to_token_stream();
            } else {
                // No discriminant specified.
                if is_flags {
                    errors.push(Error::new(
                        var.span(),
                        "When using the `flags` option, each variant must specify a value.",
                    ));
                    var_value = quote!(0);
                } else {
                    // Use the last value, if any.
                    match &last_value {
                        None => {
                            var_value = quote!(0);
                        }
                        Some(last_ident) => {
                            var_value = quote!(
                                Self::#last_ident.0 + 1
                            );
                        }
                    }
                }
            }
            last_value = Some(var_ident.clone());

            return quote! {
                #[allow(non_upper_case_globals)]
                #en_vis const #var_ident: #en_ident = #en_ident( #var_value );
            };
        })
        .collect::<TokenStream>();

    let dbg_arms = en
        .variants
        .iter()
        .map(|var| {
            let var_ident = &var.ident;
            let var_ident_string = var_ident.to_string();
            quote! {
                Self::#var_ident => #var_ident_string,
            }
        })
        .collect::<TokenStream>();

    let errors_tokens: TokenStream = errors
        .iter()
        .map(|e| e.to_compile_error().into_token_stream())
        .collect();

    let is_valid_method: TokenStream;
    if is_flags {
        // Bit flags.
        let bit_flags_union = en
            .variants
            .iter()
            .map(|var| {
                if let Some((_, discriminant)) = &var.discriminant {
                    quote!(| (#discriminant))
                } else {
                    quote!()
                }
            })
            .collect::<TokenStream>();
        is_valid_method = quote! {
            pub const fn is_valid(self) -> bool {
                const ALL_VALID_BITS: #storage = 0 #bit_flags_union;
                (self.0 & !ALL_VALID_BITS) == 0
            }
        };
    } else {
        // Normal enumerated type.
        let is_valid_arms = en
            .variants
            .iter()
            .map(|var| {
                let var_ident = &var.ident;
                quote!(| Self::#var_ident)
            })
            .collect::<TokenStream>();
        is_valid_method = quote! {
            pub const fn is_valid(self) -> bool {
                match self {
                    #is_valid_arms => true,
                    _ => false
                }
            }
        };
    }

    let mut output: TokenStream = quote! {
        #errors_tokens

        #en_attrs_tokens
        #[derive(Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd, Default)]
        #[derive(zerocopy::FromBytes, zerocopy::AsBytes)]
        #[repr(transparent)]
        #en_vis struct #en_ident(pub #storage);

        impl #en_ident {
            pub const fn zero() -> #en_ident { #en_ident(0) }

            #const_defs
            #is_valid_method
        }

        impl core::fmt::Debug for #en_ident {
            fn fmt(&self, fmt: &mut core::fmt::Formatter) -> core::fmt::Result {
                #![allow(unreachable_patterns)]
                let s = match *self {
                    #dbg_arms
                    _ => return write!(fmt, "{}", self.0),
                };
                fmt.write_str(s)
            }
        }
    };

    if let Some(old_name) = &old_name {
        output.extend(quote! {
            #[allow(non_camel_case_types)]
            #en_vis use #en_ident as #old_name;
        });
    }

    if is_com_compatible {
        output.extend(quote! {
            unsafe impl com::AbiTransferable for #en_ident {
                type Abi = Self;
                fn get_abi(&self) -> Self::Abi {
                    *self
                }
                fn set_abi(&mut self) -> *mut Self::Abi {
                    self as *mut Self::Abi
                }
            }
        });
    }

    if is_flags {
        output.extend(quote! {
            impl core::ops::BitOr<Self> for #en_ident {
                type Output = Self;
                #[must_use]
                fn bitor(self, rhs: Self) -> Self::Output {
                    Self(self.0 | rhs.0)
                }
            }

            impl core::ops::BitAnd<Self> for #en_ident {
                type Output = Self;
                #[must_use]
                fn bitand(self, rhs: Self) -> Self::Output {
                    Self(self.0 & rhs.0)
                }
            }

            impl core::ops::BitXor<Self> for #en_ident {
                type Output = Self;
                #[must_use]
                fn bitxor(self, rhs: Self) -> Self::Output {
                    Self(self.0 & rhs.0)
                }
            }

            impl core::ops::Not for #en_ident {
                type Output = Self;
                #[must_use]
                fn not(self) -> Self::Output {
                    Self(!self.0)
                }
            }
        });
    }

    if is_unscoped {
        for var in en.variants.iter() {
            let var_ident = &var.ident;
            let unscoped_ident_new: Ident;
            let unscoped_ident = if let Some(prefix) = &unscoped_prefix {
                unscoped_ident_new = Ident::new(
                    &format!("{}{}", prefix, var.ident.to_string()),
                    var.ident.span(),
                );
                &unscoped_ident_new
            } else {
                var_ident
            };
            let var_attrs = attrs_to_outer_tokens(&var.attrs);

            output.extend(quote! {
                #var_attrs
                #en_vis const #unscoped_ident: #en_ident = #en_ident::#var_ident;
            });
        }
    }

    output.into()
}
