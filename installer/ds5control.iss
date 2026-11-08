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
DisableDirPage=yes
DisableReadyPage=yes
DisableProgramGroupPage=yes
OutputBaseFilename=DS5Control-{#Version}-instalador
SetupIconFile=..\DS5Control\Resources\Logo\ds5control-fuego.ico
UninstallDisplayIcon={app}\DS5Control.exe
UninstallDisplayName=DS5Control
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern dark
WizardBackColor=#101114
DisableWelcomePage=no
WizardImageFile=lateral-100.bmp,lateral-150.bmp,lateral-200.bmp
WizardSmallImageFile=pequena-100.bmp,pequena-150.bmp,pequena-200.bmp
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
CloseApplications=force
RestartApplications=no

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Messages]
spanish.WelcomeLabel1=DS5Control
spanish.WelcomeLabel2=Tu DualSense en Windows: luz, vibración, gatillos y botones.%n%nSe instalará la versión {#Version} solo para tu usuario.
spanish.ButtonNext=Instalar
spanish.ClickNext=Pulsa Instalar para empezar.
spanish.FinishedHeadingLabel=Todo listo
spanish.FinishedLabelNoIcons=DS5Control ya está instalado. Lo tienes en el menú Inicio.
spanish.FinishedLabel=DS5Control ya está instalado. Lo tienes en el menú Inicio.

[Files]
Source: "{#Origen}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\DS5Control"; Filename: "{app}\DS5Control.exe"

[Run]
Filename: "{app}\drivers\ViGEmBus_1.22.0_x64_x86_arm64.exe"; StatusMsg: "Instalando ViGEmBus, necesario para emular mandos..."; Verb: runas; Flags: shellexec waituntilterminated; Check: FaltaViGEmBus
Filename: "{app}\DS5Control.exe"; Description: "Abrir DS5Control"; Flags: nowait postinstall skipifsilent

[Code]
function FaltaViGEmBus: Boolean;
begin
  Result := not RegKeyExists(HKLM, 'SYSTEM\CurrentControlSet\Services\ViGEmBus');
end;
