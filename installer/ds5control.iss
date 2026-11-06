; Compile with: ISCC /DVersion=0.2.0 /DOrigen=<publish folder> /O<output folder> installer\ds5control.iss
;
; Installs for the current user: no administrator prompt, and the program can
; replace its own files when it updates itself.

[Setup]
AppId={{B7E2C8E4-5D1F-4C36-9A57-5C0D5E2F1A31}
AppName=DS5Control
AppVersion={#Version}
AppVerName=DS5Control {#Version}
AppPublisher=anxo0
AppPublisherURL=https://github.com/anxo0/DS5Control
AppSupportURL=https://github.com/anxo0/DS5Control/issues
AppUpdatesURL=https://github.com/anxo0/DS5Control/releases
PrivilegesRequired=lowest
DefaultDirName={localappdata}\Programs\DS5Control
DisableDirPage=auto
DisableProgramGroupPage=yes
LicenseFile=..\COPYING
OutputBaseFilename=DS5Control-{#Version}-instalador
SetupIconFile=..\DS5Control\Resources\Logo\ds5control-fuego.ico
UninstallDisplayIcon={app}\DS5Control.exe
UninstallDisplayName=DS5Control
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
CloseApplications=force
RestartApplications=no

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "escritorio"; Description: "Crear un acceso directo en el escritorio"; Flags: unchecked

[Files]
Source: "{#Origen}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\DS5Control"; Filename: "{app}\DS5Control.exe"
Name: "{autodesktop}\DS5Control"; Filename: "{app}\DS5Control.exe"; Tasks: escritorio

[Run]
Filename: "{app}\DS5Control.exe"; Description: "Abrir DS5Control"; Flags: nowait postinstall skipifsilent
