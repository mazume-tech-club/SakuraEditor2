; Sakura2 のインストーラー (Inno Setup 6)。
; ビルド: ISCC.exe /DAppVersion=0.1.3 installer\sakura2.iss  → dist\sakura2-setup-0.1.3.exe
; 事前に cargo build --release で target\release\sakura2.exe を作っておく。
;
; 自動アップデート (src/update.rs) は実行中の exe を置き換えるため、
; 管理者権限の要らないユーザー単位のフォルダ (%LOCALAPPDATA%\Programs\Sakura2) に入れる。

#ifndef AppVersion
  #error "/DAppVersion=x.y.z を指定してください (Cargo.toml の version と同じ値)"
#endif

; VersionInfoVersion は数字 (x.y.z) しか書けないので、0.2.0-rc1 や 0.2.0+build の後ろを落とす
#define NumVersion AppVersion
#if Pos("-", NumVersion) > 0
  #define NumVersion Copy(NumVersion, 1, Pos("-", NumVersion) - 1)
#endif
#if Pos("+", NumVersion) > 0
  #define NumVersion Copy(NumVersion, 1, Pos("+", NumVersion) - 1)
#endif

#define AppName "Sakura2"
; 画面に出す名前 (src/app.rs の APP_NAME と同じ)。フォルダ・レジストリ・設定の場所は AppName (Sakura2) のまま
#define AppDisplayName "さくらえでぃた弐"
#define AppExe "sakura2.exe"
; src/shell.rs の KEY と同じ。アプリが初回起動時に作る右クリックメニュー
#define ShellKey "Software\Classes\*\shell\Sakura2"
; src/app.rs のウィンドウクラス名
#define WindowClass "Sakura2Main"

[Setup]
AppId={{B980AE8F-3EC5-4D91-BD49-87B6838A437E}
AppName={#AppDisplayName}
AppVersion={#AppVersion}
AppVerName={#AppDisplayName} {#AppVersion}
AppPublisher=mazume-tech-club
AppPublisherURL=https://github.com/mazume-tech-club/SakuraEditor2
AppSupportURL=https://github.com/mazume-tech-club/SakuraEditor2/issues
AppUpdatesURL=https://github.com/mazume-tech-club/SakuraEditor2/releases
VersionInfoVersion={#NumVersion}
VersionInfoProductTextVersion={#AppVersion}
VersionInfoDescription={#AppDisplayName} セットアップ
PrivilegesRequired=lowest
DefaultDirName={localappdata}\Programs\{#AppName}
DisableProgramGroupPage=yes
DefaultGroupName={#AppName}
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\dist
OutputBaseFilename=sakura2-setup-{#AppVersion}
SetupIconFile=..\assets\sakura2.ico
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppDisplayName}
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD_PARTY_NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion

[InstallDelete]
; v0.1.4 以前のショートカット名 (Sakura2.lnk) を消す
Type: files; Name: "{group}\{#AppName}.lnk"
Type: files; Name: "{userdesktop}\{#AppName}.lnk"

[Icons]
Name: "{group}\{#AppDisplayName}"; Filename: "{app}\{#AppExe}"
Name: "{userdesktop}\{#AppDisplayName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppDisplayName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; 自動アップデートが退避した旧 exe
Type: files; Name: "{app}\sakura2.old.exe"
Type: dirifempty; Name: "{app}"

[Code]
// アンインストール前に起動中のアプリを閉じてもらう
function InitializeUninstall(): Boolean;
begin
  Result := True;
  while FindWindowByClassName('{#WindowClass}') <> 0 do
  begin
    if SuppressibleMsgBox('{#AppDisplayName} が起動しています。すべてのウィンドウを閉じてから [再試行] を押してください。',
                          mbError, MB_RETRYCANCEL, IDCANCEL) = IDCANCEL then
    begin
      Result := False;
      Exit;
    end;
  end;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  case CurUninstallStep of
    usUninstall:
      // 右クリックメニューはアプリが作るので、インストール記録に関係なく常に消す
      RegDeleteKeyIncludingSubkeys(HKEY_CURRENT_USER, '{#ShellKey}');
    usPostUninstall:
      // 設定と更新キャッシュは確認して消す (サイレント時は残す)
      if not UninstallSilent and
         (MsgBox('設定ファイルと更新キャッシュも削除しますか?' #13#10#13#10 +
                 ExpandConstant('{userappdata}\Sakura2') + #13#10 +
                 ExpandConstant('{localappdata}\Sakura2') + #13#10#13#10 +
                 '[いいえ] を選ぶと設定を残し、再インストール時に引き継ぎます。',
                 mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES) then
      begin
        DelTree(ExpandConstant('{userappdata}\Sakura2'), True, True, True);
        DelTree(ExpandConstant('{localappdata}\Sakura2'), True, True, True);
      end;
  end;
end;
