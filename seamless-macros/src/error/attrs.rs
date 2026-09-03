#[derive(Debug)]
pub struct FinalApiErrorAttrs {
    pub external_message: Option<String>,
    pub code: u16,
    pub delegate_to_child: bool
}

#[derive(Debug)]
pub struct ApiErrorAttrs {
    external_tok: Option<syn::Path>,
    internal_tok: Option<syn::Path>,
    inner_tok: Option<syn::Path>,
    external_message: Option<syn::LitStr>,
    code: Option<syn::LitInt>
}

impl ApiErrorAttrs {
    pub fn finalise(mut self) -> syn::Result<FinalApiErrorAttrs> {
        let code = self.code
            .unwrap_or(syn::parse_quote!(500))
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
        let mut internal_tok: Option<syn::Path> = None;
        let mut external_tok: Option<syn::Path> = None;
        let mut inner_tok: Option<syn::Path> = None;
        let mut external_message: Option<syn::LitStr> = None;
        let mut code: Option<syn::LitInt> = None;

        for attr in attrs {
            // Ignore all attributes we don't care about
            if !attr.path().is_ident("api_error") {
                continue
            }

            attr.parse_nested_meta(|meta| {
                let value = meta.value();
                let path = meta.path;

                if path.is_ident("internal") {
                    internal_tok = Some(path);
                } else if path.is_ident("external") {
                    match value {
                        Ok(val) => {
                            // If `= value`, parse the value as a string
                            external_message = Some(val.parse()?);
                        },
                        Err(_) => {
                            // If no value, that's all good too.
                            external_tok = Some(path);
                        }
                    }
                } else if path.is_ident("inner") {
                    inner_tok = Some(path);
                } else if path.is_ident("code") {
                    // Here we expect `= number` else we'll error
                    code = Some(value?.parse()?);
                } else {
                    return Err(syn::Error::new_spanned(path, "unrecognized attribute"))
                }

                Ok(())
            })?;
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
            external_tok: external_tok,
            internal_tok: internal_tok,
            inner_tok: inner_tok,
            external_message: external_message,
            code: code
        })
    }
}
