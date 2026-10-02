//! 拡張子ごとのレクサーと配色

use std::path::Path;

use crate::lexer_consts::*;
use crate::sci::Sci;
use crate::sci_consts::*;
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

const FG: u32 = 0x000000;
const COMMENT: u32 = 0x008000;
const STRING: u32 = 0xA31515;
const KEYWORD: u32 = 0x0000FF;
const TYPE: u32 = 0x2B91AF;
const NUMBER: u32 = 0x098658;
const PREPROC: u32 = 0x808080;
const PROP: u32 = 0x0451A5;
const ERROR: u32 = 0xE00000;

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

/// 文書にレクサーを設定し、配色する（STYLE_DEFAULT のフォントは設定済みであること）
pub fn apply(sci: &Sci, lang: Lang) {
    sci.call(SCI_STYLECLEARALL, 0, 0);
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
            style(sci, SCE_JSON_NUMBER, NUMBER);
            style(sci, SCE_JSON_STRING, STRING);
            style(sci, SCE_JSON_STRINGEOL, STRING);
            style(sci, SCE_JSON_PROPERTYNAME, PROP);
            style(sci, SCE_JSON_ESCAPESEQUENCE, 0xEE0000);
            style(sci, SCE_JSON_LINECOMMENT, COMMENT);
            style(sci, SCE_JSON_BLOCKCOMMENT, COMMENT);
            style(sci, SCE_JSON_KEYWORD, KEYWORD);
            style(sci, SCE_JSON_LDKEYWORD, KEYWORD);
            style(sci, SCE_JSON_ERROR, ERROR);
            kw(0, "true false null");
            set_prop(sci, "lexer.json.allow.comments", "1");
            set_prop(sci, "lexer.json.escape.sequence", "1");
        }
        Lang::Cpp | Lang::JavaScript | Lang::CSharp | Lang::Java | Lang::Go => {
            for s in [SCE_C_COMMENT, SCE_C_COMMENTLINE, SCE_C_COMMENTDOC, SCE_C_COMMENTLINEDOC] {
                style(sci, s, COMMENT);
            }
            style(sci, SCE_C_NUMBER, NUMBER);
            style(sci, SCE_C_WORD, KEYWORD);
            style(sci, SCE_C_WORD2, TYPE);
            for s in [SCE_C_STRING, SCE_C_CHARACTER, SCE_C_VERBATIM, SCE_C_STRINGRAW, SCE_C_TRIPLEVERBATIM, SCE_C_HASHQUOTEDSTRING] {
                style(sci, s, STRING);
            }
            style(sci, SCE_C_PREPROCESSOR, PREPROC);
            style(sci, SCE_C_REGEX, 0x811F3F);
            kw(0, match lang {
                Lang::Cpp => CPP_KW,
                Lang::JavaScript => JS_KW,
                Lang::CSharp => CS_KW,
                Lang::Java => JAVA_KW,
                _ => GO_KW,
            });
        }
        Lang::Python => {
            style(sci, SCE_P_COMMENTLINE, COMMENT);
            style(sci, SCE_P_COMMENTBLOCK, COMMENT);
            style(sci, SCE_P_NUMBER, NUMBER);
            for s in [SCE_P_STRING, SCE_P_CHARACTER, SCE_P_TRIPLE, SCE_P_TRIPLEDOUBLE, SCE_P_FSTRING, SCE_P_FCHARACTER] {
                style(sci, s, STRING);
            }
            style(sci, SCE_P_WORD, KEYWORD);
            style(sci, SCE_P_CLASSNAME, TYPE);
            style(sci, SCE_P_DEFNAME, 0x795E26);
            style(sci, SCE_P_DECORATOR, PREPROC);
            kw(0, PY_KW);
        }
        Lang::Rust => {
            for s in [SCE_RUST_COMMENTBLOCK, SCE_RUST_COMMENTLINE, SCE_RUST_COMMENTBLOCKDOC, SCE_RUST_COMMENTLINEDOC] {
                style(sci, s, COMMENT);
            }
            style(sci, SCE_RUST_NUMBER, NUMBER);
            style(sci, SCE_RUST_WORD, KEYWORD);
            style(sci, SCE_RUST_WORD2, TYPE);
            for s in [SCE_RUST_STRING, SCE_RUST_STRINGR, SCE_RUST_CHARACTER, SCE_RUST_BYTESTRING] {
                style(sci, s, STRING);
            }
            style(sci, SCE_RUST_MACRO, 0x795E26);
            style(sci, SCE_RUST_LIFETIME, PREPROC);
            kw(0, RUST_KW);
            kw(1, RUST_TY);
        }
        Lang::Sql => {
            style(sci, SCE_SQL_COMMENT, COMMENT);
            style(sci, SCE_SQL_COMMENTLINE, COMMENT);
            style(sci, SCE_SQL_COMMENTDOC, COMMENT);
            style(sci, SCE_SQL_NUMBER, NUMBER);
            style(sci, SCE_SQL_WORD, KEYWORD);
            style(sci, SCE_SQL_STRING, STRING);
            style(sci, SCE_SQL_CHARACTER, STRING);
            kw(0, SQL_KW);
        }
        Lang::Html | Lang::Xml => {
            style(sci, SCE_H_TAG, 0x800000);
            style(sci, SCE_H_TAGUNKNOWN, 0x800000);
            style(sci, SCE_H_TAGEND, 0x800000);
            style(sci, SCE_H_ATTRIBUTE, 0xE50000);
            style(sci, SCE_H_ATTRIBUTEUNKNOWN, 0xE50000);
            style(sci, SCE_H_DOUBLESTRING, KEYWORD);
            style(sci, SCE_H_SINGLESTRING, KEYWORD);
            style(sci, SCE_H_COMMENT, COMMENT);
            style(sci, SCE_H_NUMBER, NUMBER);
            style(sci, SCE_H_ENTITY, PREPROC);
            style(sci, SCE_H_XMLSTART, PREPROC);
            style(sci, SCE_H_XMLEND, PREPROC);
            style(sci, SCE_H_CDATA, PREPROC);
        }
        Lang::Css => {
            style(sci, SCE_CSS_COMMENT, COMMENT);
            style(sci, SCE_CSS_TAG, 0x800000);
            style(sci, SCE_CSS_CLASS, 0x800000);
            style(sci, SCE_CSS_IDENTIFIER, 0xE50000);
            style(sci, SCE_CSS_VALUE, KEYWORD);
            style(sci, SCE_CSS_DOUBLESTRING, STRING);
            style(sci, SCE_CSS_SINGLESTRING, STRING);
        }
        Lang::Markdown => {
            for s in [SCE_MARKDOWN_HEADER1, SCE_MARKDOWN_HEADER2, SCE_MARKDOWN_HEADER3, SCE_MARKDOWN_HEADER4, SCE_MARKDOWN_HEADER5, SCE_MARKDOWN_HEADER6] {
                style(sci, s, KEYWORD);
                bold(sci, s);
            }
            style(sci, SCE_MARKDOWN_STRONG1, FG);
            bold(sci, SCE_MARKDOWN_STRONG1);
            style(sci, SCE_MARKDOWN_CODE, STRING);
            style(sci, SCE_MARKDOWN_CODE2, STRING);
            style(sci, SCE_MARKDOWN_CODEBK, STRING);
            style(sci, SCE_MARKDOWN_LINK, 0x0066CC);
            style(sci, SCE_MARKDOWN_BLOCKQUOTE, COMMENT);
            style(sci, SCE_MARKDOWN_ULIST_ITEM, 0x800000);
            style(sci, SCE_MARKDOWN_OLIST_ITEM, 0x800000);
        }
        Lang::Yaml => {
            style(sci, SCE_YAML_COMMENT, COMMENT);
            style(sci, SCE_YAML_IDENTIFIER, PROP);
            style(sci, SCE_YAML_KEYWORD, KEYWORD);
            style(sci, SCE_YAML_NUMBER, NUMBER);
            style(sci, SCE_YAML_REFERENCE, PREPROC);
            style(sci, SCE_YAML_DOCUMENT, PREPROC);
            style(sci, SCE_YAML_TEXT, STRING);
            style(sci, SCE_YAML_ERROR, ERROR);
            kw(0, "true false yes no null on off");
        }
        Lang::Ini => {
            style(sci, SCE_PROPS_COMMENT, COMMENT);
            style(sci, SCE_PROPS_SECTION, KEYWORD);
            bold(sci, SCE_PROPS_SECTION);
            style(sci, SCE_PROPS_ASSIGNMENT, 0x800000);
            style(sci, SCE_PROPS_KEY, PROP);
        }
        Lang::Batch => {
            style(sci, SCE_BAT_COMMENT, COMMENT);
            style(sci, SCE_BAT_WORD, KEYWORD);
            style(sci, SCE_BAT_LABEL, 0x800000);
            style(sci, SCE_BAT_IDENTIFIER, 0xE50000);
            style(sci, SCE_BAT_COMMAND, 0x795E26);
            kw(0, BAT_KW);
        }
        Lang::PowerShell => {
            style(sci, SCE_POWERSHELL_COMMENT, COMMENT);
            style(sci, SCE_POWERSHELL_COMMENTSTREAM, COMMENT);
            style(sci, SCE_POWERSHELL_STRING, STRING);
            style(sci, SCE_POWERSHELL_CHARACTER, STRING);
            style(sci, SCE_POWERSHELL_HERE_STRING, STRING);
            style(sci, SCE_POWERSHELL_NUMBER, NUMBER);
            style(sci, SCE_POWERSHELL_VARIABLE, 0xE50000);
            style(sci, SCE_POWERSHELL_KEYWORD, KEYWORD);
            style(sci, SCE_POWERSHELL_CMDLET, 0x795E26);
            kw(0, PS_KW);
        }
        Lang::Shell => {
            style(sci, SCE_SH_COMMENTLINE, COMMENT);
            style(sci, SCE_SH_NUMBER, NUMBER);
            style(sci, SCE_SH_WORD, KEYWORD);
            style(sci, SCE_SH_STRING, STRING);
            style(sci, SCE_SH_CHARACTER, STRING);
            style(sci, SCE_SH_SCALAR, 0xE50000);
            style(sci, SCE_SH_PARAM, 0xE50000);
            kw(0, SH_KW);
        }
        Lang::Diff => {
            style(sci, SCE_DIFF_COMMENT, COMMENT);
            style(sci, SCE_DIFF_COMMAND, KEYWORD);
            style(sci, SCE_DIFF_HEADER, 0x800000);
            style(sci, SCE_DIFF_POSITION, 0x800080);
            style(sci, SCE_DIFF_DELETED, 0xC00000);
            style(sci, SCE_DIFF_ADDED, 0x008000);
        }
        Lang::Text | Lang::Log => {}
    }
}

fn set_prop(sci: &Sci, key: &str, val: &str) {
    let k = format!("{key}\0");
    let v = format!("{val}\0");
    sci.call(SCI_SETPROPERTY, k.as_ptr() as usize, v.as_ptr() as isize);
}
