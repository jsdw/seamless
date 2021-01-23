
pub struct Props {
    pub docs: String,
    pub tag: Option<String>,
    pub flatten: bool
}

pub static NAME: &'static str = "api_body";

pub fn parse(attrs: &[syn::Attribute]) -> syn::Result<Props> {

    let mut props = Props {
        docs: String::new(),
        tag: None,
        flatten: false
    };

    for attr in attrs {
        // If the attr is serde based, error! not allowed
        if attr.path.is_ident("serde") {
            return Err(syn::Error::new_spanned(attr, "serde attributes not allowed; ApiBody macro handles that"))
        }

        // Process doc strings:
        if let Some(doc) = extract_doc_string(attr) {
            if props.docs.len() > 0 { props.docs.push('\n'); }
            props.docs.push_str(&doc);
        }

        // Ignore attrs we don't care about and copy them for output
        if !attr.path.is_ident(NAME) {
            continue
        }

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
                // Handle eg #[typescript(tag = "foo")]
                syn::Meta::NameValue(name_value) => {
                    if name_value.path.is_ident("tag") {
                        props.tag = Some(lit_string(name_value.lit)?);
                    } else {
                        return Err(syn::Error::new_spanned(name_value, "unrecognized attribute"))
                    }
                },
                // Handle eg #[typescript(flatten)]
                syn::Meta::Path(path) => {
                    if path.is_ident("flatten") {
                        props.flatten = true;
                    } else {
                        return Err(syn::Error::new_spanned(path, "unrecognized attribute"))
                    }
                },
                bad => return Err(syn::Error::new_spanned(bad, "unrecognized attribute"))
            }
        }
    }

    Ok(props)
}

fn extract_doc_string(attr: &syn::Attribute) -> Option<String> {
    match attr.parse_meta().ok()? {
        syn::Meta::NameValue(nv) => {
            if nv.path.is_ident("doc") {
                let doc_string = lit_string(nv.lit).ok()?.trim_start().to_owned();
                Some(doc_string)
            } else {
                None
            }
        },
        _ => None
    }
}

fn lit_string(lit: syn::Lit) -> syn::Result<String> {
    match lit {
        syn::Lit::Str(s) => Ok(s.value()),
        bad => Err(syn::Error::new_spanned(bad, "string literal required here"))
    }
}