use super::attrs;

#[allow(clippy::large_enum_variant)]
pub enum Fields {
    Single(Field),
    Unnamed(Vec<Field>),
    Named(Vec<Field>),
    Unit,
}

pub struct Field {
    pub attr_props: attrs::Props,
    pub field: syn::Field,
}

impl Fields {
    pub fn from_syn(fields: syn::Fields) -> syn::Result<Fields> {
        match fields {
            syn::Fields::Unnamed(fields) => {
                let mut fields = process_fields(fields.unnamed, false)?;
                if fields.len() == 1 {
                    Ok(Fields::Single(fields.remove(0)))
                } else {
                    Ok(Fields::Unnamed(fields))
                }
            }
            syn::Fields::Named(fields) => Ok(Fields::Named(process_fields(fields.named, true)?)),
            syn::Fields::Unit => Ok(Fields::Unit),
        }
    }
}

fn process_fields(
    fields: impl IntoIterator<Item = syn::Field>,
    named: bool,
) -> syn::Result<Vec<Field>> {
    fields
        .into_iter()
        .map(|f| process_field(f, named))
        .collect()
}

fn process_field(field: syn::Field, named: bool) -> syn::Result<Field> {
    let attr_props = attrs::parse(&field.attrs)?;
    if attr_props.optional {
        if !named {
            return Err(syn::Error::new_spanned(
                &field,
                "#[api_body(optional)] can only be used on named fields",
            ));
        }
        if attr_props.flatten {
            return Err(syn::Error::new_spanned(
                &field,
                "the `optional` and `flatten` attrs cannot be used together",
            ));
        }
    }
    Ok(Field { attr_props, field })
}
