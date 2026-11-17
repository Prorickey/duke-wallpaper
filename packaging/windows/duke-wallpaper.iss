#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

[Setup]
AppId={{6F1C2B4E-8D3A-4B7F-9E21-5A0C7D9B3F48}
AppName=Duke Wallpaper
AppVersion={#AppVersion}
AppVerName=Duke Wallpaper {#AppVersion}
AppPublisher=Prorickey
AppPublisherURL=https://github.com/Prorickey/duke-wallpaper
AppSupportURL=https://github.com/Prorickey/duke-wallpaper/issues
DefaultDirName={autopf}\Duke Wallpaper
DisableDirPage=yes
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
CloseApplications=no
SetupIconFile=app-icon.ico
UninstallDisplayIcon={app}\duke-wallpaper.exe
UninstallDisplayName=Duke Wallpaper
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
OutputBaseFilename=DukeWallpaper-Windows-Setup

[Files]
Source: "..\..\target\release\duke-wallpaper.exe"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Duke Wallpaper"; Filename: "{app}\duke-wallpaper.exe"

[Run]
Filename: "{app}\duke-wallpaper.exe"; Description: "Start Duke Wallpaper now"; Flags: postinstall nowait skipifsilent

[Code]
procedure StopRunningWallpaper;
var
  ExitCode: Integer;
begin
  if Exec(ExpandConstant('{sys}\taskkill.exe'), '/IM duke-wallpaper.exe', '', SW_HIDE, ewWaitUntilTerminated, ExitCode) and (ExitCode = 0) then
  begin
    Sleep(2000);
    Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM duke-wallpaper.exe', '', SW_HIDE, ewWaitUntilTerminated, ExitCode);
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  StopRunningWallpaper;
  Result := '';
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    StopRunningWallpaper;
  if CurUninstallStep = usPostUninstall then
    RegDeleteValue(HKEY_CURRENT_USER, 'Software\Microsoft\Windows\CurrentVersion\Run', 'DukePixelArtBackground');
end;
