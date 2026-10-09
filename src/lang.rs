//! 拡張子ごとのレクサーと配色

use std::path::Path;

use crate::lexer_consts::*;
use crate::sci::Sci;
use crate::sci_consts::*;
use crate::theme::Palette;
use crate::util::rgb;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Text,
    Log,
    Json,
    Cpp,
    JavaScript,
    CSharp,
    Java,
    Go,
    Python,
    Rust,
    Sql,
    Html,
    Xml,
    Css,
    Markdown,
    Yaml,
    Ini,
    Batch,
    PowerShell,
    Shell,
    Diff,
}

impl Lang {
    pub fn from_path(p: &Path) -> Lang {
        let ext = p.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        let name = p.file_name().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
        match ext.as_str() {
            "log" | "out" | "trace" => Lang::Log,
            "json" | "jsonc" | "json5" | "ndjson" | "jsonl" | "geojson" | "har" => Lang::Json,
            "c" | "h" | "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" | "ino" => Lang::Cpp,
            "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "mts" => Lang::JavaScript,
            "cs" => Lang::CSharp,
            "java" | "kt" | "kts" | "scala" | "groovy" | "gradle" => Lang::Java,
            "go" => Lang::Go,
            "py" | "pyw" | "pyi" => Lang::Python,
            "rs" => Lang::Rust,
            "sql" => Lang::Sql,
            "html" | "htm" | "xhtml" | "vue" | "php" | "aspx" | "cshtml" | "jsp" => Lang::Html,
            "xml" | "xaml" | "csproj" | "vbproj" | "props" | "targets" | "config" | "resx" | "svg" | "xsd" | "xsl" | "plist" | "manifest" => Lang::Xml,
            "css" | "scss" | "less" => Lang::Css,
            "md" | "markdown" => Lang::Markdown,
            "yml" | "yaml" => Lang::Yaml,
            "ini" | "cfg" | "conf" | "properties" | "toml" | "env" | "inf" | "reg" | "editorconfig" | "gitconfig" => Lang::Ini,
            "bat" | "cmd" => Lang::Batch,
            "ps1" | "psm1" | "psd1" => Lang::PowerShell,
            "sh" | "bash" | "zsh" => Lang::Shell,
            "diff" | "patch" => Lang::Diff,
            _ if name.contains(".log") || name.ends_with("log") => Lang::Log,
            _ if name == "dockerfile" || name == "makefile" => Lang::Shell,
            _ => Lang::Text,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Lang::Text => "テキスト",
            Lang::Log => "ログ",
            Lang::Json => "JSON",
            Lang::Cpp => "C/C++",
            Lang::JavaScript => "JS/TS",
            Lang::CSharp => "C#",
            Lang::Java => "Java",
            Lang::Go => "Go",
            Lang::Python => "Python",
            Lang::Rust => "Rust",
            Lang::Sql => "SQL",
            Lang::Html => "HTML",
            Lang::Xml => "XML",
            Lang::Css => "CSS",
            Lang::Markdown => "Markdown",
            Lang::Yaml => "YAML",
            Lang::Ini => "INI",
            Lang::Batch => "Batch",
            Lang::PowerShell => "PowerShell",
            Lang::Shell => "Shell",
            Lang::Diff => "Diff",
        }
    }

    pub fn is_log_like(self) -> bool {
        matches!(self, Lang::Log | Lang::Text)
    }
}

const CPP_KW: &str = "alignas alignof asm auto bool break case catch char class const constexpr const_cast continue decltype default delete do double dynamic_cast else enum explicit export extern false float for friend goto if inline int long mutable namespace new noexcept nullptr operator private protected public register reinterpret_cast return short signed sizeof static static_assert static_cast struct switch template this thread_local throw true try typedef typeid typename union unsigned using virtual void volatile wchar_t while override final";
const JS_KW: &str = "abstract any as async await boolean break case catch class const constructor continue debugger declare default delete do else enum export extends false finally for from function get if implements import in instanceof interface let module namespace new null number of package private protected public readonly return set static string super switch symbol this throw true try type typeof undefined var void while with yield";
const CS_KW: &str = "abstract as async await base bool break byte case catch char checked class const continue decimal default delegate do double else enum event explicit extern false finally fixed float for foreach get goto if implicit in int interface internal is lock long namespace new null object operator out override params private protected public readonly record ref return sbyte sealed set short sizeof stackalloc static string struct switch this throw true try typeof uint ulong unchecked unsafe ushort using var virtual void volatile when where while yield";
const JAVA_KW: &str = "abstract assert boolean break byte case catch char class const continue default do double else enum extends final finally float for fun goto if implements import instanceof int interface long native new null package private protected public return short static strictfp super switch synchronized this throw throws transient true false try val var void volatile while when object data sealed override";
const GO_KW: &str = "break case chan const continue default defer else fallthrough for func go goto if import interface map package range return select struct switch type var nil true false bool byte error float32 float64 int int8 int16 int32 int64 rune string uint uint8 uint16 uint32 uint64 uintptr any";
const PY_KW: &str = "False None True and as assert async await break class continue def del elif else except finally for from global if import in is lambda nonlocal not or pass raise return try while with yield match case self";
const RUST_KW: &str = "as async await break const continue crate dyn else enum extern false fn for if impl in let loop match mod move mut pub ref return self Self static struct super trait true type unsafe use where while";
const RUST_TY: &str = "bool char f32 f64 i8 i16 i32 i64 i128 isize str u8 u16 u32 u64 u128 usize String Vec Option Result Box Some None Ok Err";
const SQL_KW: &str = "add all alter and any as asc between by case cast check column commit constraint create cross database default delete desc distinct drop else end exists foreign from full group having in index inner insert into is join key left like limit merge not null on or order outer primary procedure references right rollback select set table then top truncate union unique update values view when where with begin declare exec execute function returns return if while over partition row_number count sum avg min max coalesce isnull nvarchar varchar int bigint datetime date char decimal numeric bit";
const PS_KW: &str = "begin break catch class continue data define do dynamicparam else elseif end exit filter finally for foreach from function if in param process return switch throw trap try until using var while";
const SH_KW: &str = "if then else elif fi case esac for while until do done in function select time return exit export local readonly declare unset shift echo cd source alias";
const BAT_KW: &str = "rem set if exist errorlevel for in do break call copy chcp cd chdir choice cls country ctty date del erase dir echo exit goto loadfix loadhigh mkdir md move path pause prompt rename ren rmdir rd shift time type ver verify vol com con lpt nul not defined else setlocal endlocal start";

fn style(sci: &Sci, id: usize, fore: u32) {
    sci.call(SCI_STYLESETFORE, id, rgb(fore));
}

fn bold(sci: &Sci, id: usize) {
    sci.call(SCI_STYLESETBOLD, id, 1);
}

/// 文書にレクサーを設定し、配色する（STYLE_DEFAULT のフォント・前景・背景は設定済みであること）
pub fn apply(sci: &Sci, lang: Lang, palette: &Palette) {
    sci.call(SCI_STYLECLEARALL, 0, 0);
    sci.call(SCI_STYLESETFORE, STYLE_LINENUMBER as usize, rgb(palette.margin_fg));
    sci.call(SCI_STYLESETBACK, STYLE_LINENUMBER as usize, rgb(palette.margin_bg));
    let lexer = match lang {
        Lang::Text | Lang::Log => "null",
        Lang::Json => "json",
        Lang::Cpp | Lang::JavaScript | Lang::CSharp | Lang::Java | Lang::Go => "cpp",
        Lang::Python => "python",
        Lang::Rust => "rust",
        Lang::Sql => "sql",
        Lang::Html => "hypertext",
        Lang::Xml => "xml",
        Lang::Css => "css",
        Lang::Markdown => "markdown",
        Lang::Yaml => "yaml",
        Lang::Ini => "props",
        Lang::Batch => "batch",
        Lang::PowerShell => "powershell",
        Lang::Shell => "bash",
        Lang::Diff => "diff",
    };
    sci.set_lexer(lexer);
    let kw = |set: usize, words: &str| {
        sci.call_str(SCI_SETKEYWORDS, set, words);
    };
    match lang {
        Lang::Json => {
            style(sci, SCE_JSON_NUMBER, palette.number);
            style(sci, SCE_JSON_STRING, palette.string);
            style(sci, SCE_JSON_STRINGEOL, palette.string);
            style(sci, SCE_JSON_PROPERTYNAME, palette.prop);
            style(sci, SCE_JSON_ESCAPESEQUENCE, palette.error);
            style(sci, SCE_JSON_LINECOMMENT, palette.comment);
            style(sci, SCE_JSON_BLOCKCOMMENT, palette.comment);
            style(sci, SCE_JSON_KEYWORD, palette.keyword);
            style(sci, SCE_JSON_LDKEYWORD, palette.keyword);
            style(sci, SCE_JSON_ERROR, palette.error);
            kw(0, "true false null");
            set_prop(sci, "lexer.json.allow.comments", "1");
            set_prop(sci, "lexer.json.escape.sequence", "1");
        }
        Lang::Cpp | Lang::JavaScript | Lang::CSharp | Lang::Java | Lang::Go => {
            for s in [SCE_C_COMMENT, SCE_C_COMMENTLINE, SCE_C_COMMENTDOC, SCE_C_COMMENTLINEDOC] {
                style(sci, s, palette.comment);
            }
            style(sci, SCE_C_NUMBER, palette.number);
            style(sci, SCE_C_WORD, palette.keyword);
            style(sci, SCE_C_WORD2, palette.type_);
            for s in [SCE_C_STRING, SCE_C_CHARACTER, SCE_C_VERBATIM, SCE_C_STRINGRAW, SCE_C_TRIPLEVERBATIM, SCE_C_HASHQUOTEDSTRING] {
                style(sci, s, palette.string);
            }
            style(sci, SCE_C_PREPROCESSOR, palette.preproc);
            style(sci, SCE_C_REGEX, palette.regex);
            kw(0, match lang {
                Lang::Cpp => CPP_KW,
                Lang::JavaScript => JS_KW,
                Lang::CSharp => CS_KW,
                Lang::Java => JAVA_KW,
                _ => GO_KW,
            });
        }
        Lang::Python => {
            style(sci, SCE_P_COMMENTLINE, palette.comment);
            style(sci, SCE_P_COMMENTBLOCK, palette.comment);
            style(sci, SCE_P_NUMBER, palette.number);
            for s in [SCE_P_STRING, SCE_P_CHARACTER, SCE_P_TRIPLE, SCE_P_TRIPLEDOUBLE, SCE_P_FSTRING, SCE_P_FCHARACTER] {
                style(sci, s, palette.string);
            }
            style(sci, SCE_P_WORD, palette.keyword);
            style(sci, SCE_P_CLASSNAME, palette.type_);
            style(sci, SCE_P_DEFNAME, palette.func);
            style(sci, SCE_P_DECORATOR, palette.preproc);
            kw(0, PY_KW);
        }
        Lang::Rust => {
            for s in [SCE_RUST_COMMENTBLOCK, SCE_RUST_COMMENTLINE, SCE_RUST_COMMENTBLOCKDOC, SCE_RUST_COMMENTLINEDOC] {
                style(sci, s, palette.comment);
            }
            style(sci, SCE_RUST_NUMBER, palette.number);
            style(sci, SCE_RUST_WORD, palette.keyword);
            style(sci, SCE_RUST_WORD2, palette.type_);
            for s in [SCE_RUST_STRING, SCE_RUST_STRINGR, SCE_RUST_CHARACTER, SCE_RUST_BYTESTRING] {
                style(sci, s, palette.string);
            }
            style(sci, SCE_RUST_MACRO, palette.func);
            style(sci, SCE_RUST_LIFETIME, palette.preproc);
            kw(0, RUST_KW);
            kw(1, RUST_TY);
        }
        Lang::Sql => {
            style(sci, SCE_SQL_COMMENT, palette.comment);
            style(sci, SCE_SQL_COMMENTLINE, palette.comment);
            style(sci, SCE_SQL_COMMENTDOC, palette.comment);
            style(sci, SCE_SQL_NUMBER, palette.number);
            style(sci, SCE_SQL_WORD, palette.keyword);
            style(sci, SCE_SQL_STRING, palette.string);
            style(sci, SCE_SQL_CHARACTER, palette.string);
            kw(0, SQL_KW);
        }
        Lang::Html | Lang::Xml => {
            style(sci, SCE_H_TAG, palette.tag);
            style(sci, SCE_H_TAGUNKNOWN, palette.tag);
            style(sci, SCE_H_TAGEND, palette.tag);
            style(sci, SCE_H_ATTRIBUTE, palette.attr);
            style(sci, SCE_H_ATTRIBUTEUNKNOWN, palette.attr);
            style(sci, SCE_H_DOUBLESTRING, palette.keyword);
            style(sci, SCE_H_SINGLESTRING, palette.keyword);
            style(sci, SCE_H_COMMENT, palette.comment);
            style(sci, SCE_H_NUMBER, palette.number);
            style(sci, SCE_H_ENTITY, palette.preproc);
            style(sci, SCE_H_XMLSTART, palette.preproc);
            style(sci, SCE_H_XMLEND, palette.preproc);
            style(sci, SCE_H_CDATA, palette.preproc);
        }
        Lang::Css => {
            style(sci, SCE_CSS_COMMENT, palette.comment);
            style(sci, SCE_CSS_TAG, palette.tag);
            style(sci, SCE_CSS_CLASS, palette.tag);
            style(sci, SCE_CSS_IDENTIFIER, palette.attr);
            style(sci, SCE_CSS_VALUE, palette.keyword);
            style(sci, SCE_CSS_DOUBLESTRING, palette.string);
            style(sci, SCE_CSS_SINGLESTRING, palette.string);
        }
        Lang::Markdown => {
            for s in [SCE_MARKDOWN_HEADER1, SCE_MARKDOWN_HEADER2, SCE_MARKDOWN_HEADER3, SCE_MARKDOWN_HEADER4, SCE_MARKDOWN_HEADER5, SCE_MARKDOWN_HEADER6] {
                style(sci, s, palette.keyword);
                bold(sci, s);
            }
            style(sci, SCE_MARKDOWN_STRONG1, palette.fg);
            bold(sci, SCE_MARKDOWN_STRONG1);
            style(sci, SCE_MARKDOWN_CODE, palette.string);
            style(sci, SCE_MARKDOWN_CODE2, palette.string);
            style(sci, SCE_MARKDOWN_CODEBK, palette.string);
            style(sci, SCE_MARKDOWN_LINK, palette.link);
            style(sci, SCE_MARKDOWN_BLOCKQUOTE, palette.comment);
            style(sci, SCE_MARKDOWN_ULIST_ITEM, palette.tag);
            style(sci, SCE_MARKDOWN_OLIST_ITEM, palette.tag);
        }
        Lang::Yaml => {
            style(sci, SCE_YAML_COMMENT, palette.comment);
            style(sci, SCE_YAML_IDENTIFIER, palette.prop);
            style(sci, SCE_YAML_KEYWORD, palette.keyword);
            style(sci, SCE_YAML_NUMBER, palette.number);
            style(sci, SCE_YAML_REFERENCE, palette.preproc);
            style(sci, SCE_YAML_DOCUMENT, palette.preproc);
            style(sci, SCE_YAML_TEXT, palette.string);
            style(sci, SCE_YAML_ERROR, palette.error);
            kw(0, "true false yes no null on off");
        }
        Lang::Ini => {
            style(sci, SCE_PROPS_COMMENT, palette.comment);
            style(sci, SCE_PROPS_SECTION, palette.keyword);
            bold(sci, SCE_PROPS_SECTION);
            style(sci, SCE_PROPS_ASSIGNMENT, palette.tag);
            style(sci, SCE_PROPS_KEY, palette.prop);
        }
        Lang::Batch => {
            style(sci, SCE_BAT_COMMENT, palette.comment);
            style(sci, SCE_BAT_WORD, palette.keyword);
            style(sci, SCE_BAT_LABEL, palette.tag);
            style(sci, SCE_BAT_IDENTIFIER, palette.attr);
            style(sci, SCE_BAT_COMMAND, palette.func);
            kw(0, BAT_KW);
        }
        Lang::PowerShell => {
            style(sci, SCE_POWERSHELL_COMMENT, palette.comment);
            style(sci, SCE_POWERSHELL_COMMENTSTREAM, palette.comment);
            style(sci, SCE_POWERSHELL_STRING, palette.string);
            style(sci, SCE_POWERSHELL_CHARACTER, palette.string);
            style(sci, SCE_POWERSHELL_HERE_STRING, palette.string);
            style(sci, SCE_POWERSHELL_NUMBER, palette.number);
            style(sci, SCE_POWERSHELL_VARIABLE, palette.attr);
            style(sci, SCE_POWERSHELL_KEYWORD, palette.keyword);
            style(sci, SCE_POWERSHELL_CMDLET, palette.func);
            kw(0, PS_KW);
        }
        Lang::Shell => {
            style(sci, SCE_SH_COMMENTLINE, palette.comment);
            style(sci, SCE_SH_NUMBER, palette.number);
            style(sci, SCE_SH_WORD, palette.keyword);
            style(sci, SCE_SH_STRING, palette.string);
            style(sci, SCE_SH_CHARACTER, palette.string);
            style(sci, SCE_SH_SCALAR, palette.attr);
            style(sci, SCE_SH_PARAM, palette.attr);
            kw(0, SH_KW);
        }
        Lang::Diff => {
            style(sci, SCE_DIFF_COMMENT, palette.comment);
            style(sci, SCE_DIFF_COMMAND, palette.keyword);
            style(sci, SCE_DIFF_HEADER, palette.tag);
            style(sci, SCE_DIFF_POSITION, palette.diff_pos);
            style(sci, SCE_DIFF_DELETED, palette.diff_deleted);
            style(sci, SCE_DIFF_ADDED, palette.diff_added);
        }
        Lang::Text | Lang::Log => {}
    }
}

fn set_prop(sci: &Sci, key: &str, val: &str) {
    let k = format!("{key}\0");
    let v = format!("{val}\0");
    sci.call(SCI_SETPROPERTY, k.as_ptr() as usize, v.as_ptr() as isize);
}
