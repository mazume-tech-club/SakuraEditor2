# Sakura2

<img src="assets/sakura2.png" width="96" alt="Sakura2 アイコン">

さくらエディタ風の、**起動が速い**ログ向けテキストエディタ（Windows 専用）。

- Rust + Win32 API 直叩き、編集部品は [Scintilla](https://www.scintilla.org/) を静的リンク。exe 1 本（約 3MB、VC++ ランタイム不要）
- 起動して画面表示まで約 80ms（素の Win32 ウィンドウを出すだけでも 45〜70ms かかる環境での実測値）
- 100MB / 127 万行のログを約 0.3 秒で開き、正規表現フィルタは約 80ms

## インストール

[Releases](https://github.com/mazume-tech-club/SakuraEditor2/releases/latest) から `sakura2-setup-<バージョン>.exe` をダウンロードして実行する。

- 導入先: `%LOCALAPPDATA%\Programs\Sakura2`（ユーザー単位。管理者権限は不要）
- スタートメニューに登録する。デスクトップのショートカットは任意
- 上書きインストールでそのまま更新できる。起動中の Sakura2 はセットアップが閉じる
- サイレントインストール: `sakura2-setup-<バージョン>.exe /VERYSILENT /SUPPRESSMSGBOXES`

インストーラーを使わずに、`sakura2.exe` 単体を書き込み可能な好きなフォルダに置いて使ってもよい（自動アップデートは exe を置き換えるので、書き込み権限が必要）。

### アンインストール

「設定 > アプリ > インストールされているアプリ」で Sakura2 を選んで削除する。

- exe、ショートカット、エクスプローラーの右クリックメニューを削除する
- 設定（`%APPDATA%\Sakura2`）と更新キャッシュ・ログ（`%LOCALAPPDATA%\Sakura2`）は、削除するかどうかを確認する。サイレントアンインストール（`unins000.exe /VERYSILENT`）のときは残す
- 自動アップデートで新しくなった後も、「インストールされているアプリ」に出るバージョンはインストール時のまま（表示だけの問題）

## 主な機能

| 機能 | 操作 |
|------|------|
| JSON 整形（VSCode と同じキー） | `Alt+Shift+F`（選択範囲／全体／JSON Lines／ログ行に埋め込まれた JSON） |
| JSON 圧縮 | `Alt+Shift+M` |
| ログフィルタ パネル | `Ctrl+Shift+L` |
| フィルタ実行 → `C:\Windows\Temp` に出力 | パネルで `Enter`／`Ctrl+Shift+Enter` |
| 抽出結果から元の行へジャンプ | ダブルクリック／`F12` |
| ファイル追従（tail -f） | `Ctrl+Shift+T` |
| Vi モード切替 | `Ctrl+Alt+V` |
| Vi チートシート | `F1` |
| 絞り込み検索（一致行だけ表示＝Grep 表示） | `Ctrl+F`（入力するとその場で絞り込み、`Esc` か空欄で `Enter` で解除、`Ctrl+Enter` で Temp に出力） |
| 次／前の一致、置換、行移動 | `F3`／`Shift+F3`、`Ctrl+H`、`Ctrl+G` |
| エクスプローラーの右クリック | 「さくらエディタ2 で開く」（初回起動時に自動登録。設定メニューで ON/OFF。Windows 11 は「その他のオプションを確認」の中） |

### 絞り込み検索（Ctrl+F）

- 入力すると、一致しない行を隠して一致行だけを表示する。行番号は元のファイルのまま
- 100MB / 127 万行のログでも 0.2 秒前後で絞り込める
- `Enter` で確定して編集画面に戻る（絞り込みは維持）。`Esc`、または空欄で `Enter` を押すと解除
- tail 中なら、追記された行にも同じ絞り込みを適用する
- 複数条件や除外条件が必要なときは、下のログフィルタを使う

### ログフィルタ

- 条件は最大 8 行。各行で「除外」「大小区別(Aa)」「正規表現(.*)」を切り替えられる
- 包含条件のどれかに一致し、除外条件のどれにも一致しない行が残る
- 入力中は一致件数をライブ表示し、エディタ上でも条件ごとの色でハイライトする（● をクリックで色変更）
- 実行すると `C:\Windows\Temp\sakura2_filter_<元ファイル名>_<日時>.log` を作って新しいタブで開く（書き込めない場合は `%TEMP%`）。7 日より古い出力は自動で削除する
- 「行番号を付ける」にチェックを入れると `L123: ` を行頭に付ける
- 抽出結果をさらに抽出しても、元ファイルの行番号に遡ってジャンプできる
- 元ファイルを tail 中なら、新しく一致した行が抽出結果タブと出力ファイルにも追記される
- ERROR / WARN 行は背景色で自動的に色分けする（表示 > ログレベルの色分け）

### Vi モード

設定 > Vi モード（`Ctrl+Alt+V`）で切り替える。普段の操作に戻したいときは `:vi`。

- ノーマル／挿入／ビジュアル（文字・行）モード、カウント、オペレータ + モーション、テキストオブジェクト（`ciw` `di"` `da(` など）、`.` リピート、`u` / `Ctrl+R`
- `/` `?` `n` `N` `*` `#` 検索（正規表現は VSCode と同じ書式、smartcase）
- `:w` `:q` `:wq` `:e` `:%s/a/b/g` `:'<,'>s/…` `:g/re/d` `:v/re/d` `:sort u` `:set nu` など
- **`:g/ERROR/` で一致行を Temp に抽出、`:v/DEBUG/` で非一致行を抽出**（ログフィルタと連動）
- ヤンクは Windows クリップボードと連動
- 学習用ヒント: 押したコマンドの意味をステータスバーに表示（例: `dw … 次の単語の先頭まで削除`）。入力途中なら次に押せるキーを案内する
- ノーマルモードでは IME を自動で OFF にする

## 自動アップデート

起動 5 秒後に、バックグラウンドで GitHub Releases の最新版を確認する（24 時間に 1 回）。

1. `https://api.github.com/repos/mazume-tech-club/SakuraEditor2/releases/latest` を取得
2. 現在のバージョンより新しければ `sakura2.exe` と `sakura2.exe.sha256` をダウンロード
3. SHA-256 を検証し、実行中の exe を `sakura2.old.exe` に退避して新しい exe を置く
4. ステータスバーに通知。次回起動（またはヘルプ > 再起動）で新バージョンになる

- 通信は OS 標準の WinHTTP を使うので、Windows のプロキシ設定がそのまま効く
- ログ: `%LOCALAPPDATA%\Sakura2\update.log`
- 手動確認: ヘルプ > アップデートを確認
- 無効化: 設定 > 自動アップデート
- プライベートリポジトリの場合は設定ファイルの `github_token=` にトークンを書く

### リリース手順

```powershell
# Cargo.toml の version を上げてから
git commit -am "v0.2.0"
git tag v0.2.0
git push origin main --tags
```

`.github/workflows/release.yml` がテスト・ビルドし、`sakura2.exe`、`sakura2.exe.sha256`、インストーラー `sakura2-setup-<バージョン>.exe` を Release に添付する。

## 設定

`%APPDATA%\Sakura2\config.ini`（設定 > 設定ファイルを開く）。保存すると即座に反映する。

| キー | 既定値 | 内容 |
|------|--------|------|
| `font_name` / `font_size` | BIZ UDゴシック / 11 | フォント |
| `tab_width` | 4 | タブ幅 |
| `json_indent` | 4 | JSON 整形のインデント（0 でタブ） |
| `wrap` / `line_numbers` | 0 / 1 | 折り返し／行番号 |
| `log_levels` | 1 | ERROR/WARN 行の色分け |
| `directwrite` | 0 | 1 で DirectWrite 描画（起動は少し遅くなる） |
| `vi_mode` / `vi_hints` | 0 / 1 | Vi モード／学習ヒント |
| `auto_update` | 1 | 自動アップデート |
| `update_repo` | mazume-tech-club/SakuraEditor2 | 更新元リポジトリ |
| `github_token` | (空) | プライベートリポジトリ用 |

## ビルド

必要なもの: Rust (stable, MSVC)、Visual Studio Build Tools（C++）

```powershell
cargo test
cargo build --release   # target\release\sakura2.exe
```

インストーラー: [Inno Setup 6](https://jrsoftware.org/isinfo.php) を入れて、ビルド後に次を実行する（`dist\sakura2-setup-<バージョン>.exe` ができる）。

```powershell
& "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe" /DAppVersion=0.1.3 installer\sakura2.iss
```

起動時間の計測: `sakura2.exe --bench-exit` を実行すると `%LOCALAPPDATA%\Sakura2\bench.log` に内訳を追記して終了する。

## 構成

| パス | 役割 |
|------|------|
| `src/app.rs` | メインウィンドウ、タブ、メニュー、フィルタ・tail・Vi の結線 |
| `src/sci.rs` | Scintilla の薄いラッパ（ダイレクト関数呼び出し） |
| `src/doc.rs` | 文字コード判定（UTF-8 / BOM / SJIS / UTF-16）と改行判定 |
| `src/json_fmt.rs` | JSON 整形・圧縮 |
| `src/filter/` | フィルタエンジン（RegexSet + 並列）、パネル UI、表示範囲ハイライト、Temp 出力 |
| `src/tail.rs` | ファイル追従 |
| `src/vi/` | Vi エンジン（Scintilla 非依存でテスト可能）、Ex コマンド、ヒント |
| `src/update.rs` | GitHub Releases からの自動更新（WinHTTP + BCrypt SHA-256） |
| `installer/sakura2.iss` | インストーラー／アンインストーラー（Inno Setup 6） |
| `assets/` `tools/make_icon.py` | 桜アイコン（`python tools/make_icon.py` で再生成） |
| `vendor/` | Scintilla 5.5.7 / Lexilla 5.4.5 のソース（使用部分のみ） |

## ライセンス

Scintilla / Lexilla は各ディレクトリの `License.txt` に従う。
