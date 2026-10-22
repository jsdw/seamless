/// What the generated `From<T> for ApiError` impl should do for a struct or enum variant.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum ErrorImpl {
    /// Delegate to the `Into<ApiError>` impl of the single unnamed field.
    Delegate,
    /// Build an `ApiError` using the `Display` impl as the internal message.
    Convert {
        code: syn::Expr,
        /// `None` means that the `Display` impl is used as the external message too.
        external_message: Option<syn::Expr>,
    },
}

#[derive(Debug, Default)]
pub struct ApiErrorAttrs {
    external_tok: Option<syn::Path>,
    internal_tok: Option<syn::Path>,
    inner_tok: Option<syn::Path>,
    external_message: Option<syn::Expr>,
    code: Option<syn::Expr>,
}

impl ApiErrorAttrs {
    pub fn finalise(self) -> ErrorImpl {
        if self.inner_tok.is_some() {
            return ErrorImpl::Delegate;
        }

        let code = self.code.unwrap_or_else(|| syn::parse_quote!(500u16));

        if self.internal_tok.is_some() || self.external_message.is_some() {
            // The error message is for internal eyes only:
            let external_message = self.external_message.unwrap_or_else(|| {
                syn::parse_quote!(::seamless::api::ApiError::SERVER_ERROR)
            });
            ErrorImpl::Convert {
                code,
                external_message: Some(external_message),
            }
        } else if self.external_tok.is_some() {
            // The error message will be shown externally:
            ErrorImpl::Convert {
                code,
                external_message: None,
            }
        } else {
            // Not internal or external? Delegate to the inner error (or fail if there isn't one):
            ErrorImpl::Delegate
        }
    }

    pub fn finalise_with_parent_attrs(mut self, parent: &ApiErrorAttrs) -> syn::Result<ErrorImpl> {
        // Only variants can have an "inner" attr:
        if let Some(t) = &parent.inner_tok {
            return Err(syn::Error::new_spanned(
                t,
                "This is not allowed at the top level of an enum, only specific fields",
            ));
        }
        // If self does not identify as external or internal, use parent props for these:
        if self.external_tok.is_none()
            && self.internal_tok.is_none()
            && self.external_message.is_none()
        {
            self.external_tok = parent.external_tok.clone();
            self.internal_tok = parent.internal_tok.clone();
            self.external_message = parent.external_message.clone();
        }
        // If self doesn't have a code, use parent code if possible:
        if self.code.is_none() {
            self.code = parent.code.clone();
        }
        Ok(self.finalise())
    }

    pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<ApiErrorAttrs> {
        let mut out = ApiErrorAttrs::default();

        for attr in attrs.iter().filter(|attr| attr.path().is_ident("api_error")) {
            attr.parse_nested_meta(|meta| {
                let value = meta.value();
                let path = meta.path;

                if path.is_ident("internal") {
                    out.internal_tok = Some(path);
                } else if path.is_ident("external") {
                    // Either `external` or `external = <expr>`:
                    match value {
                        Ok(value) => out.external_message = Some(value.parse()?),
                        Err(_) => out.external_tok = Some(path),
                    }
                } else if path.is_ident("inner") {
                    out.inner_tok = Some(path);
                } else if path.is_ident("code") {
                    out.code = Some(value?.parse()?);
                } else {
                    return Err(syn::Error::new_spanned(path, "unrecognized attribute"));
                }

                Ok(())
            })?;
        }

        // A thing can't be marked "inner" and have any other internal/external/code props,
        // since we'll be ignoring them all anyway:
        if let Some(inner_tok) = &out.inner_tok
            && (out.external_tok.is_some()
                || out.external_message.is_some()
                || out.internal_tok.is_some()
                || out.code.is_some())
        {
            return Err(syn::Error::new_spanned(
                inner_tok,
                "'inner' does not make sense alongside any other attributes",
            ));
        }

        // A thing can't be "external" and "internal" at once:
        if let Some(external_tok) = &out.external_tok
            && out.internal_tok.is_some()
        {
            return Err(syn::Error::new_spanned(
                external_tok,
                "'internal' and 'external' can't be declared together",
            ));
        }

        // Can't have external and external = "foo" at once:
        if let Some(external_tok) = &out.external_tok
            && out.external_message.is_some()
        {
            return Err(syn::Error::new_spanned(
                external_tok,
                "'external' and 'external = \"foo\"' can't be declared together",
            ));
        }

        Ok(out)
    }
}
