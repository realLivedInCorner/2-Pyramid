; 2-Pyramid Inno Setup 安装脚本
;
; 由 tools/build_release.py 调用 ISCC.exe 编译：
;   ISCC.exe /DMyAppVersion=2.0.0 /DStagingDir=..\release\staging installer.iss
;
; 特性：
;   * 每用户安装（无需管理员权限）
;   * 中英双语向导（跟随系统语言，可在向导中切换）
;   * 自定义安装目录、桌面/开始菜单快捷方式（可勾选）
;   * .zip 右键菜单「转换版本至(C)」注册与卸载清理
;   * /VERYSILENT 静默安装（自动更新器使用）
;   * 安装时检测运行中的 2-Pyramid 并提示关闭

#ifndef MyAppVersion
  #define MyAppVersion "2.0.0"
#endif
#ifndef StagingDir
  #define StagingDir "..\release\staging"
#endif

#define MyAppName "2-Pyramid"
#define MyAppPublisher "2-Pyramid Studio"
#define MyAppExeName "2-pyramid.exe"

[Setup]
AppId={{9F3E5B72-8C41-4E6A-9A2B-73C1D4E5F607}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppVerName={#MyAppName} {#MyAppVersion}
DefaultDirName={autopf}\2-Pyramid
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
; 每用户安装，免管理员；管理员可用 /ALLUSERS 覆盖
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputBaseFilename=2-Pyramid-Setup-{#MyAppVersion}
OutputDir=..\release
SetupIconFile=..\src-tauri\icons\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; 静默安装时自动关闭应用（更新器依赖）
CloseApplications=yes
RestartApplications=no
; 安装器语言：中文优先 + 英文
ShowLanguageDialog=auto
UsePreviousAppDir=yes

[Languages]
Name: "chinesesimplified"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "startmenuicon"; Description: "{cm:CreateStartMenuIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "rightclick"; Description: "为 .zip 文件添加「转换版本至(C)」右键菜单"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#StagingDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StagingDir}\two_pyramid_shell.dll"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#StagingDir}\UImage\*"; DestDir: "{app}\UImage"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#StagingDir}\overlay\*"; DestDir: "{app}\overlay"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: startmenuicon
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon

; .zip 右键菜单（当前用户）
[Registry]
Root: HKCU; Subkey: "Software\Classes\.zip\shell\2-Pyramid"; ValueType: string; ValueData: "转换版本至(&C)"; Flags: uninsdeletekey; Tasks: rightclick
Root: HKCU; Subkey: "Software\Classes\.zip\shell\2-Pyramid"; ValueType: string; ValueName: "Icon"; ValueData: "{app}\{#MyAppExeName},0"; Tasks: rightclick
Root: HKCU; Subkey: "Software\Classes\.zip\shell\2-Pyramid\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" --convert ""%1"""; Tasks: rightclick

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#MyAppName}}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
Type: filesandordirs; Name: "{app}\UImage"
Type: filesandordirs; Name: "{app}\overlay"

[Code]
// 安装前检测运行中的 2-Pyramid（静默更新时由 CloseApplications 处理，
// 交互安装时提示用户关闭）
function InitializeSetup(): Boolean;
begin
  Result := True;
end;

function InitializeUninstall(): Boolean;
begin
  Result := True;
end;
