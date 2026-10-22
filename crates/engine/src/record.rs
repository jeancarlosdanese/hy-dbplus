//! Linhas como JSON (painel de registro, CLI e MCP): os valores vêm como texto do driver e o tipo
//! do catálogo diz o que vira número, booleano, JSON cru ou string. Nenhuma inspeção do texto
//! decide tipo: "123" numa coluna `text` é string.

use serde_json::{Map, Value};

/// Como o tipo legível do catálogo (`format_type`) é tratado no JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Number,
    Bool,
    Json,
    Text,
}

/// Classifica o tipo pelo nome (sem o tamanho/precisão: `numeric(10,2)` → `numeric`).
fn kind(data_type: Option<&str>) -> Kind {
    let base = data_type
        .unwrap_or("")
        .split('(')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    match base.as_str() {
        "smallint" | "integer" | "bigint" | "numeric" | "real" | "double precision" | "decimal"
        | "oid" => Kind::Number,
        "boolean" => Kind::Bool,
        "json" | "jsonb" => Kind::Json,
        _ => Kind::Text,
    }
}

/// Uma célula como valor JSON. Texto que não cabe no tipo (número inválido, JSON inválido) fica
/// string: melhor mostrar o que o servidor mandou do que esconder.
fn cell(value: Option<&str>, data_type: Option<&str>) -> Value {
    let Some(text) = value else {
        return Value::Null;
    };
    match kind(data_type) {
        Kind::Number => serde_json::from_str::<serde_json::Number>(text)
            .map(Value::Number)
            .unwrap_or_else(|_| Value::String(text.to_owned())),
        Kind::Bool => match text {
            "t" | "true" => Value::Bool(true),
            "f" | "false" => Value::Bool(false),
            _ => Value::String(text.to_owned()),
        },
        Kind::Json => {
            serde_json::from_str::<Value>(text).unwrap_or_else(|_| Value::String(text.to_owned()))
        }
        Kind::Text => Value::String(text.to_owned()),
    }
}

/// Uma linha como objeto JSON, chaves na ordem das colunas. `types` acompanha as colunas (`None`
/// quando o catálogo ainda não as deu: vira string).
pub fn row_to_value(
    columns: &[String],
    types: &[Option<String>],
    cells: &[Option<String>],
) -> Value {
    let mut object = Map::with_capacity(columns.len());
    for (i, name) in columns.iter().enumerate() {
        let value = cells.get(i).and_then(|c| c.as_deref());
        let data_type = types.get(i).and_then(|t| t.as_deref());
        object.insert(name.clone(), cell(value, data_type));
    }
    Value::Object(object)
}

/// Uma linha vira um objeto; várias viram uma lista de objetos. Indentado com dois espaços.
pub fn to_json(columns: &[String], types: &[Option<String>], rows: &[&[Option<String>]]) -> String {
    let value = match rows {
        [single] => row_to_value(columns, types, single),
        many => Value::Array(
            many.iter()
                .map(|cells| row_to_value(columns, types, cells))
                .collect(),
        ),
    };
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "null".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cols(names: &[&str]) -> Vec<String> {
        names.iter().map(|s| (*s).to_owned()).collect()
    }
    fn types(names: &[Option<&str>]) -> Vec<Option<String>> {
        names.iter().map(|s| s.map(str::to_owned)).collect()
    }
    fn row(cells: &[Option<&str>]) -> Vec<Option<String>> {
        cells.iter().map(|c| c.map(str::to_owned)).collect()
    }

    #[test]
    fn uma_linha_vira_objeto_com_os_tipos_do_catalogo() {
        let columns = cols(&["id", "nome", "preco", "ativo", "tags", "cod", "sem_tipo"]);
        let types = types(&[
            Some("integer"),
            Some("character varying(80)"),
            Some("numeric(10,2)"),
            Some("boolean"),
            Some("jsonb"),
            Some("text"),
            None,
        ]);
        let r = row(&[
            Some("1"),
            Some("a\"b"),
            Some("12.50"),
            Some("t"),
            Some("[\"x\",{\"y\":2}]"),
            Some("123"),
            Some("7"),
        ]);
        let json = to_json(&columns, &types, &[&r]);
        assert_eq!(
            json,
            "{\n  \"id\": 1,\n  \"nome\": \"a\\\"b\",\n  \"preco\": 12.50,\n  \"ativo\": true,\n  \"tags\": [\n    \"x\",\n    {\n      \"y\": 2\n    }\n  ],\n  \"cod\": \"123\",\n  \"sem_tipo\": \"7\"\n}"
        );
    }

    #[test]
    fn null_numero_invalido_e_json_invalido() {
        let columns = cols(&["n", "j", "b"]);
        let types = types(&[Some("integer"), Some("json"), Some("boolean")]);
        let r = row(&[None, Some("{nao json"), Some("talvez")]);
        let v = row_to_value(&columns, &types, &r);
        assert_eq!(v["n"], Value::Null);
        assert_eq!(v["j"], Value::String("{nao json".into()));
        assert_eq!(v["b"], Value::String("talvez".into()));
        let r = row(&[Some("abc"), Some("null"), Some("f")]);
        let v = row_to_value(&columns, &types, &r);
        assert_eq!(v["n"], Value::String("abc".into()));
        assert_eq!(v["j"], Value::Null, "json cru 'null' é null");
        assert_eq!(v["b"], Value::Bool(false));
    }

    #[test]
    fn varias_linhas_viram_lista_e_colunas_a_mais_ou_a_menos_nao_quebram() {
        let columns = cols(&["id", "nome"]);
        let types = types(&[Some("bigint")]);
        let a = row(&[Some("1"), Some("a")]);
        let b = row(&[Some("2")]);
        let json = to_json(&columns, &types, &[&a, &b]);
        let v: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(v.as_array().map(Vec::len), Some(2));
        assert_eq!(v[0]["nome"], Value::String("a".into()));
        assert_eq!(v[1]["nome"], Value::Null, "célula ausente é null");
        assert_eq!(v[1]["id"], Value::Number(2.into()));
    }
}
