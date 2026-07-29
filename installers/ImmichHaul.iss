#define MyAppName "ImmichHaul"
#define MyAppVersion GetEnv("RELEASE_VERSION")
#define MyAppPublisher "i-machine-things"
#define MyAppURL "https://github.com/i-machine-things/ImmichSync"
#define MyAppExeName "immich-haul.exe"
#define MyAppId "{{EF081C4A-5F6E-4B02-885A-C5D7D8643100}"

[Setup]
AppId={#MyAppId}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={localappdata}\ImmichHaul
DefaultGroupName={#MyAppName}
DisableProgramGroupPage=yes
OutputDir=..\installer_out
OutputBaseFilename=ImmichHaul-{#MyAppVersion}-windows-setup
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ChangesEnvironment=yes
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Files]
Source: "..\target\release\immich-haul.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md";                     DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

[Run]
; `service install` already runs setup (ensure_config) before installing the
; schedule — see src/main.rs — so a single command covers both steps.
; Invoked directly (no cmd.exe wrapper): Windows allocates a console
; automatically for a console-subsystem exe launched this way, so the
; interactive wizard still works, and it avoids cmd.exe entirely — no
; quoting rules to trip over, and no risk of a `/K` shell staying open
; after the command finishes and blocking the installer's Finish page.
Filename: "{app}\{#MyAppExeName}"; \
    Parameters: "service install"; \
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
{ Rebuilds PATH from its ';'-separated entries, dropping any that match AppDir
  case-insensitively. Works regardless of whether AppDir is the first, middle,
  last, or only entry — unlike a padded substring search, which misses the
  first-entry case since there's no leading separator to match against. }
var
  EnvPath, Entry, Rebuilt: string;
  P: Integer;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, UserEnvKey, 'Path', EnvPath) then Exit;
  Rebuilt := '';
  while EnvPath <> '' do
  begin
    P := Pos(';', EnvPath);
    if P > 0 then
    begin
      Entry := Copy(EnvPath, 1, P - 1);
      Delete(EnvPath, 1, P);
    end else
    begin
      Entry := EnvPath;
      EnvPath := '';
    end;
    if (Entry <> '') and (CompareText(Entry, AppDir) <> 0) then
    begin
      if Rebuilt = '' then
        Rebuilt := Entry
      else
        Rebuilt := Rebuilt + ';' + Entry;
    end;
  end;
  RegWriteExpandStringValue(HKEY_CURRENT_USER, UserEnvKey, 'Path', Rebuilt);
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
