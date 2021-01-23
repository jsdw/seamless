use syn::{ spanned::Spanned };

#[derive(Debug)]
pub struct FinalApiErrorAttrs {
    pub external_message: Option<String>,
    pub code: u16,
    pub delegate_to_child: bool
}

#[derive(Debug)]
pub struct ApiErrorAttrs {
    attr_tok: Option<syn::Attribute>,
    external_tok: Option<syn::Path>,
    internal_tok: Option<syn::Path>,
    inner_tok: Option<syn::Path>,
    external_message: Option<syn::LitStr>,
    code: Option<syn::LitInt>
}

impl ApiErrorAttrs {
    pub fn finalise(mut self) -> syn::Result<FinalApiErrorAttrs> {

        let attr_span = self.attr_tok.span();
        let code = self.code
            .unwrap_or(syn::LitInt::new("500", attr_span))
            .base10_parse::<u16>()?;
        let parse_str = |s: Option<syn::LitStr>| {
            s.map(|s| s.value()).unwrap_or(String::from("Internal server error"))
        };

        // if there is an inner attr, force delegation to child:
        if self.inner_tok.is_some() {
            self.internal_tok = None;
            self.external_tok = None;
            self.external_message = None;
            self.code = None;
        }

        // Invalid: 'external' and 'external = "foo"' makes no sense (if err is external, can't provide an external msg too!)
        if self.external_tok.is_some() && self.external_message.is_some() {
            Err(syn::Error::new_spanned(self.external_message.unwrap(), "'external' and 'external = \"foo\"' shouldn't both be provided"))
        }
        // The error will be internal only:
        else if self.internal_tok.is_some() || self.external_message.is_some() {
            Ok(FinalApiErrorAttrs {
                external_message: Some(parse_str(self.external_message)),
                code: code,
                delegate_to_child: false
            })
        }
        // Error will be shown externally:
        else if self.external_tok.is_some() {
            Ok(FinalApiErrorAttrs {
                external_message: None,
                code: code,
                delegate_to_child: false
            })
        }
        // Not internal or external? Delegate to the child impl (enums) or error if we can't:
        else {
            Ok(FinalApiErrorAttrs {
                external_message: None,
                code: 0,
                delegate_to_child: true
            })
        }
    }
    pub fn finalise_with_parent_attrs(mut self, parent: &ApiErrorAttrs) -> syn::Result<FinalApiErrorAttrs> {
        // Only self can have an "inner" attr:
        if let Some(t) = &parent.inner_tok {
            return Err(syn::Error::new_spanned(t, "This is not allowed at the top level of an enum, only specific fields"))
        }
        // If self does not identify as external or internal, use parent props for these:
        if self.external_tok.is_none() && self.internal_tok.is_none() && self.external_message.is_none() {
            self.external_tok = parent.external_tok.clone();
            self.internal_tok = parent.internal_tok.clone();
            self.external_message = parent.external_message.clone();
        }
        // If self doesn't have a code, use parent code if possible:
        if self.code.is_none() {
            self.code = parent.code.clone();
        }
        self.finalise()
    }
    pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<ApiErrorAttrs> {

        let mut attr_tok = None;
        let mut internal_tok: Option<syn::Path> = None;
        let mut external_tok: Option<syn::Path> = None;
        let mut inner_tok: Option<syn::Path> = None;
        let mut external_message: Option<syn::LitStr> = None;
        let mut code: Option<syn::LitInt> = None;

        let lit_str = |lit: syn::Lit| {
            match lit {
                syn::Lit::Str(s) => Ok(s),
                bad => Err(syn::Error::new_spanned(bad, "string literal required here"))
            }
        };
        let lit_int = |lit: syn::Lit| {
            match lit {
                syn::Lit::Int(i) => Ok(i),
                bad => Err(syn::Error::new_spanned(bad, "u16 integer literal required here"))
            }
        };

        for attr in attrs {
            // Ignore all attributes we don't care about
            if !attr.path.is_ident("api_error") {
                continue
            }
            attr_tok = Some(attr.clone());

            // We should have a list of meta attributes inside the attr path
            let meta_list = match attr.parse_meta()? {
                syn::Meta::List(list) => list,
                bad => return Err(syn::Error::new_spanned(bad, "unrecognized attribute"))
            };

            for item in meta_list.nested {
                // Each list item should be a meta item:
                let meta = match item {
                    syn::NestedMeta::Meta(meta) => meta,
                    bad => return Err(syn::Error::new_spanned(bad, "unrecognized attribute"))
                };

                match meta {
                    // Handle eg #[api_error(internal, external)]
                    syn::Meta::Path(path) => {
                        if path.is_ident("internal") {
                            internal_tok = Some(path);
                        } else if path.is_ident("external") {
                            external_tok = Some(path);
                        } else if path.is_ident("inner") {
                            inner_tok = Some(path)
                        } else {
                            return Err(syn::Error::new_spanned(path, "unrecognized attribute"))
                        }
                    },
                    // Handle eg #[api_error(internal = "foo", external = "bar", code = 200)]
                    syn::Meta::NameValue(name_value) => {
                        if name_value.path.is_ident("external") {
                            external_message = Some(lit_str(name_value.lit)?);
                        } else if name_value.path.is_ident("code") {
                            code = Some(lit_int(name_value.lit)?);
                        } else {
                            return Err(syn::Error::new_spanned(name_value, "unrecognized attribute"))
                        }
                    },
                    bad => return Err(syn::Error::new_spanned(bad, "unrecognized attribute"))
                }
            }
        }

        // A thing can't be marked "inner" and have any other internal/external/code props,
        // since we'll be ignoring them all anyway:
        if inner_tok.is_some() &&
            (external_tok.is_some() || external_message.is_some()
            || internal_tok.is_some() || code.is_some()) {
                return Err(syn::Error::new_spanned(external_tok.unwrap(),
                "'inner' does not make sense alongside any other attributes"))
        }

        // A thing can't be "external" and "internal" at once:
        if external_tok.is_some() && internal_tok.is_some() {
            return Err(syn::Error::new_spanned(external_tok.unwrap(),
                    "'internal' and 'external' can't be declared together"))
        }

        // Can't have external and exteral = "foo" at once:
        if external_tok.is_some() && external_message.is_some() {
            return Err(syn::Error::new_spanned(external_tok.unwrap(),
                    "'external' and 'external = \"foo\"' can't be declared together"))
        }

        return Ok(ApiErrorAttrs {
            attr_tok: attr_tok,
            external_tok: external_tok,
            internal_tok: internal_tok,
            inner_tok: inner_tok,
            external_message: external_message,
            code: code
        })

    }
}

