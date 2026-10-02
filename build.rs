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

    resources();

    for lib in ["user32", "gdi32", "imm32", "ole32", "oleaut32", "advapi32", "uuid", "msimg32", "comctl32", "shell32", "comdlg32", "winhttp", "bcrypt"] {
        println!("cargo:rustc-link-lib={lib}");
    }
    // ビジュアルスタイル(コモンコントロール v6) と DPI 対応のマニフェストを埋め込む
    println!("cargo:rustc-link-arg-bins=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg-bins=/MANIFESTDEPENDENCY:type='win32' name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'");
    println!("cargo:rerun-if-changed=cpp/lexers.cpp");
    println!("cargo:rerun-if-changed=build.rs");
}

/// アイコンとバージョン情報を exe に埋め込む
fn resources() {
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let ico = std::fs::canonicalize("assets/sakura2.ico").unwrap();
    let ver = env!("CARGO_PKG_VERSION");
    let nums: Vec<&str> = ver.split(['.', '-']).take(3).collect();
    let comma = format!("{},0", nums.join(","));
    let rc = format!(
        r#"#pragma code_page(65001)
1 ICON "{ico}"
1 VERSIONINFO
FILEVERSION {comma}
PRODUCTVERSION {comma}
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "041104B0"
    BEGIN
      VALUE "CompanyName", "mazume-tech-club"
      VALUE "FileDescription", "Sakura2 - 高速起動のログ向けエディタ"
      VALUE "FileVersion", "{ver}"
      VALUE "ProductName", "Sakura2"
      VALUE "ProductVersion", "{ver}"
      VALUE "OriginalFilename", "sakura2.exe"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0411, 1200
  END
END
"#,
        ico = ico.display().to_string().trim_start_matches(r"\\?\").replace('\\', "\\\\"),
    );
    let rc_path = out.join("sakura2.rc");
    std::fs::write(&rc_path, rc).unwrap();
    embed_resource::compile(&rc_path, embed_resource::NONE).manifest_optional().unwrap();
    println!("cargo:rerun-if-changed=assets/sakura2.ico");
}
