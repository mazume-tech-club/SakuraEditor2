use super::buf::StrBuf;
use super::*;

/// キー列を文字列で与える。<Esc> <CR> <BS> と ^r のような Ctrl 表記に対応
fn keys(s: &str) -> Vec<Key> {
    let mut v = Vec::new();
    let mut rest = s;
    while let Some(c) = rest.chars().next() {
        if let Some(r) = rest.strip_prefix("<Esc>") {
            v.push(Key::Esc);
            rest = r;
        } else if let Some(r) = rest.strip_prefix("<CR>") {
            v.push(Key::Enter);
            rest = r;
        } else if let Some(r) = rest.strip_prefix("<BS>") {
            v.push(Key::Backspace);
            rest = r;
        } else if c == '^' && rest.len() > 1 {
            let n = rest[1..].chars().next().unwrap();
            v.push(Key::Ctrl(n));
            rest = &rest[1 + n.len_utf8()..];
        } else {
            v.push(Key::Char(c));
            rest = &rest[c.len_utf8()..];
        }
    }
    v
}

/// 挿入モードの文字はエンジンが消費しないので、テストでは自前で挿入する
fn feed(vi: &mut Vi, b: &mut StrBuf, s: &str) {
    for k in keys(s) {
        let o = vi.key(b, k);
        if !o.consumed {
            match k {
                Key::Char(c) => {
                    let p = b.caret;
                    b.insert(p, &c.to_string());
                }
                Key::Enter => {
                    let p = b.caret;
                    b.insert(p, "\n");
                }
                Key::Backspace => {
                    let p = b.caret;
                    let q = b.prev_pos(p);
                    b.delete(q, p);
                }
                _ => {}
            }
        }
    }
}

fn run(text: &str, caret: usize, input: &str) -> (String, usize, Vi) {
    let mut b = StrBuf::new(text);
    b.set_caret(caret);
    let mut vi = Vi::new();
    feed(&mut vi, &mut b, input);
    (b.s.clone(), b.caret, vi)
}

fn check(text: &str, caret: usize, input: &str, expect: &str, expect_caret: usize) {
    let (s, c, _) = run(text, caret, input);
    assert_eq!(s, expect, "input={input}");
    assert_eq!(c, expect_caret, "caret input={input}");
}

#[test]
fn basic_motions() {
    let t = "hello world foo\nsecond line\n";
    assert_eq!(run(t, 0, "w").1, 6);
    assert_eq!(run(t, 0, "2w").1, 12);
    assert_eq!(run(t, 0, "e").1, 4);
    assert_eq!(run(t, 12, "b").1, 6);
    assert_eq!(run(t, 0, "$").1, 14);
    assert_eq!(run(t, 5, "0").1, 0);
    assert_eq!(run(t, 3, "j").1, 19);
    assert_eq!(run(t, 0, "G").1, 28, "Scintilla と同じく末尾改行の後の空行も 1 行");
    assert_eq!(run(t, 20, "gg").1, 0);
    assert_eq!(run(t, 0, "fo").1, 4);
    assert_eq!(run(t, 0, "to").1, 3);
    assert_eq!(run(t, 0, "fo;").1, 7);
    assert_eq!(run(t, 0, "l").1, 1);
    assert_eq!(run(t, 14, "l").1, 14, "l は行末を越えない");
}

#[test]
fn word_motion_with_punctuation_and_japanese() {
    let t = "foo.bar(baz)";
    assert_eq!(run(t, 0, "w").1, 3);
    assert_eq!(run(t, 0, "W").1, 11);
    let j = "ログ出力のエラーです";
    // カタカナ → 漢字 → ひらがな → カタカナ … の境界で止まる
    assert_eq!(run(j, 0, "w").1, "ログ".len());
    assert_eq!(run(j, 0, "ww").1, "ログ出力".len());
}

#[test]
fn delete_ops() {
    check("one two three", 0, "dw", "two three", 0);
    check("one two three", 4, "d$", "one ", 3);
    check("one two three", 4, "D", "one ", 3);
    check("a\nb\nc\n", 2, "dd", "a\nc\n", 2);
    check("a\nb\nc\nd\n", 0, "2dd", "c\nd\n", 0);
    check("a\nb\nc", 4, "dd", "a\nb", 2);
    check("abc", 1, "x", "ac", 1);
    check("abc", 1, "3x", "a", 0);
    check("abc def", 4, "X", "abcdef", 3);
    check("one two\nthree", 4, "dw", "one \nthree", 3);
    check("a\nb\nc\nd\n", 0, "dj", "c\nd\n", 0);
    check("a\nb\nc\nd\n", 2, "dG", "a\n", 0);
    check("call(x, y)", 5, "dt)", "call()", 5);
}

#[test]
fn change_and_insert() {
    check("one two", 0, "cwXY<Esc>", "XY two", 1);
    check("one two", 0, "ciwZ<Esc>", "Z two", 0);
    check("say \"hello\" ok", 6, "ci\"bye<Esc>", "say \"bye\" ok", 7);
    check("f(a, b)", 3, "di(", "f()", 2);
    check("f(a, b)", 3, "da(", "f", 0);
    check("abc", 0, "iX<Esc>", "Xabc", 0);
    check("abc", 0, "aX<Esc>", "aXbc", 1);
    check("abc", 1, "AX<Esc>", "abcX", 3);
    check("  abc", 4, "IX<Esc>", "  Xabc", 2);
    check("  a\nb", 0, "oX<Esc>", "  a\n  X\nb", 6);
    check("a\nb", 2, "OX<Esc>", "a\nX\nb", 2);
    check("abc", 0, "3ix<Esc>", "xxxabc", 2);
    check("hello", 0, "C!<Esc>", "!", 0);
    check("a\n  b\nc", 2, "ccX<Esc>", "a\n  X\nc", 4);
    check("abc", 0, "sZ<Esc>", "Zbc", 0);
}

#[test]
fn yank_and_paste() {
    check("a\nb\n", 0, "yyp", "a\na\nb\n", 2);
    check("a\nb\n", 2, "yyP", "a\nb\nb\n", 2);
    check("a\nb", 2, "yyp", "a\nb\nb", 4);
    check("one two", 0, "yiwP", "oneone two", 2);
    check("ab", 0, "xp", "ba", 1);
    check("a\nb\nc\n", 0, "ddp", "b\na\nc\n", 2);
}

#[test]
fn undo_redo_groups_insert() {
    check("abc", 0, "ixyz<Esc>u", "abc", 0);
    let (s, _, _) = run("abc", 0, "ixyz<Esc>u^r");
    assert_eq!(s, "xyzabc");
    check("one two three", 0, "dwdwu", "two three", 0);
}

#[test]
fn dot_repeat() {
    check("a b c d", 0, "dw..", "d", 0);
    check("x\nx\nx\n", 0, "ciwYES<Esc>j0.", "YES\nYES\nx\n", 6);
    check("a\nb\nc\nd\n", 0, "dd2.", "d\n", 0);
    check("aaaa", 0, "x3.", "", 0);
}

#[test]
fn visual_mode() {
    check("hello world", 0, "vllld", "o world", 0);
    check("hello world", 0, "veyP", "hellohello world", 4);
    check("a\nb\nc\n", 0, "Vjd", "c\n", 0);
    check("abc", 0, "vlU", "ABc", 0);
    check("a\nb\nc", 0, "VjJ", "a b\nc", 1);
    check("a\nb\n", 0, "Vj>", "    a\n    b\n", 4);
    let (_, _, vi) = run("a\nb\nc\n", 0, "Vj<Esc>");
    assert_eq!(vi.mode, Mode::Normal);
    assert_eq!(vi.last_visual_lines, Some((0, 1)));
}

#[test]
fn operators_misc() {
    check("Hello", 0, "g~iw", "hELLO", 0);
    check("hello world", 0, "gUiw", "HELLO world", 0);
    check("Hello", 0, "~~", "hEllo", 2);
    check("a\nb\n", 0, ">>", "    a\nb\n", 4);
    check("    a\n", 4, "<<", "a\n", 0);
    check("abc", 0, "rx", "xbc", 0);
    check("abc", 0, "2rx", "xxc", 1);
    check("a\n  b\nc", 0, "J", "a b\nc", 1);
    check("a\nb\nc\nd", 0, "3J", "a b c\nd", 3);
}

#[test]
fn search_and_star() {
    let t = "foo bar\nbaz foo\nfoo";
    let mut b = StrBuf::new(t);
    let mut vi = Vi::new();
    vi.search(&mut b, "foo", true).unwrap();
    assert_eq!(b.caret, 12);
    feed(&mut vi, &mut b, "n");
    assert_eq!(b.caret, 16);
    feed(&mut vi, &mut b, "n");
    assert_eq!(b.caret, 0, "折り返し");
    feed(&mut vi, &mut b, "N");
    assert_eq!(b.caret, 16);
    let (_, c, _) = run("ab xx ab", 0, "*");
    assert_eq!(c, 6);
    // smartcase: 大文字を含むと区別
    let mut b = StrBuf::new("Foo foo");
    let mut vi = Vi::new();
    vi.search(&mut b, "foo", true).unwrap();
    assert_eq!(b.caret, 4);
}

#[test]
fn match_pair_and_paragraph() {
    assert_eq!(run("if (a(b)) x", 3, "%").1, 8);
    assert_eq!(run("if (a(b)) x", 8, "%").1, 3);
    assert_eq!(run("a\nb\n\nc\n", 0, "}").1, 4);
    assert_eq!(run("a\n\nb\nc", 5, "{").1, 2);
}

#[test]
fn counts_and_pending() {
    let mut b = StrBuf::new("a b c d e f");
    let mut vi = Vi::new();
    feed(&mut vi, &mut b, "d");
    assert!(vi.hint.contains("削除"));
    feed(&mut vi, &mut b, "<Esc>");
    feed(&mut vi, &mut b, "2d2w");
    assert_eq!(b.s, "e f");
    assert!(vi.hint.contains("削除"));
}

#[test]
fn hint_describes_command() {
    let (_, _, vi) = run("one two", 0, "dw");
    assert_eq!(vi.hint, "dw  … 次の単語の先頭まで削除");
    let (_, _, vi) = run("one two", 0, "ciw<Esc>");
    assert!(vi.hint.contains("Esc"));
}

#[test]
fn clipboard_interop() {
    let mut b = StrBuf::new("abc");
    let mut vi = Vi::new();
    feed(&mut vi, &mut b, "yl");
    assert_eq!(b.clip.as_deref(), Some("a"));
    b.clip = Some("XYZ".into());
    feed(&mut vi, &mut b, "P");
    assert_eq!(b.s, "XYZabc");
}

#[test]
fn mouse_selection_enters_visual() {
    let mut b = StrBuf::new("hello world");
    let mut vi = Vi::new();
    b.set_selection(0, 5);
    feed(&mut vi, &mut b, "d");
    assert_eq!(b.s, " world");
    assert_eq!(vi.mode, Mode::Normal);
}

