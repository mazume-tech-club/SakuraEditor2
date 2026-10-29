//! CSV / TSV の列揃え（空白を足して区切り位置を揃える）と、その解除

/// 区切り文字を決める。最初の空でない行にタブがあれば TSV、なければ CSV
fn delimiter(text: &str) -> char {
    let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    if first.contains('\t') { '\t' } else { ',' }
}

/// 1 行をフィールドに分ける。"..." の中の区切り文字は区切りとみなさない（"" はエスケープ）。
/// フィールドは元の文字列をそのまま（引用符も含めて）返す
fn split(line: &str, delim: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut quoted = false;
    for (i, c) in line.char_indices() {
        if c == '"' {
            quoted = !quoted;
        } else if c == delim && !quoted {
            out.push(&line[start..i]);
            start = i + c.len_utf8();
        }
    }
    out.push(&line[start..]);
    out
}

/// 等幅フォントでの表示幅（全角は 2）
fn width(s: &str) -> usize {
    s.chars().map(|c| if is_wide(c) { 2 } else { 1 }).sum()
}

fn is_wide(c: char) -> bool {
    matches!(c as u32,
        0x1100..=0x115F
        | 0x2E80..=0x303E
        | 0x3041..=0x33FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xA000..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1FAFF
        | 0x20000..=0x3FFFD)
}

/// 行と改行の組に分ける（最後の改行の後に空行は作らない）
fn lines_with_eol(text: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        match rest.find('\n') {
            Some(i) => {
                let (body, eol) = if i > 0 && rest.as_bytes()[i - 1] == b'\r' { (&rest[..i - 1], "\r\n") } else { (&rest[..i], "\n") };
                out.push((body, eol));
                rest = &rest[i + 1..];
            }
            None => {
                out.push((rest, ""));
                rest = "";
            }
        }
    }
    out
}

/// 各列の幅をそろえる。既に揃えてある場合も一度詰めてから揃え直す。
/// 区切り文字が 1 つもなければ None
pub fn align(text: &str) -> Option<String> {
    let delim = delimiter(text);
    let lines = lines_with_eol(text);
    let rows: Vec<Vec<&str>> = lines.iter().map(|(l, _)| split(l, delim).into_iter().map(str::trim).collect()).collect();
    if rows.iter().all(|r| r.len() < 2) {
        return None;
    }
    let mut widths: Vec<usize> = Vec::new();
    for r in &rows {
        // 最後の列は埋めないので幅に含めない
        for (i, f) in r.iter().enumerate().take(r.len().saturating_sub(1)) {
            if widths.len() <= i {
                widths.push(0);
            }
            widths[i] = widths[i].max(width(f));
        }
    }
    let mut out = String::with_capacity(text.len() + text.len() / 2);
    for (r, (_, eol)) in rows.iter().zip(&lines) {
        for (i, f) in r.iter().enumerate() {
            out.push_str(f);
            if i + 1 < r.len() {
                out.extend(std::iter::repeat_n(' ', widths[i] - width(f)));
                out.push(delim);
                if delim == ',' {
                    out.push(' ');
                }
            }
        }
        out.push_str(eol);
    }
    Some(out)
}

/// 列揃えで足した空白を取り除く（各フィールドの前後の空白を詰める）
pub fn shrink(text: &str) -> Option<String> {
    let delim = delimiter(text);
    let lines = lines_with_eol(text);
    let mut found = false;
    let mut out = String::with_capacity(text.len());
    for (l, eol) in &lines {
        let fields = split(l, delim);
        found |= fields.len() > 1;
        for (i, f) in fields.iter().enumerate() {
            if i > 0 {
                out.push(delim);
            }
            out.push_str(if fields.len() > 1 { f.trim() } else { f });
        }
        out.push_str(eol);
    }
    found.then_some(out)
}

/// 列揃えした状態のままか（空白を詰めて揃え直すと今の本文に一致し、詰めると変わる）
pub fn is_aligned(text: &str) -> bool {
    match shrink(text) {
        Some(s) if s != text => align(&s).as_deref() == Some(text),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn align_csv_with_wide_chars_and_quotes() {
        let src = "id,name,price\r\n1,りんご,100\r\n22,\"a,b\",5\r\n";
        let out = align(src).unwrap();
        assert_eq!(out, "id, name  , price\r\n1 , りんご, 100\r\n22, \"a,b\" , 5\r\n");
    }

    #[test]
    fn shrink_restores_original() {
        let src = "id,name,price\n1,りんご,100\n22,\"a,b\",5";
        assert_eq!(shrink(&align(src).unwrap()).unwrap(), src);
    }

    #[test]
    fn align_is_idempotent() {
        let once = align("a,bb\nccc,d\n").unwrap();
        assert_eq!(align(&once).unwrap(), once);
    }

    #[test]
    fn tsv_and_ragged_rows() {
        let out = align("a\tb\tc\nlong\tx\n").unwrap();
        assert_eq!(out, "a   \tb\tc\nlong\tx\n");
    }

    #[test]
    fn detects_aligned_state() {
        let src = "id,name\n1,りんご\n";
        let aligned = align(src).unwrap();
        assert!(is_aligned(&aligned));
        assert!(!is_aligned(src));
        // 揃えたあと手で 1 か所崩したら「揃えたまま」ではない
        assert!(!is_aligned(&aligned.replacen("id", "identifier", 1)));
    }

    #[test]
    fn no_delimiter() {
        assert!(align("hello\nworld\n").is_none());
        assert!(shrink("hello\n").is_none());
    }
}
