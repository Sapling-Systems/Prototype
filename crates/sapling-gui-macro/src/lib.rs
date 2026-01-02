use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use syn::{
  BinOp, Expr, ExprBinary, Token,
  parse::{Parse, ParseStream},
  parse_macro_input,
};

/// Get the crate name for sapling-gui, handling both internal and external usage
fn get_crate_name() -> proc_macro2::TokenStream {
  use proc_macro_crate::{FoundCrate, crate_name};

  match crate_name("sapling-gui") {
    Ok(FoundCrate::Name(name)) => {
      let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
      quote! { ::#ident }
    }
    Ok(FoundCrate::Itself) => quote! { crate },
    Err(_) => quote! { ::sapling_gui },
  }
}

struct ConstraintInput {
  expr: Expr,
  strength: Option<Expr>,
}

impl Parse for ConstraintInput {
  fn parse(input: ParseStream) -> syn::Result<Self> {
    let expr = input.parse()?;

    let strength = if input.peek(Token![,]) {
      input.parse::<Token![,]>()?;

      // Parse "strength"
      let ident: syn::Ident = input.parse()?;
      if ident != "strength" {
        return Err(syn::Error::new_spanned(
          ident,
          "Expected 'strength' after comma",
        ));
      }

      // Parse "="
      input.parse::<Token![=]>()?;

      // Parse the strength expression
      Some(input.parse()?)
    } else {
      None
    };

    Ok(ConstraintInput { expr, strength })
  }
}

/// Represents a linear term in the constraint equation
#[derive(Clone)]
enum Term {
  Constant(f32),
  RuntimeConstant(Expr),
  Variable { name: String, coefficient: f32 },
  VariableWithRuntimeCoeff { name: String, coefficient: Expr },
}

impl Term {
  fn negate(&self) -> Self {
    match self {
      Term::Constant(value) => Term::Constant(-value),
      Term::RuntimeConstant(expr) => Term::RuntimeConstant(syn::parse_quote! { -(#expr) }),
      Term::Variable { name, coefficient } => Term::Variable {
        name: name.clone(),
        coefficient: -coefficient,
      },
      Term::VariableWithRuntimeCoeff { name, coefficient } => Term::VariableWithRuntimeCoeff {
        name: name.clone(),
        coefficient: syn::parse_quote! { -(#coefficient) },
      },
    }
  }

  fn multiply(&self, scalar: f32) -> Self {
    match self {
      Term::Constant(value) => Term::Constant(value * scalar),
      Term::RuntimeConstant(expr) => Term::RuntimeConstant(syn::parse_quote! { (#expr) * #scalar }),
      Term::Variable { name, coefficient } => Term::Variable {
        name: name.clone(),
        coefficient: coefficient * scalar,
      },
      Term::VariableWithRuntimeCoeff { name, coefficient } => Term::VariableWithRuntimeCoeff {
        name: name.clone(),
        coefficient: syn::parse_quote! { (#coefficient) * #scalar },
      },
    }
  }

  fn multiply_runtime(&self, scalar_expr: &Expr) -> Self {
    match self {
      Term::Constant(value) => Term::RuntimeConstant(syn::parse_quote! { #value * (#scalar_expr) }),
      Term::RuntimeConstant(expr) => {
        Term::RuntimeConstant(syn::parse_quote! { (#expr) * (#scalar_expr) })
      }
      Term::Variable { name, coefficient } => Term::VariableWithRuntimeCoeff {
        name: name.clone(),
        coefficient: syn::parse_quote! { #coefficient * (#scalar_expr) },
      },
      Term::VariableWithRuntimeCoeff { name, coefficient } => Term::VariableWithRuntimeCoeff {
        name: name.clone(),
        coefficient: syn::parse_quote! { (#coefficient) * (#scalar_expr) },
      },
    }
  }

  fn divide(&self, scalar: f32) -> Self {
    match self {
      Term::Constant(value) => Term::Constant(value / scalar),
      Term::RuntimeConstant(expr) => Term::RuntimeConstant(syn::parse_quote! { (#expr) / #scalar }),
      Term::Variable { name, coefficient } => Term::Variable {
        name: name.clone(),
        coefficient: coefficient / scalar,
      },
      Term::VariableWithRuntimeCoeff { name, coefficient } => Term::VariableWithRuntimeCoeff {
        name: name.clone(),
        coefficient: syn::parse_quote! { (#coefficient) / #scalar },
      },
    }
  }

  fn divide_runtime(&self, scalar_expr: &Expr) -> Self {
    match self {
      Term::Constant(value) => Term::RuntimeConstant(syn::parse_quote! { #value / (#scalar_expr) }),
      Term::RuntimeConstant(expr) => {
        Term::RuntimeConstant(syn::parse_quote! { (#expr) / (#scalar_expr) })
      }
      Term::Variable { name, coefficient } => Term::VariableWithRuntimeCoeff {
        name: name.clone(),
        coefficient: syn::parse_quote! { #coefficient / (#scalar_expr) },
      },
      Term::VariableWithRuntimeCoeff { name, coefficient } => Term::VariableWithRuntimeCoeff {
        name: name.clone(),
        coefficient: syn::parse_quote! { (#coefficient) / (#scalar_expr) },
      },
    }
  }
}

/// Check if an identifier is a known constraint variable
fn is_constraint_variable(name: &str) -> bool {
  matches!(
    name,
    "parent_left"
      | "parent_right"
      | "parent_top"
      | "parent_bottom"
      | "self_left"
      | "self_right"
      | "self_top"
      | "self_bottom"
  )
}

/// Parse an expression into a list of terms
fn parse_expr_to_terms(expr: &Expr) -> Vec<Term> {
  match expr {
    // Binary operations
    Expr::Binary(ExprBinary {
      left, op, right, ..
    }) => {
      match op {
        BinOp::Add(_) => {
          let mut terms = parse_expr_to_terms(left);
          terms.extend(parse_expr_to_terms(right));
          terms
        }
        BinOp::Sub(_) => {
          let mut terms = parse_expr_to_terms(left);
          let right_terms = parse_expr_to_terms(right);
          terms.extend(right_terms.iter().map(|t| t.negate()));
          terms
        }
        BinOp::Mul(_) => {
          // Handle multiplication: one side must be a constant
          let left_terms = parse_expr_to_terms(left);
          let right_terms = parse_expr_to_terms(right);

          // Check if left is a compile-time constant
          if left_terms.len() == 1 {
            match &left_terms[0] {
              Term::Constant(scalar) => {
                return right_terms.iter().map(|t| t.multiply(*scalar)).collect();
              }
              Term::RuntimeConstant(_) => {
                return right_terms
                  .iter()
                  .map(|t| t.multiply_runtime(left))
                  .collect();
              }
              _ => {}
            }
          }

          // Check if right is a constant
          if right_terms.len() == 1 {
            match &right_terms[0] {
              Term::Constant(scalar) => {
                return left_terms.iter().map(|t| t.multiply(*scalar)).collect();
              }
              Term::RuntimeConstant(_) => {
                return left_terms
                  .iter()
                  .map(|t| t.multiply_runtime(right))
                  .collect();
              }
              _ => {}
            }
          }

          abort!(expr, "Multiplication must involve at least one constant");
        }
        BinOp::Div(_) => {
          // Handle division: right side must be a constant
          let left_terms = parse_expr_to_terms(left);
          let right_terms = parse_expr_to_terms(right);

          if right_terms.len() == 1 {
            match &right_terms[0] {
              Term::Constant(divisor) => {
                if *divisor == 0.0 {
                  abort!(expr, "Division by zero");
                }
                return left_terms.iter().map(|t| t.divide(*divisor)).collect();
              }
              Term::RuntimeConstant(_) => {
                return left_terms.iter().map(|t| t.divide_runtime(right)).collect();
              }
              _ => {}
            }
          }

          abort!(expr, "Division is only supported by constants");
        }
        _ => abort!(expr, "Unsupported binary operator in constraint expression"),
      }
    }

    // Unary minus
    Expr::Unary(expr_unary) => {
      if matches!(expr_unary.op, syn::UnOp::Neg(_)) {
        let terms = parse_expr_to_terms(&expr_unary.expr);
        terms.iter().map(|t| t.negate()).collect()
      } else {
        abort!(
          expr,
          "Only unary minus is supported in constraint expressions"
        );
      }
    }

    // Parenthesized expression
    Expr::Paren(expr_paren) => parse_expr_to_terms(&expr_paren.expr),

    // Literal (constant)
    Expr::Lit(expr_lit) => {
      if let syn::Lit::Float(lit_float) = &expr_lit.lit {
        vec![Term::Constant(lit_float.base10_parse::<f32>().unwrap())]
      } else if let syn::Lit::Int(lit_int) = &expr_lit.lit {
        vec![Term::Constant(lit_int.base10_parse::<f32>().unwrap())]
      } else {
        abort!(expr, "Only numeric literals (f32) are supported");
      }
    }

    // Path (variable name or runtime constant)
    Expr::Path(expr_path) => {
      if expr_path.path.segments.len() == 1 {
        let ident = &expr_path.path.segments[0].ident;
        let name = ident.to_string();

        // Check if this is a known constraint variable
        if is_constraint_variable(&name) {
          vec![Term::Variable {
            name,
            coefficient: 1.0,
          }]
        } else {
          // Treat as a runtime constant expression
          vec![Term::RuntimeConstant(expr.clone())]
        }
      } else {
        // Complex path - treat as runtime constant
        vec![Term::RuntimeConstant(expr.clone())]
      }
    }

    // Any other expression type - treat as runtime constant
    _ => vec![Term::RuntimeConstant(expr.clone())],
  }
}

/// Map variable shorthand names to ElementConstraintVariable variants
fn map_variable_name(
  name: &str,
  crate_name: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
  match name {
    "parent_left" => quote! { #crate_name::prelude::ElementConstraintVariable::ParentLeft },
    "parent_right" => quote! { #crate_name::prelude::ElementConstraintVariable::ParentRight },
    "parent_top" => quote! { #crate_name::prelude::ElementConstraintVariable::ParentTop },
    "parent_bottom" => quote! { #crate_name::prelude::ElementConstraintVariable::ParentBottom },
    "self_left" => quote! { #crate_name::prelude::ElementConstraintVariable::SelfLeft },
    "self_right" => quote! { #crate_name::prelude::ElementConstraintVariable::SelfRight },
    "self_top" => quote! { #crate_name::prelude::ElementConstraintVariable::SelfTop },
    "self_bottom" => quote! { #crate_name::prelude::ElementConstraintVariable::SelfBottom },
    _ => abort!(
      proc_macro2::Span::call_site(),
      "Unknown variable '{}'. Valid variables: parent_left, parent_right, parent_top, parent_bottom, self_left, self_right, self_top, self_bottom",
      name
    ),
  }
}

/// Generate a single ElementConstraint from the input
fn generate_element_constraint(
  input: &ConstraintInput,
  crate_name: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
  // Extract the comparison operator and both sides
  let (left_expr, operator, right_expr) = match &input.expr {
    Expr::Binary(ExprBinary {
      left, op, right, ..
    }) => match op {
      BinOp::Eq(_) => (
        left,
        quote! { #crate_name::prelude::ElementConstraintOperator::Equal },
        right,
      ),
      BinOp::Ge(_) => (
        left,
        quote! { #crate_name::prelude::ElementConstraintOperator::GreaterOrEqual },
        right,
      ),
      BinOp::Le(_) => (
        left,
        quote! { #crate_name::prelude::ElementConstraintOperator::LessOrEqual },
        right,
      ),
      _ => abort!(input.expr, "Constraint must use ==, >=, or <= operator"),
    },
    _ => abort!(
      input.expr,
      "Constraint must be a comparison expression (==, >=, or <=)"
    ),
  };

  // Parse both sides into terms
  let left_terms = parse_expr_to_terms(left_expr);
  let right_terms = parse_expr_to_terms(right_expr);

  // Move everything to the left side (left - right)
  let mut all_terms = left_terms;
  all_terms.extend(right_terms.iter().map(|t| t.negate()));

  // Separate into constant and variable terms
  let mut compile_time_constant = 0.0f32;
  let mut runtime_constant_exprs: Vec<Expr> = Vec::new();
  let mut var_terms: Vec<Term> = Vec::new();

  for term in all_terms {
    match term {
      Term::Constant(value) => {
        compile_time_constant += value;
      }
      Term::RuntimeConstant(expr) => {
        runtime_constant_exprs.push(expr);
      }
      Term::Variable { .. } | Term::VariableWithRuntimeCoeff { .. } => {
        var_terms.push(term);
      }
    }
  }

  // Generate code for the constant
  let constant_code = if runtime_constant_exprs.is_empty() {
    quote! { #compile_time_constant }
  } else {
    let runtime_sum = runtime_constant_exprs.iter().fold(
      quote! { #compile_time_constant },
      |acc, expr| quote! { #acc + (#expr) },
    );
    runtime_sum
  };

  // Generate code for each variable term
  let terms_code = var_terms.iter().map(|term| match term {
    Term::Variable { name, coefficient } => {
      let var_enum = map_variable_name(name, crate_name);
      quote! {
        #crate_name::prelude::ElementConstraintTerm {
          variable: #var_enum,
          coefficient: #coefficient,
        }
      }
    }
    Term::VariableWithRuntimeCoeff { name, coefficient } => {
      let var_enum = map_variable_name(name, crate_name);
      quote! {
        #crate_name::prelude::ElementConstraintTerm {
          variable: #var_enum,
          coefficient: #coefficient,
        }
      }
    }
    _ => unreachable!(),
  });

  // Determine the strength to use
  let strength_code = if let Some(strength_expr) = &input.strength {
    quote! { #strength_expr }
  } else {
    quote! { #crate_name::prelude::ElementConstraints::REQUIRED }
  };

  // Generate the ElementConstraint code
  quote! {
      #crate_name::prelude::ElementConstraint {
          operator: #operator,
          expression: #crate_name::prelude::ElementConstraintExpression {
              constant: #constant_code,
              terms: vec![
                  #(#terms_code),*
              ],
          },
          strength: #strength_code,
      }
  }
}

#[proc_macro]
#[proc_macro_error]
pub fn constraint(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ConstraintInput);
  let crate_name = get_crate_name();

  let constraint_code = generate_element_constraint(&input, &crate_name);

  // Wrap in ElementConstraints
  let expanded = quote! {
      #crate_name::prelude::ElementConstraints {
          constraints: vec![
              #constraint_code
          ],
      }
  };

  TokenStream::from(expanded)
}

#[proc_macro]
#[proc_macro_error]
pub fn constraint1(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ConstraintInput);
  let crate_name = get_crate_name();

  let constraint_code = generate_element_constraint(&input, &crate_name);

  TokenStream::from(constraint_code)
}
