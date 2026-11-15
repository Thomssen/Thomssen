; PSYCHOSEC installer (Inno Setup 6).  Build:
;   iscc /DAppVersion=1.0.0 /DSourceExe=..\target\release\Psychosec.exe installer\psychosec.iss
; Output: installer\Output\Psychosec-Setup-v<version>.exe
#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef SourceExe
  #define SourceExe "..\target\release\Psychosec.exe"
#endif

[Setup]
AppId={{6B0D6F1E-5C3A-4D8E-9B57-2F4E7A1C9D30}
AppName=PSYCHOSEC
AppVersion={#AppVersion}
AppVerName=PSYCHOSEC {#AppVersion}
AppPublisher=PSYCHOSEC
AppPublisherURL=https://psychosec.org/
AppSupportURL=https://psychosec.org/
AppUpdatesURL=https://psychosec.org/download
VersionInfoVersion={#AppVersion}
VersionInfoProductName=PSYCHOSEC
VersionInfoDescription=PSYCHOSEC Setup
VersionInfoCompany=PSYCHOSEC
; Per-user install, no administrator rights required.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
DefaultDirName={localappdata}\Programs\PSYCHOSEC
DefaultGroupName=PSYCHOSEC
DisableProgramGroupPage=yes
UsePreviousAppDir=yes
SetupIconFile=psychosec.ico
UninstallDisplayIcon={app}\Psychosec.exe
UninstallDisplayName=PSYCHOSEC
OutputDir=Output
OutputBaseFilename=Psychosec-Setup-v{#AppVersion}
Compression=lzma2/ultra
SolidCompression=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
WizardStyle=modern
InfoBeforeFile=privacy.txt
CloseApplications=yes
RestartApplications=no
; Authenticode: enabled by CI when a certificate is available (see docs/releasing.md)
#ifdef SignTool
SignTool={#SignTool}
SignedUninstaller=yes
#endif

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; GroupDescription: "Shortcuts:"; Flags: unchecked
Name: "addtopath"; Description: "Add PSYCHOSEC to my &PATH (run 'Psychosec' from any terminal)"; GroupDescription: "Command line:"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "Psychosec.exe"; Flags: ignoreversion

[Icons]
; The application is a console program: launch it in a console window.
Name: "{autoprograms}\PSYCHOSEC"; Filename: "{app}\Psychosec.exe"; WorkingDir: "{app}"; Comment: "PSYCHOSEC - private OSINT & investigation terminal"
Name: "{autodesktop}\PSYCHOSEC"; Filename: "{app}\Psychosec.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Registry]
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Tasks: addtopath; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\Psychosec.exe"; Description: "Launch PSYCHOSEC"; Flags: nowait postinstall skipifsilent unchecked

[Code]
function NeedsAddPath(Param: string): boolean;
var OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath) then begin Result := True; exit; end;
  Result := Pos(';' + Uppercase(Param) + ';', ';' + Uppercase(OrigPath) + ';') = 0;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var DataDir: string;
begin
  if CurUninstallStep = usPostUninstall then begin
    DataDir := ExpandConstant('{localappdata}\PSYCHOSEC');
    if DirExists(DataDir) then
      if MsgBox('Also delete your local PSYCHOSEC data (cases, settings, downloaded updates)?' + #13#10 +
                'Choose No to keep your investigation cases.', mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
        DelTree(DataDir, True, True, True);
  end;
end;
