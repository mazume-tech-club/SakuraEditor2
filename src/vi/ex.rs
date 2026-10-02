//! Ex コマンド（: で入力するコマンド）

use regex::{Regex, RegexBuilder};

use super::Vi;
use super::buf::Buf;

/// App 側で処理してほしいこと
#[derive(Debug, PartialEq)]
pub enum ExEffect {
    None,
    Msg(String),
    Error(String),
    Save(Option<String>),
    SaveQuit,
    Quit { force: bool },
    Edit(String),
    /// 一致行(exclude=false) / 非一致行(exclude=true) を Temp に抽出
    Filter { pattern: String, exclude: bool },
    Set(String),
    NoHighlight,
    Tail,
    Json,
    ViOff,
}

/// 行番号は 0 始まり
fn parse_addr(b: &dyn Buf, s: &str, cur: usize) -> Option<(usize, usize)> {
    let last = b.line_count().saturating_sub(1);
    let (base, rest) = if let Some(r) = s.strip_prefix('.') {
        (cur, r)
    } else if let Some(r) = s.strip_prefix('$') {
        (last, r)
    } else {
        let n = s.chars().take_while(|c| c.is_ascii_digit()).count();
        if n > 0 {
            (s[..n].parse::<usize>().ok()?.max(1) - 1, &s[n..])
        } else if s.starts_with(['+', '-']) {
            (cur, s)
        } else {
            return None;
        }
    };
    let mut line = base as i64;
    let mut rest = rest;
    while let Some(sign) = rest.chars().next().filter(|c| *c == '+' || *c == '-') {
        let num: String = rest[1..].chars().take_while(|c| c.is_ascii_digit()).collect();
        let v: i64 = if num.is_empty() { 1 } else { num.parse().ok()? };
        line += if sign == '+' { v } else { -v };
        rest = &rest[1 + num.len()..];
    }
    let consumed = s.len() - rest.len();
    Some((line.clamp(0, last as i64) as usize, consumed))
}

/// (範囲, 残りの文字列)。範囲指定が無ければ None
fn parse_range<'a>(vi: &Vi, b: &dyn Buf, s: &'a str) -> Result<(Option<(usize, usize)>, &'a str), String> {
    let cur = b.line_of(b.caret());
    let last = b.line_count().saturating_sub(1);
    if let Some(r) = s.strip_prefix('%') {
        return Ok((Some((0, last)), r));
    }
    if let Some(r) = s.strip_prefix("'<,'>") {
        let v = vi.last_visual_lines.ok_or("選択範囲がありません")?;
        return Ok((Some(v), r));
    }
    let Some((a, n)) = parse_addr(b, s, cur) else { return Ok((None, s)) };
    let rest = &s[n..];
    if let Some(r2) = rest.strip_prefix(',') {
        let (z, m) = parse_addr(b, r2, cur).ok_or("範囲の指定が不正です")?;
        return Ok((Some((a.min(z), a.max(z))), &r2[m..]));
    }
    Ok((Some((a, a)), rest))
}

/// `/pat/rep/flags` を区切り文字で分割する（\/ はエスケープ）
fn split_delim(s: &str) -> Option<(char, Vec<String>)> {
    let mut it = s.chars();
    let d = it.next()?;
    if d.is_alphanumeric() || d == '\\' || d == ' ' {
        return None;
    }
    let mut parts = vec![String::new()];
    let mut esc = false;
    for c in it {
        if esc {
            if c != d {
                parts.last_mut().unwrap().push('\\');
            }
            parts.last_mut().unwrap().push(c);
            esc = false;
        } else if c == '\\' {
            esc = true;
        } else if c == d {
            parts.push(String::new());
        } else {
            parts.last_mut().unwrap().push(c);
        }
    }
    if esc {
        parts.last_mut().unwrap().push('\\');
    }
    Some((d, parts))
}

/// Vim 形式の置換文字列を regex クレート形式へ
fn convert_replacement(rep: &str, eol: &str) -> String {
    let mut out = String::new();
    let mut chars = rep.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(d @ '0'..='9') => out += &format!("${{{d}}}"),
                Some('n' | 'r') => out += eol,
                Some('t') => out.push('\t'),
                Some('&') => out.push('&'),
                Some('\\') => out.push('\\'),
                Some(o) => out.push(o),
                None => out.push('\\'),
            },
            '&' => out += "${0}",
            '$' => out += "$$",
            o => out.push(o),
        }
    }
    out
}

fn vim_pattern(p: &str) -> String {
    p.replace("\\<", "\\b").replace("\\>", "\\b")
}

fn build(p: &str, flags: &str) -> Result<Regex, String> {
    let p = vim_pattern(p);
    let ci = flags.contains('i') || (!flags.contains('I') && !p.chars().any(|c| c.is_uppercase()));
    RegexBuilder::new(&p).case_insensitive(ci).build().map_err(|e| format!("正規表現エラー: {e}"))
}

fn lines_text(b: &dyn Buf, l1: usize, l2: usize) -> (usize, usize, String) {
    let (s, e) = (b.line_start(l1), b.line_end(l2));
    (s, e, b.text(s, e))
}

fn split_lines(t: &str) -> Vec<(&str, &str)> {
    // (行の内容, 改行)
    let mut v = Vec::new();
    let mut rest = t;
    loop {
        match rest.find('\n') {
            Some(i) => {
                let (line, eol) = if i > 0 && rest.as_bytes()[i - 1] == b'\r' { (&rest[..i - 1], &rest[i - 1..=i]) } else { (&rest[..i], &rest[i..=i]) };
                v.push((line, eol));
                rest = &rest[i + 1..];
            }
            None => {
                v.push((rest, ""));
                return v;
            }
        }
    }
}

fn substitute(vi: &mut Vi, b: &mut dyn Buf, range: (usize, usize), args: &str) -> ExEffect {
    let Some((_, parts)) = split_delim(args) else { return ExEffect::Error("書式: :s/検索/置換/g".into()) };
    let pat = if parts[0].is_empty() {
        match &vi.last_search {
            Some((p, _)) => p.clone(),
            None => return ExEffect::Error("検索パターンがありません".into()),
        }
    } else {
        parts[0].clone()
    };
    let rep = parts.get(1).cloned().unwrap_or_default();
    let flags = parts.get(2).cloned().unwrap_or_default();
    let re = match build(&pat, &flags) {
        Ok(r) => r,
        Err(e) => return ExEffect::Error(e),
    };
    vi.last_search = Some((pat, true));
    let rep = convert_replacement(&rep, b.eol());
    let global = flags.contains('g');
    let (s, e, text) = lines_text(b, range.0, range.1);
    let mut out = String::with_capacity(text.len());
    let (mut count, mut lines) = (0usize, 0usize);
    let mut last_changed = None;
    for (i, (line, eol)) in split_lines(&text).into_iter().enumerate() {
        let n = if global { re.find_iter(line).count() } else { re.is_match(line) as usize };
        if n > 0 {
            count += n;
            lines += 1;
            last_changed = Some(range.0 + i);
            let r = if global { re.replace_all(line, rep.as_str()) } else { re.replace(line, rep.as_str()) };
            out += &r;
        } else {
            out += line;
        }
        out += eol;
    }
    if count == 0 {
        return ExEffect::Error(format!("パターンが見つかりません: {}", parts[0]));
    }
    b.begin_undo();
    b.replace(s, e, &out);
    b.end_undo();
    if let Some(l) = last_changed {
        let l = l.min(b.line_count() - 1);
        b.set_caret(b.first_non_blank(l));
    }
    ExEffect::Msg(format!("{count} 箇所を置換しました（{lines} 行）"))
}

/// :g/pat/d, :v/pat/d
fn global_delete(b: &mut dyn Buf, range: (usize, usize), re: &Regex, invert: bool) -> ExEffect {
    let (s, _, _) = lines_text(b, range.0, range.1);
    let e = b.line_end_incl(range.1);
    let text = b.text(s, e);
    let mut out = String::with_capacity(text.len());
    let mut removed = 0;
    let lines = split_lines(&text);
    let n = lines.len();
    for (i, (line, eol)) in lines.into_iter().enumerate() {
        if i == n - 1 && line.is_empty() && eol.is_empty() && n > 1 {
            break;
        }
        if re.is_match(line) != invert {
            removed += 1;
        } else {
            out += line;
            out += if eol.is_empty() { "" } else { eol };
        }
    }
    if removed == 0 {
        return ExEffect::Msg("該当する行はありません".into());
    }
    b.begin_undo();
    b.replace(s, e, &out);
    b.end_undo();
    let l = b.line_of(s.min(b.len()));
    b.set_caret(b.first_non_blank(l));
    ExEffect::Msg(format!("{removed} 行削除しました"))
}

fn sort_lines(b: &mut dyn Buf, range: (usize, usize), args: &str) -> ExEffect {
    let (s, e, text) = lines_text(b, range.0, range.1);
    let eol = b.eol();
    let mut lines: Vec<&str> = split_lines(&text).into_iter().map(|(l, _)| l).collect();
    if args.contains('n') {
        let key = |l: &str| -> f64 {
            let num: String = l.trim_start().chars().take_while(|c| c.is_ascii_digit() || *c == '.' || *c == '-').collect();
            num.parse().unwrap_or(f64::MIN)
        };
        lines.sort_by(|a, b| key(a).partial_cmp(&key(b)).unwrap_or(std::cmp::Ordering::Equal));
    } else if args.contains('i') {
        lines.sort_by_key(|l| l.to_lowercase());
    } else {
        lines.sort();
    }
    if args.contains('!') {
        lines.reverse();
    }
    let before = lines.len();
    if args.contains('u') {
        lines.dedup();
    }
    let out = lines.join(eol);
    b.begin_undo();
    b.replace(s, e, &out);
    b.end_undo();
    let removed = before - lines.len();
    ExEffect::Msg(if removed > 0 { format!("並べ替えました（重複 {removed} 行を削除）") } else { "並べ替えました".into() })
}

pub fn run(vi: &mut Vi, b: &mut dyn Buf, input: &str) -> ExEffect {
    let input = input.trim_start_matches(':').trim();
    if input.is_empty() {
        return ExEffect::None;
    }
    let (range, rest) = match parse_range(vi, b, input) {
        Ok(v) => v,
        Err(e) => return ExEffect::Error(e),
    };
    let rest = rest.trim_start();
    let cur = b.line_of(b.caret());
    let whole = range.unwrap_or((0, b.line_count().saturating_sub(1)));
    let this = range.unwrap_or((cur, cur));

    // 範囲だけ → その行へジャンプ
    if rest.is_empty() {
        if let Some((_, l)) = range {
            b.set_caret(b.first_non_blank(l));
            return ExEffect::None;
        }
    }
    let name_len = rest.chars().take_while(|c| c.is_ascii_alphabetic()).map(|c| c.len_utf8()).sum::<usize>();
    let (name, arg) = rest.split_at(name_len);
    let bang = arg.starts_with('!');
    let arg_t = arg.trim_start_matches('!').trim();
    match name {
        "s" | "substitute" => substitute(vi, b, this, arg),
        "g" | "global" | "v" | "vglobal" => {
            let invert = name.starts_with('v') || bang;
            let src = arg.trim_start_matches('!');
            let Some((_, parts)) = split_delim(src) else { return ExEffect::Error("書式: :g/パターン/  または :g/パターン/d".into()) };
            let pat = parts[0].clone();
            if pat.is_empty() {
                return ExEffect::Error("パターンが空です".into());
            }
            let cmd = parts.get(1).map(|s| s.trim()).unwrap_or("");
            match cmd {
                "" | "p" | "print" => ExEffect::Filter { pattern: vim_pattern(&pat), exclude: invert },
                "d" | "delete" => match build(&pat, "") {
                    Ok(re) => global_delete(b, whole, &re, invert),
                    Err(e) => ExEffect::Error(e),
                },
                _ => ExEffect::Error(format!(":g の後のコマンド「{cmd}」は未対応です（空 / p / d のみ）")),
            }
        }
        "d" | "delete" => {
            let (s, e) = (b.line_start(this.0), b.line_end_incl(this.1));
            b.begin_undo();
            b.delete(s, e);
            b.end_undo();
            let l = b.line_of(s.min(b.len()));
            b.set_caret(b.first_non_blank(l));
            ExEffect::Msg(format!("{} 行削除しました", this.1 - this.0 + 1))
        }
        "sort" | "sor" => sort_lines(b, whole, arg),
        "w" | "write" => ExEffect::Save((!arg_t.is_empty()).then(|| arg_t.to_string())),
        "wq" | "x" | "xit" | "exit" => ExEffect::SaveQuit,
        "q" | "quit" | "qa" | "qall" | "close" => ExEffect::Quit { force: bang },
        "e" | "edit" | "o" | "open" => {
            if arg_t.is_empty() { ExEffect::Error("ファイル名を指定してください".into()) } else { ExEffect::Edit(arg_t.to_string()) }
        }
        "noh" | "nohlsearch" => ExEffect::NoHighlight,
        "set" | "se" => ExEffect::Set(arg_t.to_string()),
        "tail" => ExEffect::Tail,
        "json" => ExEffect::Json,
        "vi" | "novi" => ExEffect::ViOff,
        _ => ExEffect::Error(format!("未対応のコマンド: :{input}（F1 でチートシート）")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vi::buf::StrBuf;

    fn ex(text: &str, cmd: &str) -> (String, ExEffect) {
        let mut b = StrBuf::new(text);
        let mut vi = Vi::new();
        let r = run(&mut vi, &mut b, cmd);
        (b.s, r)
    }

    #[test]
    fn substitute_global_all_lines() {
        let (s, r) = ex("foo foo\nbar\nfoo\n", "%s/foo/X/g");
        assert_eq!(s, "X X\nbar\nX\n");
        assert_eq!(r, ExEffect::Msg("3 箇所を置換しました（2 行）".into()));
    }

    #[test]
    fn substitute_current_line_with_groups() {
        let (s, _) = ex("key=value\nkey=v2\n", r"s/(\w+)=(\w+)/\2=\1 &/");
        assert_eq!(s, "value=key key=value\nkey=v2\n");
    }

    #[test]
    fn substitute_range_and_dollar() {
        let (s, _) = ex("a\na\na\na", "2,$s/a/b/");
        assert_eq!(s, "a\nb\nb\nb");
        let (s, _) = ex("price 5\n", "s/5/$5/");
        assert_eq!(s, "price $5\n");
    }

    #[test]
    fn global_filter_and_delete() {
        let (_, r) = ex("a\nERROR x\n", "g/ERROR/");
        assert_eq!(r, ExEffect::Filter { pattern: "ERROR".into(), exclude: false });
        let (_, r) = ex("a\n", "v/DEBUG/");
        assert_eq!(r, ExEffect::Filter { pattern: "DEBUG".into(), exclude: true });
        let (s, _) = ex("INFO a\nDEBUG b\nINFO c\nDEBUG d\n", "g/DEBUG/d");
        assert_eq!(s, "INFO a\nINFO c\n");
        let (s, _) = ex("INFO a\nDEBUG b\nINFO c", "v/DEBUG/d");
        assert_eq!(s, "DEBUG b\n");
    }

    #[test]
    fn sort_unique() {
        let (s, _) = ex("c\na\nb\na", "sort u");
        assert_eq!(s, "a\nb\nc");
    }

    #[test]
    fn app_commands() {
        assert_eq!(ex("", "w").1, ExEffect::Save(None));
        assert_eq!(ex("", "w C:\\a.txt").1, ExEffect::Save(Some("C:\\a.txt".into())));
        assert_eq!(ex("", "q!").1, ExEffect::Quit { force: true });
        assert_eq!(ex("", "wq").1, ExEffect::SaveQuit);
    }

    #[test]
    fn goto_line() {
        let mut b = StrBuf::new("a\nb\n  c\n");
        let mut vi = Vi::new();
        run(&mut vi, &mut b, "3");
        assert_eq!(b.caret, 6);
    }
}
