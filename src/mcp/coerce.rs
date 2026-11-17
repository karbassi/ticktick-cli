//! FastMCP validates arguments with lax pydantic, which accepts `"50"` or `25.0`
//! for an int, `true` for an int or float, and `"yes"`, `"off"`, `1` or `0.0`
//! for a bool. Callers written against it
//! send those, and serde rejects them, so arguments are coerced against the
//! tool's input schema first. Anything not convertible is left for serde to report.

use rmcp::model::JsonObject;
use serde_json::Value;

pub fn coerce_arguments(schema: &JsonObject, args: &mut JsonObject) {
    let Some(props) = schema.get("properties").and_then(Value::as_object) else {
        return;
    };
    for (name, value) in args.iter_mut() {
        if let Some(prop) = props.get(name) {
            coerce(prop, value);
        }
    }
}

/// `type` as a string or list, plus the members of `anyOf`/`oneOf`.
fn types(schema: &Value) -> Vec<&str> {
    let mut out = Vec::new();
    match schema.get("type") {
        Some(Value::String(t)) => out.push(t.as_str()),
        Some(Value::Array(ts)) => out.extend(ts.iter().filter_map(Value::as_str)),
        _ => {}
    }
    for key in ["anyOf", "oneOf"] {
        if let Some(Value::Array(members)) = schema.get(key) {
            out.extend(members.iter().flat_map(types));
        }
    }
    out
}

fn item_schema(schema: &Value) -> Option<&Value> {
    schema.get("items").or_else(|| {
        ["anyOf", "oneOf"]
            .iter()
            .filter_map(|key| schema.get(*key).and_then(Value::as_array))
            .flatten()
            .find_map(item_schema)
    })
}

fn coerce(schema: &Value, value: &mut Value) {
    let types = types(schema);
    if types.iter().any(|t| is_type(t, value)) {
        if let (Value::Array(items), Some(item)) = (&mut *value, item_schema(schema)) {
            for v in items {
                coerce(item, v);
            }
        }
        return;
    }
    if let Some(converted) = types.iter().find_map(|t| convert(t, value)) {
        *value = converted;
    }
}

fn is_type(t: &str, v: &Value) -> bool {
    match t {
        "integer" => v.is_i64() || v.is_u64(),
        "number" => v.is_number(),
        "boolean" => v.is_boolean(),
        "string" => v.is_string(),
        "null" => v.is_null(),
        "array" => v.is_array(),
        "object" => v.is_object(),
        _ => false,
    }
}

fn integral(f: f64) -> Option<Value> {
    (f.is_finite() && f.fract() == 0.0 && f.abs() < i64::MAX as f64).then(|| Value::from(f as i64))
}

fn convert(t: &str, v: &Value) -> Option<Value> {
    match (t, v) {
        ("integer", Value::Number(n)) => n.as_f64().and_then(integral),
        ("integer", Value::String(s)) => {
            let s = s.trim();
            s.parse::<i64>()
                .ok()
                .map(Value::from)
                .or_else(|| s.parse::<f64>().ok().and_then(integral))
        }
        ("number", Value::String(s)) => s
            .trim()
            .parse::<f64>()
            .ok()
            .filter(|f| f.is_finite())
            .map(Value::from),
        ("number", Value::Bool(b)) => Some(Value::from(if *b { 1.0 } else { 0.0 })),
        ("integer", Value::Bool(b)) => Some(Value::from(i64::from(*b))),
        ("boolean", Value::Number(n)) => match n.as_f64() {
            Some(0.0) => Some(Value::Bool(false)),
            Some(1.0) => Some(Value::Bool(true)),
            _ => None,
        },
        // Unlike its int and float parsing, pydantic doesn't trim these.
        ("boolean", Value::String(s)) => match s.to_ascii_lowercase().as_str() {
            "1" | "on" | "t" | "true" | "y" | "yes" => Some(Value::Bool(true)),
            "0" | "off" | "f" | "false" | "n" | "no" => Some(Value::Bool(false)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn coerced(prop: Value, value: Value) -> Value {
        let schema = json!({"properties": {"x": prop}});
        let mut args = json!({"x": value});
        coerce_arguments(schema.as_object().unwrap(), args.as_object_mut().unwrap());
        args["x"].clone()
    }

    #[test]
    fn integers() {
        let int = json!({"type": "integer"});
        assert_eq!(coerced(int.clone(), json!("50")), json!(50));
        assert_eq!(coerced(int.clone(), json!(" 7 ")), json!(7));
        assert_eq!(coerced(int.clone(), json!(25.0)), json!(25));
        assert_eq!(coerced(int.clone(), json!("25.0")), json!(25));
        assert_eq!(coerced(int.clone(), json!(25.5)), json!(25.5));
        assert_eq!(coerced(int, json!("abc")), json!("abc"));
    }

    #[test]
    fn numbers_and_booleans() {
        assert_eq!(coerced(json!({"type": "number"}), json!("1.5")), json!(1.5));
        assert_eq!(
            coerced(json!({"type": "number"}), json!("nan")),
            json!("nan")
        );
    }

    /// Cases checked against pydantic 2.12 lax `validate_python`, as FastMCP runs it.
    #[test]
    fn booleans_like_pydantic() {
        let b = json!({"type": "boolean"});
        for (input, want) in [
            (json!("true"), json!(true)),
            (json!("False"), json!(false)),
            (json!("YES"), json!(true)),
            (json!("y"), json!(true)),
            (json!("On"), json!(true)),
            (json!("t"), json!(true)),
            (json!("1"), json!(true)),
            (json!("no"), json!(false)),
            (json!("off"), json!(false)),
            (json!("f"), json!(false)),
            (json!("n"), json!(false)),
            (json!("0"), json!(false)),
            (json!(1), json!(true)),
            (json!(0), json!(false)),
            (json!(1.0), json!(true)),
            (json!(0.0), json!(false)),
        ] {
            assert_eq!(coerced(b.clone(), input.clone()), want, "{input}");
        }
        for unchanged in [
            json!(" yes "),
            json!("2"),
            json!("1.0"),
            json!(""),
            json!(2),
            json!(-1),
            json!(1.5),
        ] {
            assert_eq!(coerced(b.clone(), unchanged.clone()), unchanged);
        }
    }

    #[test]
    fn booleans_to_numbers() {
        assert_eq!(coerced(json!({"type": "integer"}), json!(true)), json!(1));
        assert_eq!(coerced(json!({"type": "integer"}), json!(false)), json!(0));
        assert_eq!(coerced(json!({"type": "number"}), json!(true)), json!(1.0));
    }

    #[test]
    fn nullable_forms() {
        let list = json!({"type": ["integer", "null"]});
        assert_eq!(coerced(list.clone(), json!("5")), json!(5));
        assert_eq!(coerced(list, Value::Null), Value::Null);
        let any_of = json!({"anyOf": [{"type": "boolean"}, {"type": "null"}]});
        assert_eq!(coerced(any_of, json!("true")), json!(true));
    }

    #[test]
    fn a_string_type_keeps_strings() {
        let s = json!({"type": ["string", "integer"]});
        assert_eq!(coerced(s, json!("5")), json!("5"));
    }

    #[test]
    fn array_items() {
        let arr = json!({"type": ["array", "null"], "items": {"type": "integer"}});
        assert_eq!(coerced(arr, json!(["1", 2.0, "x"])), json!([1, 2, "x"]));
        let any_of =
            json!({"anyOf": [{"type": "array", "items": {"type": "boolean"}}, {"type": "null"}]});
        assert_eq!(coerced(any_of, json!(["true"])), json!([true]));
    }
}
