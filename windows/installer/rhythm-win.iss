#define AppName "Rhythm"
#define AppExeName "rhythm-win.exe"
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#define Publisher "yanyue404"

[Setup]
AppId={{8A6272A2-248F-48BD-B89E-7C2F2B0CD95E}
AppName={#AppName}
AppVersion={#AppVersion}
AppPublisher={#Publisher}
DefaultDirName={autopf}\Rhythm
DefaultGroupName=Rhythm
UninstallDisplayIcon={app}\{#AppExeName}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64
ArchitecturesInstallIn64BitMode=x64
OutputDir=..\..\dist
OutputBaseFilename=RhythmWin-Setup-v{#AppVersion}

[Languages]
Name: "chinesesimplified"; MessagesFile: "compiler:Languages\ChineseSimplified.isl"

[Tasks]
Name: "desktopicon"; Description: "创建桌面快捷方式"; GroupDescription: "附加任务:"
Name: "autostart"; Description: "安装后启用开机启动"; GroupDescription: "附加任务:"

[Files]
Source: "..\target\release\{#AppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\Rhythm"; Filename: "{app}\{#AppExeName}"
Name: "{autodesktop}\Rhythm"; Filename: "{app}\{#AppExeName}"; Tasks: desktopicon

[Run]
Filename: "{app}\{#AppExeName}"; Description: "启动 Rhythm"; Flags: nowait postinstall skipifsilent
Filename: "reg.exe"; Parameters: "add HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v RhythmWin /t REG_SZ /d """"{app}\{#AppExeName}"""" /f"; Tasks: autostart; Flags: runhidden

[UninstallRun]
Filename: "reg.exe"; Parameters: "delete HKCU\Software\Microsoft\Windows\CurrentVersion\Run /v RhythmWin /f"; Flags: runhidden
