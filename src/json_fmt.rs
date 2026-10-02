//! Alt+Shift+F の JSON 整形

use serde::Serialize;
use serde_json::Value;
use serde_json::ser::{PrettyFormatter, Serializer};

#[derive(Debug, PartialEq)]
pub struct JsonError {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

fn pretty(v: &Value, indent: &[u8]) -> String {
    let mut out = Vec::new();
    let fmt = PrettyFormatter::with_indent(indent);
    let mut ser = Serializer::with_formatter(&mut out, fmt);
    v.serialize(&mut ser).expect("serialize");
    String::from_utf8(out).expect("utf8")
}

/// テキスト全体を整形する。JSON Lines（複数の値の連続）にも対応。
pub fn format(text: &str, indent: &[u8], eol: &str) -> Result<String, JsonError> {
    let mut parts = Vec::new();
    let stream = serde_json::Deserializer::from_str(text).into_iter::<Value>();
    for v in stream {
        match v {
            Ok(v) => parts.push(pretty(&v, indent)),
            Err(e) => {
                return Err(JsonError { line: e.line(), column: e.column(), message: e.to_string() });
            }
        }
    }
    if parts.is_empty() {
        return Err(JsonError { line: 1, column: 1, message: "JSON が見つかりません".into() });
    }
    let mut s = parts.join("\n");
    if eol != "\n" {
        s = s.replace('\n', eol);
    }
    if text.ends_with('\n') {
        s.push_str(eol);
    }
    Ok(s)
}

/// ログ行の中に埋め込まれた JSON を探して整形する。
/// 戻り値: (行内の開始バイト, 終了バイト, 整形後テキスト)
pub fn format_embedded(line: &str, indent: &[u8], eol: &str) -> Option<(usize, usize, String)> {
    for (i, c) in line.char_indices() {
        if c != '{' && c != '[' {
            continue;
        }
        let mut it = serde_json::Deserializer::from_str(&line[i..]).into_iter::<Value>();
        if let Some(Ok(v)) = it.next() {
            if !(v.is_object() || v.is_array()) {
                continue;
            }
            let end = i + it.byte_offset();
            let mut s = pretty(&v, indent);
            if eol != "\n" {
                s = s.replace('\n', eol);
            }
            return Some((i, end, s));
        }
    }
    None
}

/// 1 行に圧縮する（Alt+Shift+M）
pub fn minify(text: &str) -> Result<String, JsonError> {
    let mut parts = Vec::new();
    for v in serde_json::Deserializer::from_str(text).into_iter::<Value>() {
        match v {
            Ok(v) => parts.push(v.to_string()),
            Err(e) => return Err(JsonError { line: e.line(), column: e.column(), message: e.to_string() }),
        }
    }
    Ok(parts.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_and_keeps_key_order_and_numbers() {
        let s = format(r#"{"b":1,"a":[1,2],"n":12345678901234567890.123}"#, b"  ", "\n").unwrap();
        assert_eq!(s, "{\n  \"b\": 1,\n  \"a\": [\n    1,\n    2\n  ],\n  \"n\": 12345678901234567890.123\n}");
    }

    #[test]
    fn json_lines() {
        let s = format("{\"a\":1}\n{\"b\":2}\n", b"  ", "\n").unwrap();
        assert_eq!(s, "{\n  \"a\": 1\n}\n{\n  \"b\": 2\n}\n");
    }

    #[test]
    fn crlf() {
        let s = format("[1]", b"  ", "\r\n").unwrap();
        assert_eq!(s, "[\r\n  1\r\n]");
    }

    #[test]
    fn reports_error_position() {
        let e = format("{\n  \"a\": ,\n}", b"  ", "\n").unwrap_err();
        assert_eq!(e.line, 2);
    }

    #[test]
    fn embedded_in_log_line() {
        let line = r#"2024-01-01 INFO req={"id":1,"ok":true} done"#;
        let (s, e, t) = format_embedded(line, b"  ", "\n").unwrap();
        assert_eq!(&line[s..e], r#"{"id":1,"ok":true}"#);
        assert!(t.starts_with("{\n  \"id\": 1"));
    }

    #[test]
    fn minify_works() {
        assert_eq!(minify("{\n \"a\" : [1, 2]\n}").unwrap(), r#"{"a":[1,2]}"#);
    }
}
