use crate::parser_rule_context_ext::ParserRuleContextExt;
use crate::prelude::generated::yarnspinnerlexer;
use crate::prelude::generated::yarnspinnerparser::*;
use crate::prelude::generated::yarnspinnerparservisitor::YarnSpinnerParserVisitorCompat;
use crate::prelude::*;
use crate::visitors::token_to_operator;
use antlr_rust::interval_set::Interval;
use antlr_rust::parser_rule_context::ParserRuleContext;
use antlr_rust::token::Token;
use antlr_rust::tree::{ParseTree, ParseTreeVisitorCompat};
use better_any::TidExt;
use rusty_yarn_spinner_core::prelude::convertible::Convertible;
use rusty_yarn_spinner_core::prelude::Operator;
use rusty_yarn_spinner_core::types::{FunctionType, SubTypeOf, Type, TypeOptionFormat};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::rc::Rc;

/// A visitor that walks the parse tree, checking for type consistency
/// in expressions. Existing type information is provided via the
/// [`existing_declarations`] property. This visitor will also
/// attempt to infer the type of variables that don't have an explicit
/// declaration; for each of these, a new Declaration will be created
/// and made available via the [`new_declaration`] property.
pub(crate) struct TypeCheckVisitor<'a, 'input: 'a> {
    /// <summary>
    /// Gets the collection of all declarations - both the ones we received
    /// at the start, and the new ones we've derived ourselves.
    /// </summary>
    pub(crate) diagnostics: Vec<Diagnostic>,

    /// Gets the collection of new variable declarations that were
    /// found as a result of using this  [`TypeCheckVisitor`] to visit a [`ParserRuleContext`].
    pub(crate) new_declarations: Vec<Declaration>,

    // the list of variables we aren't actually sure about
    pub(crate) deferred_types: Vec<DeferredTypeDiagnostic>,

    // The collection of variable declarations we know about before
    // starting our work
    existing_declarations: Vec<Declaration>,

    // The name of the node that we're currently visiting.
    current_node_name: Option<String>,

    // The name of the file that we're currently in.
    source_file_name: String,

    /// The type that this expression has been
    /// determined to be by a [`TypeCheckVisitor`]
    /// object.
    ///
    /// ## Implementation notes
    ///
    /// In the original implementation, this was implemented
    /// on the [`ValueContext`] directly using a `partial`.
    ///
    /// Careful, the original class has an unrelated member called `types`,
    /// but in this implementation, we replaced that member by [`Type::EXPLICITLY_CONSTRUCTABLE`].
    types: HashMap<HashableInterval, Type>,

    /// A type hint for the expression.
    /// This is mostly used by [`TypeCheckVisitor`]
    /// to give a hint that can be used by functions to
    /// influence their type when set to use inference.
    /// Won't be used if a concrete type is already known.
    ///
    /// ## Implementation notes
    ///
    /// In the original implementation, this was implemented
    /// on the [`ValueContext`] directly using a `partial`
    hints: HashMap<HashableInterval, Type>,

    tokens: &'a ActualTokenStream<'input>,
    _dummy: Option<Type>,
}

impl<'a, 'input: 'a> TypeCheckVisitor<'a, 'input> {
    pub(crate) fn new(
        source_file_name: String,
        existing_declarations: Vec<Declaration>,
        tokens: &'a ActualTokenStream<'input>,
    ) -> Self {
        Self {
            existing_declarations,
            source_file_name,
            tokens,
            diagnostics: Default::default(),
            new_declarations: Default::default(),
            deferred_types: Default::default(),
            current_node_name: Default::default(),
            types: Default::default(),
            hints: Default::default(),
            _dummy: Default::default(),
        }
    }

    /// Gets the collection of all declarations - both the ones we received
    /// at the start, and the new ones we've derived ourselves.
    pub(crate) fn declarations(&self) -> Vec<Declaration> {
        self.existing_declarations
            .iter()
            .cloned()
            .chain(self.new_declarations.iter().cloned())
            .collect()
    }

    fn get_hint(&self, ctx: &impl ParserRuleContext<'input>) -> Option<&Type> {
        let hashable_interval = get_hashable_interval(ctx);
        self.hints.get(&hashable_interval)
    }

    fn set_hint(
        &mut self,
        ctx: &impl ParserRuleContext<'input>,
        hint: impl Into<Option<Type>>,
    ) -> Option<Type> {
        let hint = hint.into()?;
        let hashable_interval = get_hashable_interval(ctx);
        self.hints.insert(hashable_interval, hint)
    }

    fn get_type(&self, ctx: &impl ParserRuleContext<'input>) -> Option<&Type> {
        let hashable_interval = get_hashable_interval(ctx);
        self.types.get(&hashable_interval)
    }

    fn set_type(
        &mut self,
        ctx: &impl ParserRuleContext<'input>,
        r#type: impl Into<Option<Type>>,
    ) -> Option<Type> {
        let r#type = r#type.into()?;
        let hashable_interval = get_hashable_interval(ctx);
        self.types.insert(hashable_interval, r#type)
    }
}

impl<'a, 'input: 'a> ParseTreeVisitorCompat<'input> for TypeCheckVisitor<'a, 'input> {
    type Node = YarnSpinnerParserContextType;

    type Return = Option<Type>;

    fn temp_result(&mut self) -> &mut Self::Return {
        &mut self._dummy
    }
}

impl<'a, 'input: 'a> YarnSpinnerParserVisitorCompat<'input> for TypeCheckVisitor<'a, 'input> {
    fn visit_node(&mut self, ctx: &NodeContext<'input>) -> Self::Return {
        for header in ctx.header_all() {
            let key = header.header_key.as_ref().unwrap().get_text();
            if key == "title" {
                let value = header.header_value.as_ref().unwrap().get_text();
                self.current_node_name = Some(value.to_owned());
            }
        }
        if let Some(body) = ctx.body() {
            self.visit(&*body);
        }
        None
    }

    fn visit_valueNull(&mut self, ctx: &ValueNullContext<'input>) -> Self::Return {
        self.diagnostics.push(
            Diagnostic::from_message("Null is not a permitted type in Yarn Spinner 2.0 and later")
                .with_file_name(&self.source_file_name)
                .read_parser_rule_context(ctx, self.tokens),
        );

        None
    }

    fn visit_valueString(&mut self, _ctx: &ValueStringContext<'input>) -> Self::Return {
        Some(Type::String)
    }

    fn visit_valueTrue(&mut self, _ctx: &ValueTrueContext<'input>) -> Self::Return {
        Some(Type::Boolean)
    }

    fn visit_valueFalse(&mut self, _ctx: &ValueFalseContext<'input>) -> Self::Return {
        Some(Type::Boolean)
    }

    fn visit_valueNumber(&mut self, _ctx: &ValueNumberContext<'input>) -> Self::Return {
        Some(Type::Number)
    }

    fn visit_valueVar(&mut self, ctx: &ValueVarContext<'input>) -> Self::Return {
        let variable = ctx.variable().unwrap();
        self.visit_variable(&*variable)
    }

    fn visit_variable(&mut self, ctx: &VariableContext<'input>) -> Self::Return {
        // The type of the value depends on the declared type of the
        // variable
        let Some(var_id) = ctx.get_token(yarnspinnerlexer::VAR_ID, 0) else {
                // We don't have a variable name for this Variable context.
                // The parser will have generated an error for us in an
                // earlier stage; here, we'll bail out.
            return None
        };
        let name = var_id.get_text();
        if let Some(declaration) = self
            .declarations()
            .into_iter()
            .find(|decl| decl.name == name)
        {
            return declaration.r#type;
        }

        // do we already have a potential warning about this?
        // no need to make more
        if self
            .deferred_types
            .iter()
            .any(|deferred_type| deferred_type.name == name)
        {
            return None;
        }

        // creating a new diagnostic for us having an undefined variable
        // this won't get added into the existing diags though because its possible a later pass will clear it up
        // so we save this as a potential diagnostic for the compiler itself to resolve
        let diagnostic =
            Diagnostic::from_message(format_cannot_determine_variable_type_error(&name))
                .with_file_name(&self.source_file_name)
                .read_parser_rule_context(ctx, self.tokens);
        self.deferred_types
            .push(DeferredTypeDiagnostic { name, diagnostic });

        // We don't have a declaration for this variable. Return
        // Undefined. Hopefully, other context will allow us to infer a
        // type.
        None
    }

    fn visit_valueFunc(&mut self, ctx: &ValueFuncContext<'input>) -> Self::Return {
        let function_name = ctx
            .function_call()
            .unwrap()
            .get_token(yarnspinnerlexer::FUNC_ID, 0)
            .unwrap()
            .get_text();
        let function_declaration = self
            .declarations()
            .into_iter()
            .find(|decl| decl.name == function_name);
        let hint = self.get_hint(ctx).cloned();
        let function_type = if let Some(function_declaration) = function_declaration {
            let Some(Type::Function(mut function_type)) = function_declaration.r#type.clone() else {
                 unreachable!("Internal error: function declaration is not of type Function. This is a bug. Please report it at https://github.com/Mafii/rusty-yarn-spinner/issues/new")
            };

            // we have an existing function but its undefined
            // if we also have a type hint we can use that to update it
            if function_type.return_type.is_none() {
                if let Some(hint) = hint {
                    self.new_declarations.find_remove(&function_declaration);
                    function_type.set_return_type(hint);
                    let new_declaration = Declaration {
                        r#type: Some(Type::from(function_type.clone())),
                        ..function_declaration
                    };
                    self.new_declarations.push(new_declaration);
                }
            }
            function_type
        } else {
            // We don't have a declaration for this function. Create an
            // implicit one.
            let mut function_type = FunctionType::default();
            // because it is an implicit declaration we will use the type hint to give us a return type
            function_type.set_return_type(hint);
            let line = ctx.start().get_line();
            let column = ctx.start().get_column();
            let function_declaration = Declaration::default()
                .with_type(Type::from(function_type.clone()))
                .with_name(&function_name)
                .with_description(format!(
                    "Implicit declaration of function at {}:{}:{}",
                    self.source_file_name, line, column
                ))
                // All positions are +1 compared to original implementation, but the result is the same.
                // I suspect the C# ANTLR implementation is 1-based while antlr4rust is 0-based.
                .with_range(
                    Position {
                        line: line as usize,
                        character: column as usize + 1,
                    }..=Position {
                        line: line as usize,
                        character: column as usize + 1 + ctx.stop().get_text().len(),
                    },
                )
                .with_implicit();

            // Create the array of parameters for this function based
            // on how many we've seen in this call. Set them all to be
            // undefined; we'll bind their type shortly.
            let expressions = ctx.function_call().unwrap().expression_all();
            let parameter_types = expressions.iter().map(|_| None);
            for parameter_type in parameter_types {
                function_type.add_parameter(parameter_type);
            }
            self.new_declarations.push(function_declaration);
            function_type
        };
        // Check each parameter of the function
        let supplied_parameters = ctx.function_call().unwrap().expression_all();
        let expected_parameter_types = function_type.parameters;

        if supplied_parameters.len() != expected_parameter_types.len() {
            // Wrong number of parameters supplied
            let parameters = if expected_parameter_types.len() == 1 {
                "parameter"
            } else {
                "parameters"
            };
            let diagnostic = Diagnostic::from_message(format!(
                "Function {} expects {} {}, but received {}",
                function_name,
                expected_parameter_types.len(),
                parameters,
                supplied_parameters.len()
            ))
            .with_file_name(&self.source_file_name)
            .read_parser_rule_context(ctx, self.tokens);
            self.diagnostics.push(diagnostic);
            return *function_type.return_type;
        }

        for (i, (supplied_parameter, mut expected_type)) in supplied_parameters
            .iter()
            .cloned()
            .zip(expected_parameter_types.iter())
            .enumerate()
        {
            let supplied_type = self.visit(&*supplied_parameter);
            if expected_type.is_none() {
                // The type of this parameter hasn't yet been bound.
                // Bind this parameter type to what we've resolved the
                // type to.
                expected_type = &supplied_type;
            }
            if !expected_type.is_sub_type_of(&supplied_type) {
                let diagnostic = Diagnostic::from_message(format!(
                    "{} parameter {} expects a {}, not a {}",
                    function_name,
                    i + 1,
                    expected_type.format(),
                    supplied_type.format()
                ))
                .with_file_name(&self.source_file_name)
                .read_parser_rule_context(ctx, self.tokens);
                self.diagnostics.push(diagnostic);
            }
        }
        // Cool, all the parameters check out!

        // Finally, return the return type of this function.
        *function_type.return_type
    }

    fn visit_expValue(&mut self, ctx: &ExpValueContext<'input>) -> Self::Return {
        // passing the hint from the expression down into the values within
        let hint = self.get_hint(ctx).cloned();
        let value = ctx.value().unwrap();
        self.set_hint(&*value, hint);
        // Value expressions have the type of their inner value
        let r#type = self.visit(&*value);
        self.set_type(ctx, r#type.clone());
        r#type
    }

    fn visit_expParens(&mut self, ctx: &ExpParensContext<'input>) -> Self::Return {
        // Parens expressions have the type of their inner expression
        let r#type = self.visit(&*ctx.expression().unwrap());
        self.set_type(ctx, r#type.clone());
        r#type
    }

    fn visit_expAndOrXor(&mut self, ctx: &ExpAndOrXorContext<'input>) -> Self::Return {
        let expressions: Vec<_> = ctx.expression_all().into_iter().map(Term::from).collect();
        let operator_context = ctx.op.as_ref().unwrap();
        let operator: Operator = token_to_operator(operator_context.token_type).unwrap();
        let description = operator_context.get_text().to_owned();
        let r#type = self.check_operation(ctx, expressions, operator, description, vec![]);
        self.set_type(ctx, r#type.clone());
        r#type
    }

    fn visit_set_statement(&mut self, ctx: &Set_statementContext<'input>) -> Self::Return {
        todo!()
    }

    fn visit_if_clause(&mut self, ctx: &If_clauseContext<'input>) -> Self::Return {
        todo!()
    }

    fn visit_else_if_clause(&mut self, ctx: &Else_if_clauseContext<'input>) -> Self::Return {
        todo!()
    }

    fn visit_expAddSub(&mut self, ctx: &ExpAddSubContext<'input>) -> Self::Return {
        todo!()
    }

    fn visit_expMultDivMod(&mut self, ctx: &ExpMultDivModContext<'input>) -> Self::Return {
        todo!()
    }

    fn visit_expComparison(&mut self, ctx: &ExpComparisonContext<'input>) -> Self::Return {
        todo!()
    }

    fn visit_expEquality(&mut self, ctx: &ExpEqualityContext<'input>) -> Self::Return {
        todo!()
    }

    fn visit_expNegative(&mut self, ctx: &ExpNegativeContext<'input>) -> Self::Return {
        todo!()
    }

    fn visit_expNot(&mut self, ctx: &ExpNotContext<'input>) -> Self::Return {
        todo!()
    }

    fn visit_jumpToExpression(&mut self, ctx: &JumpToExpressionContext<'input>) -> Self::Return {
        todo!()
    }
}

impl<'a, 'input: 'a> TypeCheckVisitor<'a, 'input> {
    /// ok so what do we actually need to do in here?
    /// we need to do a few different things
    /// basically we need to go through the various types in the expression
    /// if any are known we need to basically log that
    /// then at the end if there are still unknowns we check if the operation itself forces a type
    /// so if we have say Undefined = Undefined + Number then we know that only one operation supports + Number and that is Number + Number
    /// so we can slot the type into the various parts
    fn check_operation(
        &mut self,
        context: &impl ParserRuleContext<'input>,
        terms: Vec<Term<'input>>,
        operation_type: impl Into<Option<Operator>>,
        operation_description: String,
        permitted_types: Vec<Type>,
    ) -> Option<Type> {
        let operation_type = operation_type.into();
        let mut term_types = Vec::new();
        let mut expression_type = None;
        for expression in &terms {
            // Visit this expression, and determine its type.
            let r#type = self.visit(&**expression);
            if let Some(r#type) = r#type.clone() {
                if expression_type.is_none() {
                    // This is the first concrete type we've seen. This
                    // will be our expression type.
                    expression_type = Some(r#type.clone());
                }
                term_types.push(r#type);
            }
        }
        if permitted_types.len() == 1 && expression_type.is_none() {
            // If we aren't sure of the expression type from
            // parameters, but we only have one permitted one, then
            // assume that the expression type is the single permitted
            // type.

            // Guaranteed to be `Some`
            expression_type = permitted_types.first().cloned();
        }

        if expression_type.is_none() {
            // We still don't know what type of expression this is, and
            // don't have a reasonable guess.

            // Last-ditch effort: is the operator that we were given
            // valid in exactly one type? In that case, we'll decide
            // it's that type.
            if let Some(operation_type) = operation_type {
                let operation_type_name = operation_type.to_string();
                let types_implementing_method: Vec<_> = Type::EXPLICITLY_CONSTRUCTABLE
                    .iter()
                    .filter(|t| t.properties().methods.contains_key(&operation_type_name))
                    .collect();
                if types_implementing_method.len() == 1 {
                    // Only one type implements the operation we were
                    // given. Given no other information, we will assume
                    // that it is this type.

                    // Guaranteed to be `Some`
                    expression_type = types_implementing_method.first().cloned().cloned();
                } else if types_implementing_method.len() > 1 {
                    // Multiple types implement this operation.
                    let type_names = types_implementing_method
                        .iter()
                        .map(|t| t.properties().name)
                        .collect::<Vec<_>>()
                        .join(", or ");
                    let message = format!(
                        "Type of expression \"{}\" can't be determined without more context (the compiler thinks it could be {type_names}). Use a type cast on at least one of the terms (e.g. the string(), number(), bool() functions)",
                        context.get_text_with_whitespace(self.tokens),
                    );
                    let diagnostic = Diagnostic::from_message(message)
                        .with_file_name(&self.source_file_name)
                        .read_parser_rule_context(context, self.tokens);
                    self.diagnostics.push(diagnostic);
                    return None;
                } else {
                    // No types implement this operation (??) [sic]
                    let message = format!(
                        "Type of expression \"{}\" can't be determined without more context. Use a type cast on at least one of the terms (e.g. the string(), number(), bool() functions)",
                        context.get_text_with_whitespace(self.tokens),
                    );
                    let diagnostic = Diagnostic::from_message(message)
                        .with_file_name(&self.source_file_name)
                        .read_parser_rule_context(context, self.tokens);
                    self.diagnostics.push(diagnostic);
                    return None;
                }
            }
        }

        // to reach this point we have either worked out the final type of the expression
        // or had to give up, and if we gave up we have nothing left to do
        // there are then two parts to this, first we need to declare the implicit type of any variables (that appears to be working)
        // or the implicit type of any function.
        // annoyingly the function will already have an implicit definition created for it
        // we will have to strip that out and add in a new one with the new return type
        for term in &terms {
            let Term::Expression(expression) = term else { continue; };
            let ExpressionContextAll::ExpValueContext(value_context) = expression.as_ref() else { continue; };
            let Some(value) = value_context.value() else { continue; };
            let ValueContextAll::ValueFuncContext(func_context) = value.as_ref() else { continue; };

            let id = func_context
                .function_call()
                .unwrap()
                .FUNC_ID()
                .unwrap()
                .get_text();

            let function_type = self
                .new_declarations
                .iter_mut()
                .filter(|decl| decl.name == id)
                .find_map(|decl| {
                    if let Some(Type::Function(ref mut func)) = decl.r#type {
                        Some(func)
                    } else {
                        None
                    }
                });
            if let Some(func) = function_type {
                if func.return_type.is_some() {
                    continue;
                }
                func.return_type = Box::new(expression_type.clone());
            } else {
                self.visit(&**term);
            }
        }
        // Were any of the terms variables for which we don't currently
        // have a declaration for?

        // Start by building a list of all terms that are variables.
        // These are either variable values, or variable names . (The
        // difference between these two is that a ValueVarContext
        // occurs in syntax where the value of the variable is used
        // (like an expression), while a VariableContext occurs in
        // syntax where it's just a variable name (like a set
        // statements)

        // All VariableContexts in the terms of this expression (but
        // not in the children of those terms)
        let variable_contexts = terms
            .iter()
            .filter_map(|term| {
                term.child_of_type_unsized::<ValueContextAll>(0)
                    .and_then(|value_context| {
                        if let ValueContextAll::ValueVarContext(context) = value_context.as_ref() {
                            context.variable()
                        } else {
                            None
                        }
                    })
            })
            .chain(
                terms
                    .iter()
                    .find_map(|term| term.child_of_type_unsized::<VariableContext>(0)),
            )
            .chain(
                terms.iter().filter_map(|term| {
                    term.generic_context().downcast_rc::<VariableContext>().ok()
                }),
            )
            .chain(
                terms
                    .iter()
                    .filter_map(|term| term.generic_context().downcast_rc::<ValueContextAll>().ok())
                    .filter_map(|value_context| {
                        if let ValueContextAll::ValueVarContext(context) = value_context.as_ref() {
                            context.variable()
                        } else {
                            None
                        }
                    }),
            );

        // Build the list of variable contexts that we don't have a
        // declaration for. We'll check for explicit declarations first.
        let mut undefined_variable_contexts: Vec<_> = variable_contexts
            .filter(|v| {
                !self
                    .declarations()
                    .iter()
                    .any(|d| d.name == v.VAR_ID().unwrap().get_text())
            })
            .collect();
        // Implementation note: The original compares by reference here. The interval should be unique for each context, so let's use that instead.
        undefined_variable_contexts.sort_by_key(|v| get_hashable_interval(&**v));
        undefined_variable_contexts.dedup_by_key(|v| get_hashable_interval(&**v));

        for undefined_variable_context in undefined_variable_contexts {
            // We have references to variables that we don't have a an
            // explicit declaration for! Time to create implicit
            // references for them!

            let var_name = undefined_variable_context.VAR_ID().unwrap().get_text();
            // We can only create an implicit declaration for a variable
            // if we have a default value for it, because all variables
            // are required to have a value. If we can't, it's generally
            // because we couldn't figure out a concrete type for the
            // variable given the context.
            if let Some(default_value) = default_value_for_type(&expression_type) {
                let file_name = filename(&self.source_file_name);
                let node = self
                    .current_node_name
                    .as_ref()
                    .map(|name| format!(", node {name}"))
                    .unwrap_or_default();
                let decl = Declaration::default()
                    .with_name(&var_name)
                    .with_description(format!("Implicitly declared in {file_name}{node}"))
                    .with_type(expression_type.clone())
                    .with_default_value(default_value)
                    .with_source_file_name(self.source_file_name.clone())
                    .with_source_node_name_optional(self.current_node_name.clone())
                    .with_range(
                        Position {
                            line: undefined_variable_context.start().line as usize - 1,
                            character: undefined_variable_context.start().column as usize,
                        }..=Position {
                            line: undefined_variable_context.stop().line as usize - 1,
                            character: undefined_variable_context.stop().column as usize
                                // Implementation note: The original called `.stop()` here before the `get_text`,
                                //but I suspect that is at best unnecessary and at worst incorrect.
                                + undefined_variable_context.get_text().len(),
                        },
                    )
                    .with_implicit();
                self.new_declarations.push(decl);
            } else {
                // If we can't produce this, then we can't generate the
                // declaration.
                let diagnostic = Diagnostic::from_message(
                    format_cannot_determine_variable_type_error(&var_name),
                )
                .with_file_name(&self.source_file_name)
                .read_parser_rule_context(&*undefined_variable_context, self.tokens);
                self.diagnostics.push(diagnostic);
                continue;
            }
        }
        todo!()
    }
}

trait DeclarationVecExt {
    fn position(&self, declaration: &Declaration) -> Option<usize>;
    fn find_remove(&mut self, declaration: &Declaration);
}

impl DeclarationVecExt for Vec<Declaration> {
    fn position(&self, declaration: &Declaration) -> Option<usize> {
        self.iter()
            .filter_map(|decl| decl.eq(declaration, 1e-4).ok())
            .position(|eq| eq)
    }

    fn find_remove(&mut self, declaration: &Declaration) {
        if let Some(index) = self.position(declaration) {
            self.remove(index);
        }
    }
}

/// {0} = variable name
fn format_cannot_determine_variable_type_error(name: &str) -> String {
    format!("Can't figure out the type of variable {name} given its context. Specify its type with a <<declare>> statement.")
}

fn default_value_for_type(expression_type: &Option<Type>) -> Option<Convertible> {
    match expression_type.as_ref()? {
        Type::String => Some(Convertible::String(Default::default())),
        Type::Number => Some(Convertible::Number(Default::default())),
        Type::Boolean => Some(Convertible::Boolean(Default::default())),
        _ => None,
    }
}

fn get_hashable_interval<'input>(ctx: &impl ParserRuleContext<'input>) -> HashableInterval {
    let interval = ctx.get_source_interval();
    HashableInterval(interval)
}

fn filename(path: &str) -> &str {
    if let Some(os_str) = Path::new(path).file_name() {
        if let Some(file_name) = os_str.to_str() {
            return file_name;
        }
    }
    path
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
struct HashableInterval(Interval);

impl From<Interval> for HashableInterval {
    fn from(interval: Interval) -> Self {
        Self(interval)
    }
}

impl Ord for HashableInterval {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.a.cmp(&other.0.a).then(self.0.b.cmp(&other.0.b))
    }
}

impl PartialOrd for HashableInterval {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Hash for HashableInterval {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.a.hash(state);
        self.0.b.hash(state);
    }
}

impl Deref for HashableInterval {
    type Target = Interval;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HashableInterval {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Bandaid enum to allow static type checks that work via dynamic dispatch on C#
enum Term<'input> {
    Expression(Rc<ExpressionContextAll<'input>>),
    Variable(Rc<VariableContextAll<'input>>),
}

impl<'input> Term<'input> {
    fn generic_context(&self) -> Rc<ActualParserContext<'input>> {
        match self {
            Term::Expression(ctx) => ctx.clone() as Rc<ActualParserContext<'input>>,
            Term::Variable(ctx) => ctx.clone(),
        }
    }
}

impl<'input> Deref for Term<'input> {
    type Target = ActualParserContext<'input>;

    fn deref(&self) -> &Self::Target {
        match self {
            Term::Expression(ctx) => ctx.as_ref() as &ActualParserContext<'input>,
            Term::Variable(ctx) => ctx.as_ref(),
        }
    }
}

impl<'input> From<Rc<ExpressionContextAll<'input>>> for Term<'input> {
    fn from(ctx: Rc<ExpressionContextAll<'input>>) -> Self {
        Self::Expression(ctx)
    }
}

impl<'input> From<Rc<VariableContextAll<'input>>> for Term<'input> {
    fn from(ctx: Rc<VariableContextAll<'input>>) -> Self {
        Self::Variable(ctx)
    }
}
