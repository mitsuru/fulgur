use std::collections::BTreeMap;

use minijinja::machinery::{WhitespaceConfig, ast, parse};
use minijinja::syntax::SyntaxConfig;
use serde_json::{Value, json};

/// MiniJinjaテンプレートを解析し、JSON Schemaを生成する。
pub fn extract_schema(template_str: &str, template_name: &str) -> crate::error::Result<Value> {
    let stmt = parse(
        template_str,
        template_name,
        SyntaxConfig::default(),
        WhitespaceConfig::default(),
    )?;

    let mut root = BTreeMap::new();
    let scope = BTreeMap::new();
    collect_from_stmt(&stmt, &mut root, &scope);

    let mut schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "description": format!("Schema for template {}", template_name),
    });

    let properties = inferred_map_to_schema(&root);
    schema["properties"] = properties;

    Ok(schema)
}

#[derive(Debug, Clone)]
enum InferredType {
    String,
    Object(BTreeMap<String, InferredType>),
    Array(Box<InferredType>),
}

/// Convert a BTreeMap of inferred types to a JSON Schema "properties" object.
fn inferred_map_to_schema(map: &BTreeMap<String, InferredType>) -> Value {
    let mut props = serde_json::Map::new();
    for (key, ty) in map {
        props.insert(key.clone(), inferred_to_schema(ty));
    }
    Value::Object(props)
}

fn inferred_to_schema(t: &InferredType) -> Value {
    match t {
        InferredType::String => json!({"type": "string"}),
        InferredType::Object(fields) => {
            let mut schema = json!({"type": "object"});
            schema["properties"] = inferred_map_to_schema(fields);
            schema
        }
        InferredType::Array(inner) => {
            let mut schema = json!({"type": "array"});
            schema["items"] = inferred_to_schema(inner);
            schema
        }
    }
}

/// Scope maps loop variable names to paths in the root context.
/// For example, `{% for item in items %}` maps "item" to ["items"].
type Scope = BTreeMap<String, Vec<String>>;

fn collect_from_stmt(
    stmt: &ast::Stmt<'_>,
    root: &mut BTreeMap<String, InferredType>,
    scope: &Scope,
) {
    match stmt {
        ast::Stmt::Template(t) => {
            for child in &t.children {
                collect_from_stmt(child, root, scope);
            }
        }
        ast::Stmt::EmitExpr(e) => {
            collect_from_expr(&e.expr, root, scope);
        }
        ast::Stmt::ForLoop(f) => {
            // Extract loop variable name from target
            let loop_var = match &f.target {
                ast::Expr::Var(v) => v.id.to_string(),
                _ => return, // Tuple unpacking etc. not supported yet
            };

            // Resolve the iterator expression to a path
            let iter_path = resolve_expr_path(&f.iter, scope);

            if let Some(path) = &iter_path {
                // Mark the iter path as an Array in root
                ensure_array_at_path(root, path);
            }

            // Create new scope with loop variable mapped
            let mut new_scope = scope.clone();
            if let Some(path) = iter_path {
                new_scope.insert(loop_var, path);
            }

            for child in &f.body {
                collect_from_stmt(child, root, &new_scope);
            }
            for child in &f.else_body {
                collect_from_stmt(child, root, scope);
            }
        }
        ast::Stmt::IfCond(c) => {
            collect_from_expr(&c.expr, root, scope);
            for child in &c.true_body {
                collect_from_stmt(child, root, scope);
            }
            for child in &c.false_body {
                collect_from_stmt(child, root, scope);
            }
        }
        ast::Stmt::WithBlock(w) => {
            for child in &w.body {
                collect_from_stmt(child, root, scope);
            }
        }
        ast::Stmt::FilterBlock(fb) => {
            for child in &fb.body {
                collect_from_stmt(child, root, scope);
            }
        }
        ast::Stmt::AutoEscape(ae) => {
            for child in &ae.body {
                collect_from_stmt(child, root, scope);
            }
        }
        ast::Stmt::SetBlock(sb) => {
            for child in &sb.body {
                collect_from_stmt(child, root, scope);
            }
        }
        _ => {}
    }
}

fn collect_from_expr(
    expr: &ast::Expr<'_>,
    root: &mut BTreeMap<String, InferredType>,
    scope: &Scope,
) {
    // Try to resolve expression to a variable path and register it
    if let Some(path) = resolve_expr_path(expr, scope) {
        resolve_path(root, &path, InferredType::String);
    }

    // Also recurse into sub-expressions for filters, calls, etc.
    match expr {
        ast::Expr::Filter(f) => {
            if let Some(ref e) = f.expr {
                collect_from_expr(e, root, scope);
            }
            for arg in &f.args {
                collect_from_call_arg(arg, root, scope);
            }
        }
        ast::Expr::Call(c) => {
            collect_from_expr(&c.expr, root, scope);
            for arg in &c.args {
                collect_from_call_arg(arg, root, scope);
            }
        }
        ast::Expr::Test(t) => {
            collect_from_expr(&t.expr, root, scope);
            for arg in &t.args {
                collect_from_call_arg(arg, root, scope);
            }
        }
        ast::Expr::BinOp(b) => {
            collect_from_expr(&b.left, root, scope);
            collect_from_expr(&b.right, root, scope);
        }
        ast::Expr::UnaryOp(u) => {
            collect_from_expr(&u.expr, root, scope);
        }
        ast::Expr::IfExpr(i) => {
            collect_from_expr(&i.test_expr, root, scope);
            collect_from_expr(&i.true_expr, root, scope);
            if let Some(ref fe) = i.false_expr {
                collect_from_expr(fe, root, scope);
            }
        }
        ast::Expr::GetAttr(_) | ast::Expr::Var(_) => {
            // Already handled above via resolve_expr_path
        }
        _ => {}
    }
}

fn collect_from_call_arg(
    arg: &ast::CallArg<'_>,
    root: &mut BTreeMap<String, InferredType>,
    scope: &Scope,
) {
    match arg {
        ast::CallArg::Pos(e)
        | ast::CallArg::Kwarg(_, e)
        | ast::CallArg::PosSplat(e)
        | ast::CallArg::KwargSplat(e) => {
            collect_from_expr(e, root, scope);
        }
    }
}

/// Resolve an expression to a path of variable names.
/// Returns None if the expression is not a simple variable/attribute chain.
fn resolve_expr_path(expr: &ast::Expr<'_>, scope: &Scope) -> Option<Vec<String>> {
    match expr {
        ast::Expr::Var(v) => {
            let name = v.id;
            // Skip built-in variables
            if name == "loop"
                || name == "self"
                || name == "true"
                || name == "false"
                || name == "none"
            {
                return None;
            }
            if let Some(base_path) = scope.get(name) {
                Some(base_path.clone())
            } else {
                Some(vec![name.to_string()])
            }
        }
        ast::Expr::GetAttr(a) => {
            let mut base = resolve_expr_path(&a.expr, scope)?;
            base.push(a.name.to_string());
            Some(base)
        }
        _ => None,
    }
}

/// Merge a path into the root BTreeMap as nested InferredType entries.
/// The leaf is set to `leaf_type`. Intermediate nodes become Object.
fn resolve_path(
    root: &mut BTreeMap<String, InferredType>,
    path: &[String],
    leaf_type: InferredType,
) {
    if path.is_empty() {
        return;
    }

    let key = &path[0];
    if path.len() == 1 {
        // Leaf: only insert if not already present (don't overwrite Object with String)
        root.entry(key.clone()).or_insert(leaf_type);
    } else {
        // Intermediate: ensure this is an Object, then recurse
        let entry = root
            .entry(key.clone())
            .or_insert_with(|| InferredType::Object(BTreeMap::new()));
        match entry {
            InferredType::Object(children) => {
                resolve_path(children, &path[1..], leaf_type);
            }
            InferredType::Array(inner) => {
                // Path continues into array items
                match inner.as_mut() {
                    InferredType::Object(children) => {
                        resolve_path(children, &path[1..], leaf_type);
                    }
                    other => {
                        // Upgrade from String to Object
                        let mut children = BTreeMap::new();
                        resolve_path(&mut children, &path[1..], leaf_type);
                        *other = InferredType::Object(children);
                    }
                }
            }
            _ => {
                // Upgrade String to Object
                let mut children = BTreeMap::new();
                resolve_path(&mut children, &path[1..], leaf_type);
                *entry = InferredType::Object(children);
            }
        }
    }
}

/// Ensure the path points to an Array type in the root map.
fn ensure_array_at_path(root: &mut BTreeMap<String, InferredType>, path: &[String]) {
    if path.is_empty() {
        return;
    }

    let key = &path[0];
    if path.len() == 1 {
        let entry = root
            .entry(key.clone())
            .or_insert_with(|| InferredType::Array(Box::new(InferredType::String)));
        // If it was previously something else, convert to Array
        if !matches!(entry, InferredType::Array(_)) {
            *entry = InferredType::Array(Box::new(InferredType::String));
        }
    } else {
        let entry = root
            .entry(key.clone())
            .or_insert_with(|| InferredType::Object(BTreeMap::new()));
        if let InferredType::Object(children) = entry {
            ensure_array_at_path(children, &path[1..]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_variable() {
        let schema = extract_schema("{{ title }}", "test.html").unwrap();
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["title"]["type"], "string");
    }

    #[test]
    fn test_for_loop_infers_array() {
        let schema =
            extract_schema("{% for item in items %}{{ item }}{% endfor %}", "test.html").unwrap();
        assert_eq!(schema["properties"]["items"]["type"], "array");
    }

    #[test]
    fn test_nested_attr_infers_object() {
        let schema = extract_schema("{{ user.name }}", "test.html").unwrap();
        assert_eq!(schema["properties"]["user"]["type"], "object");
        assert_eq!(
            schema["properties"]["user"]["properties"]["name"]["type"],
            "string"
        );
    }

    #[test]
    fn test_for_loop_with_attr() {
        let schema = extract_schema(
            "{% for item in items %}{{ item.name }}{% endfor %}",
            "test.html",
        )
        .unwrap();
        let items = &schema["properties"]["items"];
        assert_eq!(items["type"], "array");
        assert_eq!(items["items"]["type"], "object");
        assert_eq!(items["items"]["properties"]["name"]["type"], "string");
    }

    #[test]
    fn test_schema_metadata() {
        let schema = extract_schema("{{ x }}", "invoice.html").unwrap();
        assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
        assert!(
            schema["description"]
                .as_str()
                .unwrap()
                .contains("invoice.html")
        );
    }
}
