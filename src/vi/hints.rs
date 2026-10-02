//! Vi 学習用のヒントとチートシート

use super::{Act, Cmd, CmdKind, InsPos, Key, Motion, Obj, Op, Target, keys_label};

fn op_name(op: Op) -> &'static str {
    match op {
        Op::Delete => "削除",
        Op::Change => "削除して挿入",
        Op::Yank => "ヤンク(コピー)",
        Op::Indent => "インデント",
        Op::Outdent => "インデント解除",
        Op::Lower => "小文字化",
        Op::Upper => "大文字化",
        Op::Toggle => "大小文字反転",
    }
}

fn motion_name(m: Motion) -> String {
    match m {
        Motion::Left => "左へ".into(),
        Motion::Right => "右へ".into(),
        Motion::Down => "下の行へ".into(),
        Motion::Up => "上の行へ".into(),
        Motion::WordFwd(false) => "次の単語の先頭へ".into(),
        Motion::WordFwd(true) => "次の WORD(空白区切り) の先頭へ".into(),
        Motion::WordEnd(false) => "単語の末尾へ".into(),
        Motion::WordEnd(true) => "WORD の末尾へ".into(),
        Motion::WordBack(false) => "前の単語の先頭へ".into(),
        Motion::WordBack(true) => "前の WORD の先頭へ".into(),
        Motion::LineStart => "行頭へ".into(),
        Motion::FirstNonBlank => "行の最初の非空白文字へ".into(),
        Motion::LineEnd => "行末へ".into(),
        Motion::GotoLast => "最終行（数字付きならその行）へ".into(),
        Motion::GotoFirst => "先頭行（数字付きならその行）へ".into(),
        Motion::Find { c, forward: true, till: false } => format!("右の「{c}」へ"),
        Motion::Find { c, forward: true, till: true } => format!("右の「{c}」の手前へ"),
        Motion::Find { c, forward: false, till: false } => format!("左の「{c}」へ"),
        Motion::Find { c, forward: false, till: true } => format!("左の「{c}」の直後へ"),
        Motion::RepeatFind(false) => "直前の f/t を繰り返す".into(),
        Motion::RepeatFind(true) => "直前の f/t を逆方向に".into(),
        Motion::MatchPair => "対応する括弧へ".into(),
        Motion::ParaFwd => "次の空行(段落)へ".into(),
        Motion::ParaBack => "前の空行(段落)へ".into(),
        Motion::ScreenTop => "画面の一番上へ".into(),
        Motion::ScreenMid => "画面の中央へ".into(),
        Motion::ScreenBottom => "画面の一番下へ".into(),
        Motion::SearchNext(false) => "次の検索結果へ".into(),
        Motion::SearchNext(true) => "前の検索結果へ".into(),
        Motion::Star(true) => "カーソル下の単語を下へ検索".into(),
        Motion::Star(false) => "カーソル下の単語を上へ検索".into(),
        Motion::NextLine => "次の行の先頭へ".into(),
        Motion::PrevLine => "前の行の先頭へ".into(),
    }
}

fn obj_name(o: Obj) -> String {
    let (inner, what) = match o {
        Obj::Word { big, around } => (!around, if big { "WORD".to_string() } else { "単語".to_string() }),
        Obj::Quote { q, around } => (!around, format!("{q}…{q}")),
        Obj::Pair { open, close, around } => (!around, format!("{open}…{close}")),
    };
    if inner { format!("{what}の内側") } else { format!("{what}全体(周囲含む)") }
}

fn act_name(a: Act) -> &'static str {
    match a {
        Act::PasteAfter => "カーソルの後に貼り付け",
        Act::PasteBefore => "カーソルの前に貼り付け",
        Act::Undo => "元に戻す",
        Act::Redo => "やり直し",
        Act::Repeat => "直前の変更を繰り返す",
        Act::Join => "次の行と連結",
        Act::Insert(InsPos::Before) => "カーソル位置から挿入",
        Act::Insert(InsPos::LineStart) => "行頭から挿入",
        Act::Insert(InsPos::After) => "カーソルの後ろから挿入",
        Act::Insert(InsPos::LineEnd) => "行末から挿入",
        Act::Insert(InsPos::Below) => "下に新しい行を作って挿入",
        Act::Insert(InsPos::Above) => "上に新しい行を作って挿入",
        Act::Visual => "ビジュアル(文字)選択",
        Act::VisualLine => "ビジュアル(行)選択",
        Act::ToggleCase => "大小文字を反転して右へ",
        Act::ReplaceChar(_) => "1 文字置き換え",
        Act::Cmdline(':') => "コマンドライン（:w 保存, :q 閉じる, :%s/a/b/g 置換, :g/re/ 抽出）",
        Act::Cmdline('/') => "下方向へ検索",
        Act::Cmdline(_) => "上方向へ検索",
        Act::SaveQuit => "保存して閉じる",
        Act::QuitForce => "保存せず閉じる",
        Act::ScrollCenter => "カーソル行を画面中央に",
        Act::ScrollTop => "カーソル行を画面上端に",
        Act::ScrollBottom => "カーソル行を画面下端に",
        Act::HalfDown => "半画面下へ",
        Act::HalfUp => "半画面上へ",
        Act::PageDown => "1 画面下へ",
        Act::PageUp => "1 画面上へ",
        Act::LineDown => "1 行スクロール(下)",
        Act::LineUp => "1 行スクロール(上)",
        Act::SwapEnds => "選択の反対側の端へ",
    }
}

pub(crate) fn describe(cmd: &Cmd) -> String {
    let mut keys = String::new();
    if let Some(c) = cmd.count {
        keys += &c.to_string();
    }
    keys += &keys_label(&cmd.body);
    let times = match cmd.count {
        Some(c) if c > 1 => format!("（{c} 回）"),
        _ => String::new(),
    };
    let desc = match cmd.kind {
        CmdKind::Move(m) => motion_name(m),
        CmdKind::Operate(op, Target::Line) => format!("行を{}", op_name(op)),
        CmdKind::Operate(op, Target::Object(o)) => format!("{}を{}", obj_name(o), op_name(op)),
        CmdKind::Operate(op, Target::Motion(m)) => {
            let target = motion_name(m);
            let target = target.trim_end_matches('へ');
            format!("{target}まで{}", op_name(op))
        }
        CmdKind::Act(a) => act_name(a).into(),
    };
    format!("{keys}  … {desc}{times}")
}

/// 入力途中のキーに対する「次に何を押せるか」の案内
pub(crate) fn pending(keys: &[Key]) -> String {
    let s = keys_label(keys);
    let last = s.trim_start_matches(|c: char| c.is_ascii_digit());
    let guide = match last {
        "" => "数字: 回数指定。続けてコマンドを入力",
        "d" => "削除: w 単語 / $ 行末 / d 行全体 / iw 単語内 / i\" 引用符内 / t文字 その手前まで",
        "c" => "変更: w 単語 / $ 行末 / c 行全体 / iw 単語内 / i( 括弧内",
        "y" => "ヤンク: w 単語 / $ 行末 / y 行全体 / iw 単語内 / ip は未対応",
        ">" | "<" => "インデント: もう一度押すと行単位、または j/k/G などのモーション",
        "g" => "g: g 先頭行へ / u 小文字化 / U 大文字化 / ~ 大小反転",
        "f" | "F" | "t" | "T" => "移動先の文字を 1 つ入力",
        "r" => "置き換える文字を 1 つ入力",
        "z" => "z: z 中央 / t 上端 / b 下端 にスクロール",
        "Z" => "Z: Z 保存して閉じる / Q 保存せず閉じる",
        _ if last.ends_with('i') || last.ends_with('a') => "テキストオブジェクト: w 単語 / \" ' ` 引用符 / ( [ { < 括弧",
        _ => "続けて入力…（Esc で取り消し）",
    };
    format!("{s}  … {guide}")
}

pub const CHEAT_SHEET: &str = r#"Sakura2 Vi モード チートシート                     （F1 でいつでも表示 / Ctrl+Alt+V で Vi モード切替）
================================================================================================

■ モード
  Esc / Ctrl+[      ノーマルモードへ戻る
  i  a  I  A        挿入: カーソル前 / 後 / 行頭 / 行末
  o  O              下 / 上に行を作って挿入
  v  V              ビジュアル選択（文字 / 行）  … 選択して d y c > < ~ u U J p
  :                 コマンドライン

■ 移動（モーション） ※先に数字で回数指定: 3w, 10j
  h j k l           ← ↓ ↑ →
  w b e             次の単語頭 / 前の単語頭 / 単語末    （W B E は空白区切り）
  0 ^ $             行頭 / 最初の文字 / 行末
  gg G  5G          先頭行 / 最終行 / 5 行目
  f{c} t{c}         行内で文字 c へ / c の手前へ   （F T は左方向、; , で繰り返し）
  %                 対応する括弧
  { }               前 / 次の空行
  H M L             画面の上 / 中 / 下
  Ctrl+d Ctrl+u     半画面 下 / 上        Ctrl+f Ctrl+b   1 画面 下 / 上
  zz zt zb          カーソル行を中央 / 上 / 下に

■ 編集（オペレータ + モーション）
  d{motion}         削除      例: dw  d$  dG  dt,  d3j
  c{motion}         削除して挿入   例: cw  ciw  ci"  ci(
  y{motion}         ヤンク(コピー)  ※Windows クリップボードと連動
  > <               インデント / 解除   例: >>  >ip は未対応
  gu gU g~          小文字 / 大文字 / 反転   例: gUiw
  dd cc yy          行全体         3dd = 3 行削除
  x X               1 文字削除（右 / 左）
  D C Y             行末まで削除 / 変更 / 行ヤンク
  p P               後ろ / 前に貼り付け
  r{c}              1 文字置換
  J                 行を連結
  ~                 大小文字反転
  u  Ctrl+r         元に戻す / やり直し
  .                 直前の変更を繰り返す（超便利!）

■ テキストオブジェクト（d c y の後に）
  iw aw             単語の中 / 単語 + 空白
  i" a"  i' i`      引用符の中 / 引用符ごと
  i( a(  i[ i{ i<   括弧の中 / 括弧ごと

■ 検索（正規表現は VSCode と同じ書式。小文字だけなら大小無視 = smartcase）
  /pattern  ?pattern   下 / 上へ検索      n N  次 / 前
  * #                  カーソル下の単語を検索

■ コマンドライン（:）
  :w  :w ファイル名    保存 / 名前を付けて保存
  :q  :q!  :wq  :x     閉じる / 破棄して閉じる / 保存して閉じる
  :e ファイル名        開く
  :123                 123 行目へ
  :s/old/new/g         現在行を置換     :%s/old/new/g  全体を置換（\1 や & で参照）
  :'<,'>s/a/b/g        選択範囲を置換
  :g/ERROR/            ★ 一致行を抽出して C:\Windows\Temp に出力（ログ検索!）
  :v/DEBUG/            ★ 一致しない行を抽出
  :g/re/d  :v/re/d     一致する / しない行を削除
  :sort  :sort u       行をソート / 重複削除
  :noh                 検索ハイライトを消す
  :set nu / nonu       行番号 表示 / 非表示     :set wrap / nowrap  折り返し
  :tail                ファイル追従(tail -f) の切替
  :json                JSON 整形（Alt+Shift+F と同じ）
  :vi                  Vi モードを終了して通常のエディタ操作へ

■ 練習のコツ
  1. まずは  h j k l  と  i / Esc  だけで 1 日過ごす
  2. 次に  w b  と  dd yy p  を覚える
  3. 慣れたら  ciw  ci"  dt,  .  を使ってみる（ステータスバーに意味が出ます）
"#;
