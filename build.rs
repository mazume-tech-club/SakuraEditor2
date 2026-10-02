// Scintilla / Lexilla をソースから静的ライブラリとしてビルドし、exe に同梱する
use std::path::Path;

const LEXERS: &[&str] = &[
    "LexJSON", "LexCPP", "LexPython", "LexProps", "LexBatch", "LexSQL", "LexHTML",
    "LexMarkdown", "LexYAML", "LexPowerShell", "LexRust", "LexCSS", "LexBash", "LexDiff",
    "LexNull",
];

fn cxx_files(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut v: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "cxx"))
        .collect();
    v.sort();
    v
}

fn base() -> cc::Build {
    let mut b = cc::Build::new();
    b.cpp(true)
        .std("c++17")
        .flag_if_supported("/EHsc")
        .flag_if_supported("/utf-8")
        .flag_if_supported("/W0")
        .define("NDEBUG", None)
        .warnings(false);
    b
}

fn main() {
    let sci = Path::new("vendor/scintilla");
    let lex = Path::new("vendor/lexilla");

    let mut b = base();
    b.include(sci.join("include")).include(sci.join("src"));
    b.files(cxx_files(&sci.join("src")));
    for f in ["HanjaDic", "PlatWin", "ListBox", "SurfaceGDI", "SurfaceD2D", "ScintillaWin"] {
        b.file(sci.join("win32").join(format!("{f}.cxx")));
    }
    b.include(sci.join("win32"));
    b.compile("scintilla");

    let mut l = base();
    l.include(sci.join("include")).include(lex.join("include")).include(lex.join("lexlib"));
    l.files(cxx_files(&lex.join("lexlib")));
    for f in LEXERS {
        l.file(lex.join("lexers").join(format!("{f}.cxx")));
    }
    l.file("cpp/lexers.cpp");
    l.compile("lexilla");

    for lib in ["user32", "gdi32", "imm32", "ole32", "oleaut32", "advapi32", "uuid", "msimg32", "comctl32", "shell32", "comdlg32", "winhttp", "bcrypt"] {
        println!("cargo:rustc-link-lib={lib}");
    }
    // ビジュアルスタイル(コモンコントロール v6) と DPI 対応のマニフェストを埋め込む
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bins=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    println!("cargo:rerun-if-changed=cpp/lexers.cpp");
    println!("cargo:rerun-if-changed=build.rs");
}
