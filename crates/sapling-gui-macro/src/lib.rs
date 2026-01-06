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
  RuntimeVariable { expr: Expr, coefficient: f32 },
  RuntimeVariableWithRuntimeCoeff { expr: Expr, coefficient: Expr },
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
      Term::RuntimeVariable { expr, coefficient } => Term::RuntimeVariable {
        expr: expr.clone(),
        coefficient: -coefficient,
      },
      Term::RuntimeVariableWithRuntimeCoeff { expr, coefficient } => {
        Term::RuntimeVariableWithRuntimeCoeff {
          expr: expr.clone(),
          coefficient: syn::parse_quote! { -(#coefficient) },
        }
      }
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
      Term::RuntimeVariable { expr, coefficient } => Term::RuntimeVariable {
        expr: expr.clone(),
        coefficient: coefficient * scalar,
      },
      Term::RuntimeVariableWithRuntimeCoeff { expr, coefficient } => {
        Term::RuntimeVariableWithRuntimeCoeff {
          expr: expr.clone(),
          coefficient: syn::parse_quote! { (#coefficient) * #scalar },
        }
      }
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
      Term::RuntimeVariable { expr, coefficient } => Term::RuntimeVariableWithRuntimeCoeff {
        expr: expr.clone(),
        coefficient: syn::parse_quote! { #coefficient * (#scalar_expr) },
      },
      Term::RuntimeVariableWithRuntimeCoeff { expr, coefficient } => {
        Term::RuntimeVariableWithRuntimeCoeff {
          expr: expr.clone(),
          coefficient: syn::parse_quote! { (#coefficient) * (#scalar_expr) },
        }
      }
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
      Term::RuntimeVariable { expr, coefficient } => Term::RuntimeVariable {
        expr: expr.clone(),
        coefficient: coefficient / scalar,
      },
      Term::RuntimeVariableWithRuntimeCoeff { expr, coefficient } => {
        Term::RuntimeVariableWithRuntimeCoeff {
          expr: expr.clone(),
          coefficient: syn::parse_quote! { (#coefficient) / #scalar },
        }
      }
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
      Term::RuntimeVariable { expr, coefficient } => Term::RuntimeVariableWithRuntimeCoeff {
        expr: expr.clone(),
        coefficient: syn::parse_quote! { #coefficient / (#scalar_expr) },
      },
      Term::RuntimeVariableWithRuntimeCoeff { expr, coefficient } => {
        Term::RuntimeVariableWithRuntimeCoeff {
          expr: expr.clone(),
          coefficient: syn::parse_quote! { (#coefficient) / (#scalar_expr) },
        }
      }
    }
  }
}

/// Check if an identifier is a known constraint variable
fn is_constraint_variable(name: &str) -> bool {
  matches!(
    name,
    "parent_x"
      | "parent_y"
      | "parent_width"
      | "parent_height"
      | "self_x"
      | "self_y"
      | "self_width"
      | "self_height"
      | "screen_width"
      | "screen_height"
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
          // Handle multiplication: one side must be a constant or both are runtime values
          let left_terms = parse_expr_to_terms(left);
          let right_terms = parse_expr_to_terms(right);

          // Check if left is a compile-time constant or special term
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
              Term::Variable { .. } | Term::VariableWithRuntimeCoeff { .. } => {
                // Constraint variable on left - check if right is a runtime value
                if right_terms.len() == 1 && matches!(right_terms[0], Term::RuntimeVariable { .. })
                {
                  return left_terms
                    .iter()
                    .map(|t| t.multiply_runtime(right))
                    .collect();
                }
              }
              Term::RuntimeVariable { .. } => {
                // If right is a constant or constraint variable, multiply the runtime variable by it
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
                    Term::RuntimeVariable { .. } => {
                      // Both are runtime values, treat whole expression as runtime constant
                      return vec![Term::RuntimeConstant(expr.clone())];
                    }
                    _ => {}
                  }
                }
              }
              _ => {}
            }
          }

          // Check if right is a constant or special term
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
              Term::Variable { .. } | Term::VariableWithRuntimeCoeff { .. } => {
                // Constraint variable on right - check if left is a runtime value
                if left_terms.len() == 1 && matches!(left_terms[0], Term::RuntimeVariable { .. }) {
                  return right_terms
                    .iter()
                    .map(|t| t.multiply_runtime(left))
                    .collect();
                }
              }
              Term::RuntimeVariable { .. } => {
                // If left is a constant or constraint variable, multiply the runtime variable by it
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
                    Term::RuntimeVariable { .. } => {
                      // Both are runtime values, treat whole expression as runtime constant
                      return vec![Term::RuntimeConstant(expr.clone())];
                    }
                    _ => {}
                  }
                }
              }
              _ => {}
            }
          }

          // If both sides are runtime values (constants or runtime variables), treat as runtime constant
          let left_has_constraint_vars = left_terms.iter().any(|t| {
            matches!(
              t,
              Term::Variable { .. } | Term::VariableWithRuntimeCoeff { .. }
            )
          });
          let right_has_constraint_vars = right_terms.iter().any(|t| {
            matches!(
              t,
              Term::Variable { .. } | Term::VariableWithRuntimeCoeff { .. }
            )
          });

          if !left_has_constraint_vars && !right_has_constraint_vars {
            return vec![Term::RuntimeConstant(expr.clone())];
          }

          abort!(
            expr,
            "Multiplication must involve at least one constant or both operands must be non-constraint values"
          );
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

    // Path (variable name or runtime constant/variable)
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
          // Treat as a runtime variable (could be constant or variable at runtime)
          vec![Term::RuntimeVariable {
            expr: expr.clone(),
            coefficient: 1.0,
          }]
        }
      } else {
        // Complex path - treat as runtime variable
        vec![Term::RuntimeVariable {
          expr: expr.clone(),
          coefficient: 1.0,
        }]
      }
    }

    // Method call or any other expression - treat as runtime variable
    _ => vec![Term::RuntimeVariable {
      expr: expr.clone(),
      coefficient: 1.0,
    }],
  }
}

/// Map variable shorthand names to ElementConstraintVariable variants
fn map_variable_name(
  name: &str,
  crate_name: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
  match name {
    "screen_width" => quote! { #crate_name::prelude::ElementConstraintVariable::ScreenWidth },
    "screen_height" => quote! { #crate_name::prelude::ElementConstraintVariable::ScreenHeight },
    "parent_x" => quote! { #crate_name::prelude::ElementConstraintVariable::ParentX },
    "parent_y" => quote! { #crate_name::prelude::ElementConstraintVariable::ParentY },
    "parent_width" => quote! { #crate_name::prelude::ElementConstraintVariable::ParentWidth },
    "parent_height" => quote! { #crate_name::prelude::ElementConstraintVariable::ParentHeight },
    "self_x" => quote! { #crate_name::prelude::ElementConstraintVariable::SelfX },
    "self_y" => quote! { #crate_name::prelude::ElementConstraintVariable::SelfY },
    "self_width" => quote! { #crate_name::prelude::ElementConstraintVariable::SelfWidth },
    "self_height" => quote! { #crate_name::prelude::ElementConstraintVariable::SelfHeight },
    _ => abort!(
      proc_macro2::Span::call_site(),
      "Unknown variable '{}'. Valid variables: parent_x, parent_y, parent_width, parent_height, self_x, self_y, self_width, self_height",
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
  let mut runtime_var_terms: Vec<Term> = Vec::new();

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
      Term::RuntimeVariable { .. } | Term::RuntimeVariableWithRuntimeCoeff { .. } => {
        runtime_var_terms.push(term);
      }
    }
  }

  // Determine the strength to use
  let strength_code = if let Some(strength_expr) = &input.strength {
    quote! { #strength_expr }
  } else {
    quote! { #crate_name::prelude::ElementConstraints::REQUIRED }
  };

  // If there are no runtime variables, we can generate a simple static constraint
  if runtime_var_terms.is_empty() {
    // Generate code for the constant
    let constant_code = if runtime_constant_exprs.is_empty() {
      quote! { #compile_time_constant }
    } else {
      runtime_constant_exprs.iter().fold(
        quote! { #compile_time_constant },
        |acc, expr| quote! { #acc + (#expr) },
      )
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
  } else {
    // We have runtime variables, so we need to build the constraint dynamically

    // Start with compile-time constant
    let mut constant_init = quote! { #compile_time_constant };

    // Add runtime constant expressions
    for expr in &runtime_constant_exprs {
      constant_init = quote! { #constant_init + (#expr) };
    }

    // Generate code for static variable terms
    let static_terms_code = var_terms.iter().map(|term| match term {
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

    // Generate code to evaluate runtime variables
    let runtime_var_eval_code = runtime_var_terms.iter().map(|term| match term {
      Term::RuntimeVariable { expr, coefficient } => {
        quote! {
          {
            use #crate_name::prelude::IntoConstraintTerm;
            match (#expr).into_constraint_term() {
              #crate_name::prelude::ConstraintTermValue::Constant(c) => {
                __constraint_constant += c * #coefficient;
              }
              #crate_name::prelude::ConstraintTermValue::Variable(v) => {
                __constraint_terms.push(#crate_name::prelude::ElementConstraintTerm {
                  variable: v,
                  coefficient: #coefficient,
                });
              }
            }
          }
        }
      }
      Term::RuntimeVariableWithRuntimeCoeff { expr, coefficient } => {
        quote! {
          {
            use #crate_name::prelude::IntoConstraintTerm;
            let __coeff = #coefficient;
            match (#expr).into_constraint_term() {
              #crate_name::prelude::ConstraintTermValue::Constant(c) => {
                __constraint_constant += c * __coeff;
              }
              #crate_name::prelude::ConstraintTermValue::Variable(v) => {
                __constraint_terms.push(#crate_name::prelude::ElementConstraintTerm {
                  variable: v,
                  coefficient: __coeff,
                });
              }
            }
          }
        }
      }
      _ => unreachable!(),
    });

    // Generate the ElementConstraint code with runtime evaluation
    quote! {
        {
          let mut __constraint_constant: f32 = #constant_init;
          let mut __constraint_terms = vec![
              #(#static_terms_code),*
          ];

          #(#runtime_var_eval_code)*

          #crate_name::prelude::ElementConstraint {
              operator: #operator,
              expression: #crate_name::prelude::ElementConstraintExpression {
                  constant: __constraint_constant,
                  terms: __constraint_terms,
              },
              strength: #strength_code,
          }
        }
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
