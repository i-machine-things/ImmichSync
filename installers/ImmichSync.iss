#define MyAppName "ImmichSync"
#define MyAppVersion GetEnv("RELEASE_VERSION")
#define MyAppPublisher "i-machine-things"
#define MyAppURL "https://github.com/i-machine-things/ImmichSync"
#define MyAppExeName "immichsync.exe"
#define MyAppId "{{EF081C4A-5F6E-4B02-885A-C5D7D8643100}"

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\ImmichSync
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\installer_out
OutputBaseFilename=ImmichSync-{#MyAppVersion}-windows-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ChangesEnvironment=yes
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\target\release\immichsync.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md";                     DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

[Run]
Filename: "{cmd}"; \
    Parameters: "/K ""{app}\{#MyAppExeName}"" init && ""{app}\{#MyAppExeName}"" service install"; \
    Description: "Run setup now (server URL, API key, photos folder) and enable the nightly schedule"; \
    Flags: postinstall skipifsilent unchecked

[UninstallRun]
Filename: "{app}\{#MyAppExeName}"; Parameters: "service uninstall"; Flags: runhidden skipifdoesntexist

[Code]
const
  UserEnvKey = 'Environment';

procedure AddToPath(AppDir: string);
{ Append AppDir to the user's PATH (per-user install, so always HKCU). Safe to
  call on upgrade — skipped when AppDir is already present. }
var
  EnvPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, UserEnvKey, 'Path', EnvPath) then
    EnvPath := '';

  if Pos(';' + Uppercase(AppDir) + ';', ';' + Uppercase(EnvPath) + ';') > 0 then
    Exit; { already present }

  if EnvPath = '' then
    EnvPath := AppDir
  else
    EnvPath := EnvPath + ';' + AppDir;

  RegWriteExpandStringValue(HKEY_CURRENT_USER, UserEnvKey, 'Path', EnvPath);
end;

procedure RemoveFromPath(AppDir: string);
var
  EnvPath: string;
  SearchStr: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, UserEnvKey, 'Path', EnvPath) then Exit;
  SearchStr := ';' + AppDir;
  P := Pos(LowerCase(SearchStr + ';'), LowerCase(EnvPath + ';'));
  if P > 0 then
  begin
    Delete(EnvPath, P, Length(SearchStr));
    RegWriteExpandStringValue(HKEY_CURRENT_USER, UserEnvKey, 'Path', EnvPath);
  end;
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
    AddToPath(ExpandConstant('{app}'));
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
    RemoveFromPath(ExpandConstant('{app}'));
end;
