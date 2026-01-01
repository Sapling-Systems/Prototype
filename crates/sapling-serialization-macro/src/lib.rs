use darling::FromMeta;
use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
  Attribute, Data, DeriveInput, Ident, LitStr, PathArguments, Type, parse_macro_input,
  spanned::Spanned,
};

#[derive(Debug, Default, FromMeta)]
struct SaplingAttr {
  rename: Option<syn::LitStr>,
  indexed: Option<bool>,
}

fn sapling_attr(attrs: &[Attribute]) -> syn::Result<SaplingAttr> {
  let mut out = SaplingAttr::default();

  for attr in attrs {
    if !attr.path().is_ident("sapling") {
      continue;
    }
    let parsed = SaplingAttr::from_meta(&attr.meta)?;
    if parsed.rename.is_some() {
      out.rename = parsed.rename;
    }
    if parsed.indexed.is_some() {
      out.indexed = parsed.indexed;
    }
  }

  Ok(out)
}

fn get_property_static_ident(struct_name: &Ident, property_ident: &Ident) -> Ident {
  format_ident!("__LAZY_{}_{}_PROPERTY", struct_name, property_ident)
}

fn get_vect_inner_type(ty: &Type) -> Type {
  match ty {
    Type::Path(path) => {
      if let Some(segment) = path.path.segments.last() {
        if segment.ident == "Vec" {
          if let PathArguments::AngleBracketed(generic) = &segment.arguments {
            if let Some(syn::GenericArgument::Type(generic_ty)) = generic.args.first() {
              return generic_ty.clone();
            }
          }
        }
      }
      abort!(ty.span(), "Unsupported type for indexed field");
    }
    _ => abort!(ty.span(), "Unsupported type for indexed field"),
  }
}

fn parse_serialization_fact_fields(input: &DeriveInput) -> (TokenStream2, TokenStream2) {
  let Data::Struct(struc) = &input.data else {
    abort!(input.span(), "Only structs are supported as of now");
  };

  let mut fields = vec![];
  let mut global_fields = vec![];

  for field in struc.fields.iter() {
    let ident = field.ident.clone().unwrap();
    let mut name = LitStr::new(&ident.to_string(), ident.span());
    let real_name = field.ident.as_ref().unwrap().clone();
    let mut ty = field.ty.clone();
    let mut indexed = false;

    let attributes = sapling_attr(&field.attrs)
      .unwrap_or_else(|err| abort!(err.span(), "Failed to parse attributes"));

    if let Some(rename) = attributes.rename {
      name = rename;
    }
    if let Some(attr_indexed) = attributes.indexed {
      indexed = attr_indexed;
      ty = get_vect_inner_type(&ty);
    }

    let static_property = get_property_static_ident(&input.ident, field.ident.as_ref().unwrap());

    global_fields.push(quote! {
        #[doc(hidden)]
        #[allow(non_upper_case_globals)]
        static #static_property: std::sync::OnceLock<sapling_data_model::Subject> = std::sync::OnceLock::new();
    });

    let property_selector = if indexed {
      quote! {
          subject: sapling_data_model::Subject::Integer { value: index as i64 },
      }
    } else {
      quote! {
          subject: property_subject.clone(),
      }
    };

    let value_selector = if indexed {
      quote! {
          subject: <#ty as sapling_serialization::SaplingSerializable::<TSerializeContext>>::serialize_to_facts(&self.#real_name[index], context, stringify!(#real_name)),
      }
    } else {
      quote! {
          subject: <#ty as sapling_serialization::SaplingSerializable::<TSerializeContext>>::serialize_to_facts(&self.#real_name, context, stringify!(#real_name)),
      }
    };

    let fact_creation = quote! {
        let fact = Fact {
            subject: SubjectSelector {
                evaluated: false,
                subject: subject.clone(),
                property: None,
            },
            property: SubjectSelector {
                #property_selector
                evaluated: false,
                property: None,
            },
            operator: System::CORE_OPERATOR_IS.clone(),
            value: SubjectSelector {
                #value_selector
                evaluated: false,
                property: None,
            },
            meta: Subject::String { value: "default".into() },
        };
        context.add_fact(fact);
    };

    if indexed {
      fields.push(quote! {
            {
                let property_subject = #static_property.get_or_init(|| context.new_static_subject(#name));
                for index in 0..self.#real_name.len() {
                    #fact_creation
                }
            }
        });
    } else {
      fields.push(quote! {
            {
                let property_subject = #static_property.get_or_init(|| context.new_static_subject(#name));
                #fact_creation
            }
        });
    }
  }

  (
    quote! {
      #(#fields)*
    },
    quote! {
      #(#global_fields)*
    },
  )
}

#[proc_macro_error]
#[proc_macro_derive(SaplingSerialization, attributes(sapling))]
pub fn sapling_serialization_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);

  let (fields, global_fields) = parse_serialization_fact_fields(&input);

  let ident = input.ident;
  let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

  let expanded = quote! {
      #global_fields

      impl<TSerializeContext: sapling_serialization::SerializerContext, #ty_generics> sapling_serialization::SaplingSerializable<TSerializeContext> for #ident<#impl_generics> #where_clause {
          fn serialize_to_facts(&self, context: &mut TSerializeContext, name: &str) -> sapling_data_model::Subject {
              use sapling_data_model::*;
              use sapling_query_engine::System;

              let subject = context.new_static_subject(name);

              let crate_name: &str = env!("CARGO_PKG_NAME");
              let source_fact = Fact {
                  subject: SubjectSelector {
                      evaluated: false,
                      subject: subject.clone(),
                      property: None,
                  },
                  property: SubjectSelector {
                      evaluated: false,
                      subject: System::CORE_SERIALIZATION_SOURCE.clone(),
                      property: None,
                  },
                  operator: System::CORE_OPERATOR_IS.clone(),
                  value: SubjectSelector {
                      evaluated: false,
                      subject: Subject::String { value: format!("{}::{}::{}", crate_name, module_path!(), stringify!(#ident)) },
                      property: None,
                  },
                  meta: Subject::String { value: "default".into() },
              };
              context.add_fact(source_fact);

              #fields

              subject
          }
      }
  };

  // Hand the output tokens back to the compiler
  TokenStream::from(expanded)
}

fn parse_deserialization_fact_fields(input: &DeriveInput) -> (TokenStream2, TokenStream2) {
  let Data::Struct(struc) = &input.data else {
    abort!(input.span(), "Only structs are supported as of now");
  };

  let mut fields = vec![];
  let mut field_names = vec![];
  let mut queries = vec![];

  for field in struc.fields.iter() {
    let ident = field.ident.clone().unwrap();
    let mut name = LitStr::new(&ident.to_string(), ident.span());
    let mut indexed = false;

    let attributes = sapling_attr(&field.attrs)
      .unwrap_or_else(|err| abort!(err.span(), "Failed to parse attributes"));

    if let Some(rename) = attributes.rename {
      name = rename;
    }

    if let Some(attr_indexed) = attributes.indexed {
      indexed = attr_indexed;
    }

    let static_property = get_property_static_ident(&input.ident, field.ident.as_ref().unwrap());
    field_names.push(ident.clone());

    if indexed {
      queries.push(quote! {
          sapling_data_model::Query {
              subject: subject.clone(),
              evaluated: false,
              meta: None,
              property: Some(System::CORE_INTEGER_PROPERTY.clone()),
          }
      });
      fields.push(quote! {
          let #ident = {
            let mut result = std::vec::Vec::new();
            let mut index = 0;
            loop {
                let query = sapling_data_model::Query {
                    subject: subject.clone(),
                    evaluated: false,
                    meta: None,
                    property: Some(Subject::Integer { value: index }),
                };
                if let Ok(value) = sapling_serialization::__macro_query_deep(context, &query) {
                    result.push(value);
                    index += 1;
                } else {
                    break;
                }
            }
            result
          };
      });
    } else {
      queries.push(quote! {
          {
            let property_subject = #static_property.get_or_init(|| context.new_static_subject(#name));
            sapling_data_model::Query {
                subject: subject.clone(),
                evaluated: false,
                meta: None,
                property: Some(property_subject.clone()),
            }
          }
      });
      fields.push(quote! {
            let #ident = {
                let property_subject = #static_property.get_or_init(|| context.new_static_subject(#name));
                let query = sapling_data_model::Query {
                    subject: subject.clone(),
                    evaluated: false,
                    meta: None,
                    property: Some(property_subject.clone()),
                };
                sapling_serialization::__macro_query_deep(context, &query)?
            };
        });
    }
  }

  (
    quote! {
      #(#fields)*

      Ok(Self {
        #(#field_names,)*
      })
    },
    quote! {
        vec![#(#queries),*]
    },
  )
}

#[proc_macro_error]
#[proc_macro_derive(SaplingDeserialization, attributes(sapling))]
pub fn sapling_deserialization_derive(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);

  let (fields, queries) = parse_deserialization_fact_fields(&input);

  let ident = input.ident;

  let expanded = quote! {
      impl<T: sapling_serialization::DeserializerContext> sapling_serialization::SaplingDeserializable<T> for #ident {
        fn first_level_queries(subject: &sapling_data_model::Subject, context: &mut T) -> Vec<sapling_data_model::Query> {
            use sapling_query_engine::System;
            #queries
        }

        fn deserialize_subject(
          subject: &sapling_data_model::Subject,
          context: &mut T,
        ) -> Result<Self, sapling_serialization::DeserializeError> {
            use sapling_data_model::{Subject};
            use sapling_query_engine::System;
            use sapling_serialization::DeserializeError;

            #fields
        }

        fn deserialize_all(_context: &mut T) -> Vec<Result<Self, sapling_serialization::DeserializeError>> {
          todo!("not supported on integers")
        }
      }
  };

  // Hand the output tokens back to the compiler
  TokenStream::from(expanded)
}
