use super::attrs;

pub enum Fields {
    Single(Field),
    Unnamed(Vec<Field>),
    Named(Vec<Field>),
    Unit
}

pub struct Field {
    pub attr_props: attrs::Props,
    pub field: syn::Field
}

impl Fields {
    pub fn from_syn (fields: syn::Fields) -> syn::Result<Fields> {
        match fields {
            syn::Fields::Unnamed(fields) => {
                if fields.unnamed.len() == 1 {
                    let field = process_field(fields.unnamed[0].clone());
                    Ok(Fields::Single(field?))
                } else {
                    let fields = process_fields(fields.unnamed);
                    Ok(Fields::Unnamed(fields?))
                }
            },
            syn::Fields::Named(fields) => {
                let fields = process_fields(fields.named);
                Ok(Fields::Named(fields?))
            },
            syn::Fields::Unit => {
                Ok(Fields::Unit)
            }
        }
    }
}

fn process_fields (fields: impl IntoIterator<Item = syn::Field>) -> syn::Result<Vec<Field>> {
    fields.into_iter().map(process_field).collect()
}

fn process_field (field: syn::Field) -> syn::Result<Field> {
    match attrs::parse(&field.attrs) {
        Ok(attr_props) => {
            Ok(Field {
                attr_props,
                field
            })
        },
        Err(e) => {
            Err(e)
        }
    }
}