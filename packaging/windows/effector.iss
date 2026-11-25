; Inno Setup script for the EFFECTOR Windows installer.
; Built by .github/workflows/release.yml:  iscc /DAppVersion=x.y.z packaging\windows\effector.iss

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6B0C3E52-6B54-4C49-9C2E-EFFEC7000001}
AppName=EFFECTOR
AppVersion={#AppVersion}
AppVerName=EFFECTOR {#AppVersion}
AppPublisher=EFFECTOR
DefaultDirName={autopf}\EFFECTOR
DefaultGroupName=EFFECTOR
DisableProgramGroupPage=yes
OutputDir=..\..\dist
OutputBaseFilename=EFFECTOR-{#AppVersion}-Windows-Setup
SetupIconFile=..\..\assets\icons\icon.ico
UninstallDisplayIcon={app}\EFFECTOR.exe
Compression=lzma2/ultra64
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequiredOverridesAllowed=dialog
ChangesAssociations=yes
MinVersion=10.0

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\..\target\release\effector.exe"; DestDir: "{app}"; DestName: "EFFECTOR.exe"; Flags: ignoreversion
Source: "..\..\assets\fonts\OFL-*.txt"; DestDir: "{app}\licenses"; Flags: ignoreversion
Source: "..\..\assets\fonts\LICENSE-Phosphor.txt"; DestDir: "{app}\licenses"; Flags: ignoreversion

[Icons]
Name: "{group}\EFFECTOR"; Filename: "{app}\EFFECTOR.exe"
Name: "{autodesktop}\EFFECTOR"; Filename: "{app}\EFFECTOR.exe"; Tasks: desktopicon

[Registry]
; .effector projects open in EFFECTOR
Root: HKA; Subkey: "Software\Classes\.effector"; ValueType: string; ValueName: ""; ValueData: "EFFECTOR.Project"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\EFFECTOR.Project"; ValueType: string; ValueName: ""; ValueData: "EFFECTOR Project"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\EFFECTOR.Project\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\EFFECTOR.exe,0"
Root: HKA; Subkey: "Software\Classes\EFFECTOR.Project\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\EFFECTOR.exe"" ""%1"""
; "Open with EFFECTOR" for images
Root: HKA; Subkey: "Software\Classes\Applications\EFFECTOR.exe\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\EFFECTOR.exe"" ""%1"""; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\EFFECTOR.exe\SupportedTypes"; ValueType: string; ValueName: ".jpg"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\EFFECTOR.exe\SupportedTypes"; ValueType: string; ValueName: ".jpeg"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\EFFECTOR.exe\SupportedTypes"; ValueType: string; ValueName: ".png"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\EFFECTOR.exe\SupportedTypes"; ValueType: string; ValueName: ".tif"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\EFFECTOR.exe\SupportedTypes"; ValueType: string; ValueName: ".tiff"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\EFFECTOR.exe\SupportedTypes"; ValueType: string; ValueName: ".heic"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\EFFECTOR.exe\SupportedTypes"; ValueType: string; ValueName: ".webp"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\EFFECTOR.exe\SupportedTypes"; ValueType: string; ValueName: ".dng"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\EFFECTOR.exe\SupportedTypes"; ValueType: string; ValueName: ".effector"; ValueData: ""

[Run]
Filename: "{app}\EFFECTOR.exe"; Description: "{cm:LaunchProgram,EFFECTOR}"; Flags: nowait postinstall skipifsilent
