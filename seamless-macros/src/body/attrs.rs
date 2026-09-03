pub struct Props {
    pub docs: String,
    pub tag: Option<String>,
    pub flatten: bool,
}

pub static NAME: &str = "api_body";

pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Props> {
    let mut props = Props {
        docs: String::new(),
        tag: None,
        flatten: false,
    };

    for attr in attrs {
        // If the attr is serde based, error! not allowed
        if attr.path().is_ident("serde") {
            return Err(syn::Error::new_spanned(
                attr,
                "serde attributes not allowed; ApiBody macro handles that",
            ));
        }

        // Process doc strings:
        if let Some(doc) = extract_doc_string(attr) {
            if !props.docs.is_empty() {
                props.docs.push('\n');
            }
            props.docs.push_str(&doc);
        }

        // Ignore attrs we don't care about and copy them for output
        if !attr.path().is_ident(NAME) {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            let value = meta.value();
            let path = meta.path;

            if path.is_ident("tag") {
                props.tag = Some(lit_string(&value?.parse()?)?);
            } else if path.is_ident("flatten") {
                props.flatten = true;
            } else {
                return Err(syn::Error::new_spanned(path, "unrecognized attribute"));
            }

            Ok(())
        })?;
    }

    Ok(props)
}

fn extract_doc_string(attr: &syn::Attribute) -> Option<String> {
    let syn::Meta::NameValue(nv) = &attr.meta else {
        return None;
    };

    if !nv.path.is_ident("doc") {
        return None;
    }

    let doc_string = lit_string(&nv.value).ok()?.trim_start().to_owned();
    Some(doc_string)
}

fn lit_string(expr: &syn::Expr) -> syn::Result<String> {
    match expr {
        syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => Ok(s.value()),
        bad => Err(syn::Error::new_spanned(bad, "string literal required here")),
    }
}
