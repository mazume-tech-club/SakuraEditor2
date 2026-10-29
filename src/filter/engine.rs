//! 行フィルタエンジン。包含/除外パターンを RegexSet にまとめ、チャンク単位で並列評価する。

use memchr::memchr;
use rayon::prelude::*;
use regex::bytes::{Regex, RegexBuilder, RegexSet, RegexSetBuilder};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PatternSpec {
    pub text: String,
    pub exclude: bool,
    pub case_sensitive: bool,
    pub regex: bool,
    /// ハイライト色のインデックス
    pub color: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FilterSpec {
    pub patterns: Vec<PatternSpec>,
    pub line_numbers: bool,
}

impl FilterSpec {
    pub fn active(&self) -> impl Iterator<Item = &PatternSpec> {
        self.patterns.iter().filter(|p| !p.text.is_empty())
    }

    /// タブ名などに使う短い要約
    pub fn summary(&self) -> String {
        let v: Vec<String> = self
            .active()
            .map(|p| if p.exclude { format!("!{}", p.text) } else { p.text.clone() })
            .collect();
        v.join(" ")
    }
}

pub struct Compiled {
    include: Option<RegexSet>,
    exclude: Option<RegexSet>,
    /// 包含パターンのハイライト用 (正規表現, 色)
    pub highlights: Vec<(Regex, usize)>,
}

fn source(p: &PatternSpec) -> String {
    if p.regex { p.text.clone() } else { regex::escape(&p.text) }
}

/// 大文字を含まなければ大小無視（Vim の smartcase と同じ）にするかは呼び出し側で決める
pub fn build_regex(pattern: &str, case_sensitive: bool) -> Result<Regex, String> {
    RegexBuilder::new(pattern)
        .case_insensitive(!case_sensitive)
        .multi_line(true)
        .size_limit(64 << 20)
        .build()
        .map_err(|e| e.to_string())
}

pub fn compile(spec: &FilterSpec) -> Result<Compiled, String> {
    let mut inc = Vec::new();
    let mut exc = Vec::new();
    let mut highlights = Vec::new();
    for p in spec.active() {
        let src = source(p);
        // RegexSet は大小無視をパターン単位で持てないので (?i) を前置する
        let flagged = if p.case_sensitive { src.clone() } else { format!("(?i:{src})") };
        let re = build_regex(&src, p.case_sensitive).map_err(|e| format!("「{}」: {e}", p.text))?;
        if p.exclude {
            exc.push(flagged);
        } else {
            inc.push(flagged);
            highlights.push((re, p.color));
        }
    }
    if inc.is_empty() && exc.is_empty() {
        return Err("条件が空です".into());
    }
    let set = |v: Vec<String>| -> Result<Option<RegexSet>, String> {
        if v.is_empty() {
            return Ok(None);
        }
        RegexSetBuilder::new(v).size_limit(64 << 20).build().map(Some).map_err(|e| e.to_string())
    };
    Ok(Compiled { include: set(inc)?, exclude: set(exc)?, highlights })
}

/// 一致した行: (0 始まりの行番号, 行頭バイト, 改行を含む行末バイト)
pub type LineHit = (u32, usize, usize);

const CHUNK: usize = 4 << 20;

impl Compiled {
    #[inline]
    pub fn line_matches(&self, line: &[u8]) -> bool {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if let Some(inc) = &self.include {
            if !inc.is_match(line) {
                return false;
            }
        }
        if let Some(exc) = &self.exclude {
            if exc.is_match(line) {
                return false;
            }
        }
        true
    }

    fn scan(&self, text: &[u8], offset: usize, mut f: impl FnMut(usize, usize, usize)) -> usize {
        let mut line = 0usize;
        let mut start = 0usize;
        while start < text.len() {
            let (content_end, next) = match memchr(b'\n', &text[start..]) {
                Some(i) => (start + i, start + i + 1),
                None => (text.len(), text.len()),
            };
            if self.line_matches(&text[start..content_end]) {
                f(line, offset + start, offset + next);
            }
            line += 1;
            start = next;
        }
        line
    }

    /// 一致行を列挙する。`base_line` は text 先頭の行番号。
    pub fn matching_lines(&self, text: &[u8], base_line: usize) -> Vec<LineHit> {
        let chunks = split_chunks(text);
        let parts: Vec<(Vec<LineHit>, usize)> = chunks
            .par_iter()
            .map(|&(s, e)| {
                let mut hits = Vec::new();
                let n = self.scan(&text[s..e], s, |l, a, b| hits.push((l as u32, a, b)));
                (hits, n)
            })
            .collect();
        let mut out = Vec::new();
        let mut line_base = base_line;
        for (hits, n) in parts {
            out.extend(hits.into_iter().map(|(l, a, b)| (l + line_base as u32, a, b)));
            line_base += n;
        }
        out
    }

    /// (一致行数, 総行数)
    pub fn count(&self, text: &[u8]) -> (usize, usize) {
        split_chunks(text)
            .par_iter()
            .map(|&(s, e)| {
                let mut c = 0;
                let n = self.scan(&text[s..e], s, |_, _, _| c += 1);
                (c, n)
            })
            .reduce(|| (0, 0), |a, b| (a.0 + b.0, a.1 + b.1))
    }
}

/// 改行境界でおよそ CHUNK バイトずつに分割
fn split_chunks(text: &[u8]) -> Vec<(usize, usize)> {
    let mut v = Vec::new();
    let mut s = 0;
    while s < text.len() {
        let mut e = (s + CHUNK).min(text.len());
        if e < text.len() {
            e = match memchr(b'\n', &text[e..]) {
                Some(i) => e + i + 1,
                None => text.len(),
            };
        }
        v.push((s, e));
        s = e;
    }
    v
}

/// 一致行から出力テキストと「出力行 → 元の行番号」の対応表を作る
pub fn build_output(text: &[u8], hits: &[LineHit], line_numbers: bool) -> (Vec<u8>, Vec<u32>) {
    let total: usize = hits.iter().map(|h| h.2 - h.1 + 12).sum();
    let mut out = Vec::with_capacity(total);
    let mut map = Vec::with_capacity(hits.len());
    for &(line, s, e) in hits {
        if line_numbers {
            out.extend_from_slice(format!("L{}: ", line + 1).as_bytes());
        }
        out.extend_from_slice(&text[s..e]);
        if !text[s..e].ends_with(b"\n") {
            out.extend_from_slice(if text[s..e].ends_with(b"\r") { b"\n" } else { b"\r\n" });
        }
        map.push(line);
    }
    (out, map)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str, exclude: bool) -> PatternSpec {
        PatternSpec { text: text.into(), exclude, regex: true, ..Default::default() }
    }

    const LOG: &[u8] = b"INFO start\r\nERROR disk full\r\nWARN slow\r\nerror lower\r\nINFO ERROR-ish ok\r\nlast";

    #[test]
    fn include_and_exclude() {
        let spec = FilterSpec { patterns: vec![p("error", false), p("ok$", true)], line_numbers: false };
        let c = compile(&spec).unwrap();
        let hits = c.matching_lines(LOG, 0);
        let lines: Vec<u32> = hits.iter().map(|h| h.0).collect();
        assert_eq!(lines, vec![1, 3]);
        assert_eq!(c.count(LOG), (2, 6));
    }

    #[test]
    fn case_sensitive_literal() {
        let spec = FilterSpec {
            patterns: vec![PatternSpec { text: "ERROR".into(), case_sensitive: true, ..Default::default() }],
            line_numbers: true,
        };
        let c = compile(&spec).unwrap();
        let hits = c.matching_lines(LOG, 0);
        let (out, map) = build_output(LOG, &hits, true);
        assert_eq!(map, vec![1, 4]);
        assert_eq!(String::from_utf8(out).unwrap(), "L2: ERROR disk full\r\nL5: INFO ERROR-ish ok\r\n");
    }

    #[test]
    fn exclude_only_and_last_line_without_newline() {
        let mut pat = p("^(INFO|WARN|ERROR)", true);
        pat.case_sensitive = true;
        let spec = FilterSpec { patterns: vec![pat], line_numbers: false };
        let c = compile(&spec).unwrap();
        let (out, _) = build_output(LOG, &c.matching_lines(LOG, 0), false);
        assert_eq!(String::from_utf8(out).unwrap(), "error lower\r\nlast\r\n");
    }

    #[test]
    fn chunked_parallel_matches_sequential() {
        let mut big = Vec::new();
        for i in 0..400_000 {
            big.extend_from_slice(format!("{i} {}\n", if i % 7 == 0 { "HIT" } else { "miss" }).as_bytes());
        }
        let spec = FilterSpec { patterns: vec![p("HIT", false)], line_numbers: false };
        let c = compile(&spec).unwrap();
        let hits = c.matching_lines(&big, 10);
        assert_eq!(hits.len(), 400_000 / 7 + 1);
        assert_eq!(hits[1].0, 17);
        let last = hits.last().unwrap();
        assert!(std::str::from_utf8(&big[last.1..last.2]).unwrap().contains("HIT"));
    }

    #[test]
    fn invalid_regex_reports_error() {
        let spec = FilterSpec { patterns: vec![p("(", false)], line_numbers: false };
        assert!(compile(&spec).is_err());
    }
}
