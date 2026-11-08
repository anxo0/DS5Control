; Compile with: ISCC /DVersion=0.2.0 /DOrigen=<publish folder> /O<output folder> installer\ds5control.iss
; Draw the screens first: installer\diseno\construir.ps1 -Version 0.2.0
;
; Installs for the current user: no administrator prompt, and the program can
; replace its own files when it updates itself.
;
; The wizard is replaced by one frameless DS5Control window. Its background and
; fixed text are images drawn from installer\diseno; buttons, folder, progress and
; the checkbox are pieces laid on top. The standard wizard still runs underneath:
; its buttons are moved out of sight, so Enter and Escape keep working.

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
DisableWelcomePage=no
OutputBaseFilename=DS5Control-{#Version}-instalador
SetupIconFile=..\DS5Control\Resources\Logo\ds5control-fuego.ico
UninstallDisplayIcon={app}\DS5Control.exe
UninstallDisplayName=DS5Control
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern dark
WizardBackColor=#101114
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
CloseApplications=force
RestartApplications=no

[Languages]
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[Messages]
spanish.SetupWindowTitle=Instalar DS5Control
spanish.ButtonNext=Instalar
spanish.ButtonFinish=Terminar
spanish.StatusExtractFiles=Copiando archivos
spanish.StatusRunProgram=Terminando la instalación
spanish.ExitSetupTitle=Cancelar la instalación
spanish.ExitSetupMessage=DS5Control no está instalado todavía. ¿Quieres salir?

[Files]
Source: "{#Origen}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "piezas\*.bmp"; Flags: dontcopy

[Icons]
Name: "{autoprograms}\DS5Control"; Filename: "{app}\DS5Control.exe"

[Run]
Filename: "{app}\drivers\ViGEmBus_1.22.0_x64_x86_arm64.exe"; StatusMsg: "Instalando ViGEmBus, necesario para emular mandos"; Verb: runas; Flags: shellexec waituntilterminated; Check: FaltaViGEmBus
Filename: "{app}\DS5Control.exe"; Description: "Abrir DS5Control ahora"; Flags: nowait postinstall skipifsilent

[Code]
const
  // Inno colours are BGR: the app's dark tokens.
  ColorFondo = $141110;   // #101114
  ColorTinta = $F7F4F2;   // #f2f4f7
  ColorSuave = $BCB0A9;   // #a9b0bc
  AnchoVentana = 760;
  AltoVentana = 460;
  AltoCabecera = 56;
  WM_NCHITTEST = $84;
  HTCLIENT = 1;
  HTCAPTION = 2;
  GWL_WNDPROC = -4;

type
  TPieza = record
    Normal, Encima: TBitmapImage;
    X, Y, W, H: Integer;
  end;

var
  Escala: String;
  Fondo: TBitmapImage;
  Instalar, Cancelar, CancelarInstalando, Terminar, Cambiar, Cerrar: TPieza;
  CasillaSi, CasillaNo, Relleno: TBitmapImage;
  RutaTexto, EstadoTexto, CifraTexto: TNewStaticText;
  AnchoPista: Integer;
  ProcAnterior: LongInt;
  Temporizador: LongWord;

function SetWindowLong(Wnd: HWND; Index: Integer; NewLong: LongInt): LongInt;
  external 'SetWindowLongW@user32.dll stdcall';
function CallWindowProc(Proc: LongInt; Wnd: HWND; Msg: UINT; WParam, LParam: LongInt): LongInt;
  external 'CallWindowProcW@user32.dll stdcall';
function SetTimer(Wnd: HWND; Id, Elapse: LongWord; Func: LongWord): LongWord;
  external 'SetTimer@user32.dll stdcall';
function KillTimer(Wnd: HWND; Id: LongWord): Boolean;
  external 'KillTimer@user32.dll stdcall';
function GetCursorPos(var Punto: TPoint): Boolean;
  external 'GetCursorPos@user32.dll stdcall';
function ScreenToClient(Wnd: HWND; var Punto: TPoint): Boolean;
  external 'ScreenToClient@user32.dll stdcall';
function GetSystemMetrics(Index: Integer): Integer;
  external 'GetSystemMetrics@user32.dll stdcall';
function DwmSetWindowAttribute(Wnd: HWND; Atributo: Integer; var Valor: Integer; Tamano: Integer): Integer;
  external 'DwmSetWindowAttribute@dwmapi.dll stdcall';

function FaltaViGEmBus: Boolean;
begin
  Result := not RegKeyExists(HKLM, 'SYSTEM\CurrentControlSet\Services\ViGEmBus');
end;

{ Pieces }

function Imagen(const Nombre: String; X, Y, W, H: Integer): TBitmapImage;
var
  Archivo: String;
begin
  Archivo := Nombre + '-' + Escala + '.bmp';
  ExtractTemporaryFile(Archivo);
  Result := TBitmapImage.Create(WizardForm);
  Result.Parent := WizardForm;
  Result.Left := ScaleX(X);
  Result.Top := ScaleY(Y);
  Result.Width := ScaleX(W);
  Result.Height := ScaleY(H);
  Result.Stretch := True;
  Result.Bitmap.LoadFromFile(ExpandConstant('{tmp}\') + Archivo);
end;

function Pieza(const Nombre: String; X, Y, W, H: Integer; AlPulsar: TNotifyEvent): TPieza;
begin
  Result.X := X; Result.Y := Y; Result.W := W; Result.H := H;
  Result.Normal := Imagen(Nombre, X, Y, W, H);
  Result.Encima := Imagen(Nombre + '-hover', X, Y, W, H);
  Result.Encima.Visible := False;
  Result.Normal.Cursor := crHand;
  Result.Encima.Cursor := crHand;
  Result.Normal.OnClick := AlPulsar;
  Result.Encima.OnClick := AlPulsar;
end;

procedure Mostrar(var P: TPieza; Visible: Boolean);
begin
  P.Normal.Visible := Visible;
  P.Encima.Visible := False;
end;

function Texto(X, Y, W, Tamano: Integer; Negrita: Boolean; Color: TColor): TNewStaticText;
begin
  Result := TNewStaticText.Create(WizardForm);
  Result.Parent := WizardForm;
  Result.AutoSize := False;
  Result.Left := ScaleX(X);
  Result.Top := ScaleY(Y);
  Result.Width := ScaleX(W);
  Result.Font.Name := 'Segoe UI Variable Text';
  Result.Font.Size := Tamano;
  Result.Font.Color := Color;
  if Negrita then Result.Font.Style := [fsBold];
  Result.Color := ColorFondo;
  Result.Caption := ' ';
  Result.AdjustHeight;
end;

procedure Fondear(const Pantalla: String);
begin
  Fondo.Bitmap.LoadFromFile(ExpandConstant('{tmp}\fondo-') + Pantalla + '-' + Escala + '.bmp');
end;

{ Hover: Inno only reports clicks, so a light timer follows the cursor. }

function Dentro(const P: TPieza; X, Y: Integer): Boolean;
begin
  Result := (X >= ScaleX(P.X)) and (X < ScaleX(P.X + P.W)) and (Y >= ScaleY(P.Y)) and (Y < ScaleY(P.Y + P.H));
end;

procedure Seguir(var P: TPieza; X, Y: Integer);
var
  Encima: Boolean;
begin
  if not (P.Normal.Visible or P.Encima.Visible) then Exit;
  Encima := Dentro(P, X, Y);
  if Encima <> P.Encima.Visible then
  begin
    P.Encima.Visible := Encima;
    P.Normal.Visible := not Encima;
  end;
end;

procedure AlTemporizador(Wnd: HWND; Msg: UINT; Id: LongWord; Tiempo: DWORD);
var
  Punto: TPoint;
begin
  if Temporizador = 0 then Exit;
  if not GetCursorPos(Punto) then Exit;
  ScreenToClient(WizardForm.Handle, Punto);
  Seguir(Instalar, Punto.X, Punto.Y);
  Seguir(Cancelar, Punto.X, Punto.Y);
  Seguir(CancelarInstalando, Punto.X, Punto.Y);
  Seguir(Terminar, Punto.X, Punto.Y);
  Seguir(Cambiar, Punto.X, Punto.Y);
  Seguir(Cerrar, Punto.X, Punto.Y);
  // ViGEmBus and the last step report themselves through the hidden status label.
  if EstadoTexto.Visible and (WizardForm.StatusLabel.Caption <> '') then
    EstadoTexto.Caption := WizardForm.StatusLabel.Caption;
end;

{ Dragging: the top band of the window behaves as its title bar. }

function ProcVentana(Wnd: HWND; Msg: UINT; WParam, LParam: LongInt): LongInt;
var
  Punto: TPoint;
begin
  Result := CallWindowProc(ProcAnterior, Wnd, Msg, WParam, LParam);
  if (Msg = WM_NCHITTEST) and (Result = HTCLIENT) then
  begin
    Punto.X := LParam and $FFFF;
    Punto.Y := (LParam shr 16) and $FFFF;
    if Punto.X > $7FFF then Punto.X := Punto.X - $10000;
    if Punto.Y > $7FFF then Punto.Y := Punto.Y - $10000;
    ScreenToClient(Wnd, Punto);
    if (Punto.Y < ScaleY(AltoCabecera)) and not Dentro(Cerrar, Punto.X, Punto.Y) then
      Result := HTCAPTION;
  end;
end;

{ Hooks must be released before the window goes away: Windows would still call them. }

procedure SoltarArrastre;
begin
  if ProcAnterior <> 0 then
  begin
    SetWindowLong(WizardForm.Handle, GWL_WNDPROC, ProcAnterior);
    ProcAnterior := 0;
  end;
end;

procedure Soltar;
begin
  if Temporizador <> 0 then
  begin
    KillTimer(0, Temporizador);
    Temporizador := 0;
  end;
  SoltarArrastre;
end;

{ Actions }

procedure Siguiente;
begin
  WizardForm.NextButton.OnClick(WizardForm.NextButton);
end;

procedure PulsarInstalar(Sender: TObject);
begin
  if WizardForm.CurPageID = wpFinished then Soltar;
  Siguiente;
end;

procedure PulsarCancelar(Sender: TObject);
begin
  if WizardForm.CurPageID = wpFinished then Siguiente
  else WizardForm.CancelButton.OnClick(WizardForm.CancelButton);
end;

procedure PulsarCambiar(Sender: TObject);
var
  Carpeta: String;
begin
  Carpeta := WizardForm.DirEdit.Text;
  if BrowseForFolder('Elige dónde instalar DS5Control', Carpeta, True) then
  begin
    WizardForm.DirEdit.Text := Carpeta;
    RutaTexto.Caption := MinimizePathName(Carpeta, RutaTexto.Font, RutaTexto.Width);
  end;
end;

procedure PulsarCasilla(Sender: TObject);
var
  Abrir: Boolean;
begin
  Abrir := not WizardForm.RunList.Checked[0];
  WizardForm.RunList.Checked[0] := Abrir;
  CasillaSi.Visible := Abrir;
  CasillaNo.Visible := not Abrir;
end;

{ Wizard }

procedure EsconderAsistente;
begin
  // Out of sight but alive: they still drive the pages and answer Enter and Escape.
  WizardForm.NextButton.Left := -ScaleX(2000);
  WizardForm.CancelButton.Left := -ScaleX(2000);
  WizardForm.BackButton.Left := -ScaleX(2000);
  WizardForm.OuterNotebook.Visible := False;
  WizardForm.Bevel.Visible := False;
end;

procedure InitializeWizard;
var
  Redondo: Integer;
begin
  // A silent install shows no window: nothing to draw.
  if WizardSilent then Exit;
  if ScaleX(100) >= 175 then Escala := '200'
  else if ScaleX(100) >= 138 then Escala := '150'
  else if ScaleX(100) >= 113 then Escala := '125'
  else Escala := '100';

  WizardForm.BorderStyle := bsNone;
  WizardForm.ClientWidth := ScaleX(AnchoVentana);
  WizardForm.ClientHeight := ScaleY(AltoVentana);
  WizardForm.Left := (GetSystemMetrics(0) - WizardForm.Width) div 2;
  WizardForm.Top := (GetSystemMetrics(1) - WizardForm.Height) div 2;
  EsconderAsistente;

  ExtractTemporaryFile('fondo-instalando-' + Escala + '.bmp');
  ExtractTemporaryFile('fondo-listo-' + Escala + '.bmp');
  Fondo := Imagen('fondo-bienvenida', 0, 0, AnchoVentana, AltoVentana);

  Instalar := Pieza('instalar', 272, 388, 132, 44, @PulsarInstalar);
  Cancelar := Pieza('cancelar', 172, 388, 92, 44, @PulsarCancelar);
  CancelarInstalando := Pieza('cancelar', 312, 388, 92, 44, @PulsarCancelar);
  Terminar := Pieza('terminar', 272, 388, 132, 44, @PulsarInstalar);
  Cambiar := Pieza('cambiar', 354, 323, 50, 19, @PulsarCambiar);
  Cerrar := Pieza('cerrar', 718, 14, 28, 28, @PulsarCancelar);

  RutaTexto := Texto(40, 323, 302, 10, False, ColorTinta);
  RutaTexto.Caption := MinimizePathName(WizardForm.DirEdit.Text, RutaTexto.Font, RutaTexto.Width);

  EstadoTexto := Texto(40, 262, 300, 10, False, ColorSuave);
  CifraTexto := Texto(334, 262, 70, 10, True, ColorTinta);
  CifraTexto.Alignment := taRightJustify;
  AnchoPista := 364;
  Relleno := Imagen('relleno', 40, 289, AnchoPista, 6);
  Relleno.Stretch := False;

  CasillaSi := Imagen('casilla-si', 40, 278, 364, 22);
  CasillaNo := Imagen('casilla-no', 40, 278, 364, 22);
  CasillaSi.Cursor := crHand; CasillaNo.Cursor := crHand;
  CasillaSi.OnClick := @PulsarCasilla; CasillaNo.OnClick := @PulsarCasilla;

  Redondo := 2;
  DwmSetWindowAttribute(WizardForm.Handle, 33, Redondo, 4);
  ProcAnterior := SetWindowLong(WizardForm.Handle, GWL_WNDPROC, CreateCallback(@ProcVentana));
  Temporizador := SetTimer(0, 0, 40, CreateCallback(@AlTemporizador));
end;

procedure CurPageChanged(CurPageID: Integer);
var
  Bienvenida, Instalando, Listo: Boolean;
begin
  if WizardSilent then Exit;
  Bienvenida := CurPageID = wpWelcome;
  Instalando := CurPageID = wpInstalling;
  Listo := CurPageID = wpFinished;
  if Bienvenida then Fondear('bienvenida')
  else if Instalando then Fondear('instalando')
  else if Listo then Fondear('listo');

  Mostrar(Instalar, Bienvenida);
  Mostrar(Cancelar, Bienvenida);
  Mostrar(Cambiar, Bienvenida);
  RutaTexto.Visible := Bienvenida;
  Mostrar(CancelarInstalando, Instalando);
  EstadoTexto.Visible := Instalando;
  CifraTexto.Visible := Instalando;
  Relleno.Visible := Instalando;
  Mostrar(Terminar, Listo);
  CasillaSi.Visible := Listo and WizardForm.RunList.Checked[0];
  CasillaNo.Visible := Listo and not WizardForm.RunList.Checked[0];
  Mostrar(Cerrar, True);
  // The last screen closes the window: stop dragging before that happens.
  if Listo then SoltarArrastre;

  if Instalando then
  begin
    EstadoTexto.Caption := 'Copiando archivos';
    CifraTexto.Caption := '0 %';
    Relleno.Width := 0;
  end;
  EsconderAsistente;
end;

procedure CurInstallProgressChanged(CurProgress, MaxProgress: Integer);
begin
  if (MaxProgress <= 0) or (Relleno = nil) then Exit;
  Relleno.Width := ScaleX(AnchoPista) * CurProgress div MaxProgress;
  CifraTexto.Caption := IntToStr(CurProgress * 100 div MaxProgress) + ' %';
end;

procedure CancelButtonClick(CurPageID: Integer; var Cancel, Confirm: Boolean);
begin
  if WizardSilent then Exit;
  // Asked here, so the hooks are released only when leaving for real.
  Confirm := False;
  Cancel := MsgBox('DS5Control no está instalado todavía. ¿Quieres salir?', mbConfirmation, MB_YESNO) = IDYES;
  if Cancel then Soltar;
end;

procedure DeinitializeSetup;
begin
  if Temporizador <> 0 then KillTimer(0, Temporizador);
  Temporizador := 0;
end;
