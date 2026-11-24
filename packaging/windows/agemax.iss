; Inno Setup 6 Installer Script for Age Max
; Dual-licensed MIT OR Apache-2.0
; Build with: iscc packaging\windows\agemax.iss

#define MyAppName "Age Max"
#define MyAppVersion "0.1.0"
#define MyAppPublisher "Age Max Contributors"
#define MyAppURL "https://github.com/hatimbenayad/Agemax"
#define MyAppExeName "agemax-app.exe"
#define MyCliExeName "agemax.exe"

[Setup]
AppId={{8F64D4B5-4C2B-4E1C-92F2-9B3C7E4A10F8}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\Programs\AgeMax
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
LicenseFile=..\..\LICENSE
OutputDir=..\..\target\installer
OutputBaseFilename=AgeMax-Setup-{#MyAppVersion}-x64
SetupIconFile=..\..\assets\agemax.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequired=lowest
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "addtopath"; Description: "Add agemax CLI tool to user PATH"; GroupDescription: "Developer Integration"
Name: "explorermenu"; Description: "Add ""Open Folder in Age Max"" to Windows Explorer context menu"; GroupDescription: "Explorer Integration"

[Files]
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\target\release\{#MyCliExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\agemax.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "open-in-agemax-install.ps1"; DestDir: "{app}"; Flags: ignoreversion
Source: "open-in-agemax-uninstall.ps1"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\agemax.ico"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\agemax.ico"; Tasks: desktopicon

[Registry]
; Explorer folder context menu (per-user HKCU)
Root: HKCU; Subkey: "Software\Classes\Directory\shell\AgeMax"; ValueType: string; ValueData: "Open Folder in Age Max"; Tasks: explorermenu; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\shell\AgeMax"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#MyAppExeName}"",0"; Tasks: explorermenu
Root: HKCU; Subkey: "Software\Classes\Directory\shell\AgeMax\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%1"""; Tasks: explorermenu

Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\AgeMax"; ValueType: string; ValueData: "Open Folder in Age Max"; Tasks: explorermenu; Flags: uninsdeletekey
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\AgeMax"; ValueType: string; ValueName: "Icon"; ValueData: """{app}\{#MyAppExeName}"",0"; Tasks: explorermenu
Root: HKCU; Subkey: "Software\Classes\Directory\Background\shell\AgeMax\command"; ValueType: string; ValueData: """{app}\{#MyAppExeName}"" ""%V"""; Tasks: explorermenu

[Code]
procedure AddToUserPath(DirToAdd: string);
var
  CurrentPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', CurrentPath) then
    CurrentPath := '';
  if Pos(';' + Uppercase(DirToAdd) + ';', ';' + Uppercase(CurrentPath) + ';') = 0 then
  begin
    if (CurrentPath <> '') and (CurrentPath[Length(CurrentPath)] <> ';') then
      CurrentPath := CurrentPath + ';';
    CurrentPath := CurrentPath + DirToAdd;
    RegWriteExpandStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', CurrentPath);
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('addtopath') then
    AddToUserPath(ExpandConstant('{app}'));
end;

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram,{#StringChange(MyAppName, '&', '&&')}}"; Flags: nowait postinstall skipifsilent
