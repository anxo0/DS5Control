//! Un perfil (`Profiles/<nombre>.xml`), campo a campo como `ProfileDTO.cs`.
//!
//! Cada campo guarda el valor tal como queda escrito en el archivo, después de los ajustes que
//! le aplica la versión en C# al leerlo (límites, paso a radianes y vuelta...). Así, leer y
//! guardar da exactamente el mismo archivo que C#.

use std::f64::consts::PI;

use crate::campos::*;
use crate::formato::{
    enumerado, grupo, Anidado, BoolTexto, BoolXml, Byte, ByteConSigno, Doble, Entero, Enumerado,
    Grupo, Natural, Texto, TextoOpcional,
};
use crate::mapeos::Mapeos;
use crate::migracion;
use crate::numeros;
use crate::xml::{Elemento, Escritor};

enumerado! { pub enum ModoLuz { None = "None", DS5Win = "DS4Win", Passthru = "Passthru" } }
enumerado! { pub enum TipoZonaMuerta { Radial = "Radial", Axial = "Axial" } }
enumerado! { pub enum SalidaPanel { None = "None", Mouse = "Mouse", Controls = "Controls", MouseJoystick = "MouseJoystick", AbsoluteMouse = "AbsoluteMouse", Passthru = "Passthru" } }
enumerado! { pub enum EjeVolante { None = "None", LX = "LX", LY = "LY", RX = "RX", RY = "RY", L2R2 = "L2R2", VJoy1X = "VJoy1X", VJoy1Y = "VJoy1Y", VJoy1Z = "VJoy1Z", VJoy2X = "VJoy2X", VJoy2Y = "VJoy2Y", VJoy2Z = "VJoy2Z" } }
enumerado! { pub enum SalidaGiroscopio { None = "None", Controls = "Controls", Mouse = "Mouse", MouseJoystick = "MouseJoystick", DirectionalSwipe = "DirectionalSwipe", Passthru = "Passthru" } }
enumerado! { pub enum PalancaSalida { None = "None", LeftStick = "LeftStick", RightStick = "RightStick" } }
enumerado! { pub enum EjesSalida { None = "None", XY = "XY", X = "X", Y = "Y" } }
enumerado! { pub enum EjeDeslizar { Yaw = "Yaw", Roll = "Roll" } }
enumerado! { pub enum ModoPalanca { None = "None", Controls = "Controls", FlickStick = "FlickStick" } }
enumerado! { pub enum EmulacionVibracion { Accurate = "Accurate", Legacy = "Legacy", Disabled = "Disabled", Passthru = "Passthru" } }
enumerado! { pub enum DosFases { Disabled = "Disabled", Normal = "Normal", ExclusiveButtons = "ExclusiveButtons", HairTrigger = "HairTrigger", HipFire = "HipFire", HipFireExclusiveButtons = "HipFireExclusiveButtons" } }
enumerado! { pub enum EfectoGatillo { None = "None", Rigid = "Rigid", Pulse = "Pulse", FullClick = "FullClick" } }
enumerado! { pub enum CurvaPanel { Linear = "Linear", EnhancedPrecision = "EnhancedPrecision", Quadratic = "Quadratic", Cubic = "Cubic", EaseoutQuad = "EaseoutQuad", EaseoutCubic = "EaseoutCubic" } }
enumerado! { pub enum SuavizadoPanel { None = "None", OneEuro = "OneEuro", WeightedAverage = "WeightedAverage" } }
enumerado! { pub enum ActivacionPanel { Click = "Click", Touch = "Touch", Release = "Release" } }
enumerado! { pub enum MandoVirtual { None = "None", X360 = "X360", DS4 = "DS4" } }
enumerado! { pub enum SalidaGatillos { Default = "Default", Analog = "Analog", Buttons = "Buttons" } }

fn porcentaje(v: f64) -> f64 {
    (v * 0.01).clamp(0.0, 1.0) * 100.0
}

grupo! {
    /// Zonas muertas por eje (`LSAxialDeadOptions`). Sin valor, todo es 0.
    pub struct ZonasAxiales {
        "DeadZoneX" => zona_muerta_x: Entero = 0, |v: i32| v.clamp(0, 127);
        "DeadZoneY" => zona_muerta_y: Entero = 0, |v: i32| v.clamp(0, 127);
        "MaxZoneX" => zona_maxima_x: Entero = 0, |v: i32| v.clamp(0, 100);
        "MaxZoneY" => zona_maxima_y: Entero = 0, |v: i32| v.clamp(0, 100);
        "AntiDeadZoneX" => antizona_x: Entero = 0, |v: i32| v.clamp(0, 100);
        "AntiDeadZoneY" => antizona_y: Entero = 0, |v: i32| v.clamp(0, 100);
        "MaxOutputX" => salida_maxima_x: Doble = 0.0, |v: f64| v.clamp(0.0, 100.0);
        "MaxOutputY" => salida_maxima_y: Doble = 0.0, |v: f64| v.clamp(0.0, 100.0);
    }
}

grupo! {
    /// Aceleración por velocidad del stick (`LSDeltaAccelSettings`).
    pub struct AceleracionPalanca {
        "Enabled" => activa: BoolTexto = false;
        "Multiplier" => multiplicador: Doble = 4.0, |v: f64| v.clamp(0.0, 10.0);
        "MaxTravel" => recorrido_maximo: Doble = 0.2, |v: f64| v.clamp(0.0, 1.0);
        "MinTravel" => recorrido_minimo: Doble = 0.01, |v: f64| v.clamp(0.0, 1.0);
        "EasingDuration" => duracion_suavizado: Doble = 0.2, |v: f64| v.clamp(0.0, 600.0);
        "MinFactor" => factor_minimo: Doble = 1.0, |v: f64| v.clamp(1.0, 10.0);
    }
}

grupo! {
    pub struct SuavizadoVolante {
        "SASteeringWheelUseSmoothing" => activo: BoolTexto = false;
        "SASteeringWheelSmoothMinCutoff" => corte_minimo: Doble = 0.1;
        "SASteeringWheelSmoothBeta" => beta: Doble = 0.1;
    }
}

grupo! {
    pub struct ControlesGiroscopio {
        "Triggers" => gatillos: Texto = "-1".into();
        "TriggerCond" => condicion: Condicion = true;
        "TriggerTurns" => gatillo_alterna: BoolTexto = true;
        "Toggle" => alterna: BoolTexto = false;
    }
}

grupo! {
    /// Suavizado del giroscopio como ratón y como stick. El peso se guarda en porcentaje.
    pub struct SuavizadoGiroscopio {
        "UseSmoothing" => activo: BoolTexto = false;
        "SmoothingMethod" => metodo: Suavizado = MetodoSuavizado::UnEuro;
        "SmoothingWeight" => peso: Doble = 50.0, porcentaje;
        "SmoothingMinCutoff" => corte_minimo: Doble = 0.4, |v: f64| v.clamp(0.0, 100.0);
        "SmoothingBeta" => beta: Doble = 0.7, |v: f64| v.clamp(0.0, 1.0);
    }
}

grupo! {
    pub struct DeslizarGiroscopio {
        "DeadZoneX" => zona_muerta_x: Entero = 80;
        "DeadZoneY" => zona_muerta_y: Entero = 80;
        "Triggers" => gatillos: Texto = "-1".into();
        "TriggerCond" => condicion: Condicion = true;
        "TriggerTurns" => gatillo_alterna: BoolTexto = true;
        "XAxis" => eje_x: Enumerado<EjeDeslizar> = EjeDeslizar::Yaw;
        "DelayTime" => retardo: Entero = 0;
    }
}

grupo! {
    pub struct FlickStick {
        "RealWorldCalibration" => calibracion: Doble = 0.0;
        "FlickThreshold" => umbral: Doble = 0.0;
        "FlickTime" => tiempo: Doble = 0.0;
        "MinAngleThreshold" => angulo_minimo: Doble = 0.0;
    }
}

grupo! {
    pub struct SalidaPalanca {
        "FlickStickSettings" => flick: Anidado<FlickStick> = FlickStick::default();
    }
}

grupo! {
    pub struct VibracionDualSense {
        "EmulationMode" => emulacion: Enumerado<EmulacionVibracion> = EmulacionVibracion::Accurate;
        "EnableGenericRumbleRescale" => reescalado_generico: BoolTexto = false;
        "HapticPowerLevel" => potencia_haptica: Byte = 0;
    }
}

grupo! {
    pub struct AjustesDualSense {
        "RumbleSettings" => vibracion: Anidado<VibracionDualSense> = VibracionDualSense::default();
    }
}

grupo! {
    pub struct RatonAbsolutoPanel {
        "MaxZoneX" => zona_maxima_x: Entero = 90;
        "MaxZoneY" => zona_maxima_y: Entero = 90;
        "SnapToCenter" => volver_al_centro: BoolTexto = false;
    }
}

grupo! {
    pub struct SuavizadoPanelStick {
        "SmoothingMethod" => metodo: Enumerado<SuavizadoPanel> = SuavizadoPanel::None;
        "SmoothingMinCutoff" => corte_minimo: Doble = 0.8;
        "SmoothingBeta" => beta: Doble = 0.7;
    }
}

grupo! {
    /// El panel táctil como stick. `Rotation` se guarda en grados enteros, pero C# lo lee como
    /// grados y lo escribe como si fuera otra unidad, así que casi siempre vuelve a 0.
    pub struct PanelComoStick {
        "DeadZone" => zona_muerta: Entero = 0;
        "MaxZone" => zona_maxima: Entero = 8;
        "OutputStick" => stick: Enumerado<PalancaSalida> = PalancaSalida::RightStick;
        "OutputStickAxes" => ejes: Enumerado<EjesSalida> = EjesSalida::XY;
        "AntiDeadX" => antizona_x: Doble = 0.4;
        "AntiDeadY" => antizona_y: Doble = 0.4;
        "Invert" => invertir: Natural = 0;
        "MaxOutput" => salida_maxima: Doble = 100.0;
        "MaxOutputEnabled" => salida_maxima_activa: BoolTexto = false;
        "VerticalScale" => escala_vertical: Entero = 100;
        "OutputCurve" => curva: Enumerado<CurvaPanel> = CurvaPanel::Linear;
        "Rotation" => rotacion: Entero = 0, |v: i32| (v as f64 * PI / 180.0 / 180.0 * PI) as i32;
        "SmoothingSettings" => suavizado: Anidado<SuavizadoPanelStick> = SuavizadoPanelStick::default();
    }
}

grupo! {
    pub struct RegionRatonAbsoluto {
        "AbsWidth" => ancho: Doble = 1.0, |v: f64| v.clamp(0.0, 1.0);
        "AbsHeight" => alto: Doble = 1.0, |v: f64| v.clamp(0.0, 1.0);
        "AbsXCenter" => centro_x: Doble = 0.5, |v: f64| v.clamp(0.0, 1.0);
        "AbsYCenter" => centro_y: Doble = 0.5, |v: f64| v.clamp(0.0, 1.0);
        "AntiRadius" => antirradio: Doble = 0.0, |v: f64| v.clamp(0.0, 1.0);
        "SnapToCenter" => volver_al_centro: BoolTexto = true;
    }
}

fn rotacion_palanca(v: f64) -> f64 {
    v.clamp(-180.0, 180.0) * PI / 180.0 * 180.0 / PI
}

/// C# pasa la rotación del ratón táctil a radianes al leerla, la guarda en radianes y al
/// escribirla la vuelve a pasar a radianes como si fueran grados.
fn rotacion_raton_tactil(v: f64) -> f64 {
    let radianes = v.clamp(-180.0, 180.0) * PI / 180.0;
    radianes.clamp(-180.0, 180.0) * PI / 180.0
}

grupo! {
    /// Todos los ajustes simples del perfil, en el orden del archivo.
    pub struct Ajustes {
        "touchToggle" => tactil_alterna: BoolTexto = true;
        "idleDisconnectTimeout" => desconexion_inactivo: Entero = 0;
        "outputDataToDS4" => enviar_al_mando: BoolTexto = true;
        "Color" => color: ColorRgb = Rgb::default();
        "LeftStickDriftXAxis" => deriva_izq_x: ByteConSigno = 0;
        "LeftStickDriftYAxis" => deriva_izq_y: ByteConSigno = 0;
        "RightStickDriftXAxis" => deriva_der_x: ByteConSigno = 0;
        "RightStickDriftYAxis" => deriva_der_y: ByteConSigno = 0;
        "DebouncingMs" => antirrebote_ms: Entero = 0;
        "InverseRumbleMotors" => invertir_motores: BoolXml = false;
        "UseDs3PitchRollSim" => simular_ds3: BoolXml = false;
        "RumbleBoost" => vibracion: Byte = 100;
        "RumbleAutostopTime" => parada_vibracion: Entero = 0;
        "LightbarMode" => modo_luz: Enumerado<ModoLuz> = ModoLuz::DS5Win;
        "ledAsBatteryIndicator" => luz_bateria: BoolTexto = false;
        "FlashType" => tipo_destello: Byte = 0;
        "flashBatteryAt" => destello_bateria_en: Entero = 0;
        "touchSensitivity" => sensibilidad_tactil: Byte = 100;
        "LowColor" => color_bajo: ColorRgb = Rgb::default();
        "ChargingColor" => color_carga: ColorRgb = Rgb::default();
        "FlashColor" => color_destello: ColorRgb = Rgb::default();
        "touchpadJitterCompensation" => compensar_temblor_tactil: BoolTexto = true;
        "lowerRCOn" => esquina_inferior_derecha: BoolTexto = false;
        "tapSensitivity" => sensibilidad_toque: Byte = 0;
        "doubleTap" => doble_toque: BoolTexto = false;
        "scrollSensitivity" => sensibilidad_desplazamiento: Entero = 0;
        "LeftTriggerMiddle" => l2_zona_muerta: Byte = 0;
        "RightTriggerMiddle" => r2_zona_muerta: Byte = 0;
        "TouchpadInvert" => invertir_tactil: Entero = 0, |v: i32| v.clamp(0, 3);
        "TouchpadClickPassthru" => clic_tactil_directo: BoolTexto = false;
        "L2AntiDeadZone" => l2_antizona: Entero = 0;
        "R2AntiDeadZone" => r2_antizona: Entero = 0;
        "L2MaxZone" => l2_zona_maxima: Entero = 100, |v: i32| v.clamp(0, 100);
        "R2MaxZone" => r2_zona_maxima: Entero = 100, |v: i32| v.clamp(0, 100);
        "L2MaxOutput" => l2_salida_maxima: Doble = 100.0, |v: f64| v.clamp(0.0, 100.0);
        "R2MaxOutput" => r2_salida_maxima: Doble = 100.0, |v: f64| v.clamp(0.0, 100.0);
        "ButtonMouseSensitivity" => sensibilidad_raton_botones: Entero = 25;
        "ButtonMouseOffset" => desfase_raton_botones: Doble = 0.008;
        "Rainbow" => arcoiris: Doble = 0.0;
        "MaxSatRainbow" => saturacion_arcoiris: Doble = 100.0, |v: f64| v.clamp(0.0, 100.0) / 100.0 * 100.0;
        "LSDeadZone" => ls_zona_muerta: Entero = 10, |v: i32| v.clamp(0, 127);
        "RSDeadZone" => rs_zona_muerta: Entero = 10, |v: i32| v.clamp(0, 127);
        "LSAntiDeadZone" => ls_antizona: Entero = 20;
        "RSAntiDeadZone" => rs_antizona: Entero = 20;
        "LSMaxZone" => ls_zona_maxima: Entero = 100, |v: i32| v.clamp(0, 100);
        "RSMaxZone" => rs_zona_maxima: Entero = 100, |v: i32| v.clamp(0, 100);
        "LSVerticalScale" => ls_escala_vertical: Doble = 100.0, |v: f64| v.clamp(0.0, 400.0);
        "RSVerticalScale" => rs_escala_vertical: Doble = 100.0, |v: f64| v.clamp(0.0, 400.0);
        "LSMaxOutput" => ls_salida_maxima: Doble = 100.0, |v: f64| v.clamp(0.0, 100.0);
        "RSMaxOutput" => rs_salida_maxima: Doble = 100.0, |v: f64| v.clamp(0.0, 100.0);
        "LSMaxOutputForce" => ls_forzar_salida_maxima: BoolTexto = false;
        "RSMaxOutputForce" => rs_forzar_salida_maxima: BoolTexto = false;
        "LSDeadZoneType" => ls_tipo_zona_muerta: Enumerado<TipoZonaMuerta> = TipoZonaMuerta::Radial;
        "RSDeadZoneType" => rs_tipo_zona_muerta: Enumerado<TipoZonaMuerta> = TipoZonaMuerta::Radial;
        "LSAxialDeadOptions" => ls_zonas_axiales: Anidado<ZonasAxiales> = ZonasAxiales::default();
        "RSAxialDeadOptions" => rs_zonas_axiales: Anidado<ZonasAxiales> = ZonasAxiales::default();
        "LSRotation" => ls_rotacion: Doble = 0.0, rotacion_palanca;
        "RSRotation" => rs_rotacion: Doble = 0.0, rotacion_palanca;
        "LSFuzz" => ls_difuminado: Entero = 0, |v: i32| v.clamp(0, 100);
        "RSFuzz" => rs_difuminado: Entero = 0, |v: i32| v.clamp(0, 100);
        "LSOuterBindDead" => ls_zona_exterior: Doble = 75.0, |v: f64| v.clamp(0.0, 100.0);
        "RSOuterBindDead" => rs_zona_exterior: Doble = 75.0, |v: f64| v.clamp(0.0, 100.0);
        "LSOuterBindInvert" => ls_invertir_exterior: BoolTexto = false;
        "RSOuterBindInvert" => rs_invertir_exterior: BoolTexto = false;
        "LSDeltaAccelSettings" => ls_aceleracion: Anidado<AceleracionPalanca> = AceleracionPalanca::default();
        "RSDeltaAccelSettings" => rs_aceleracion: Anidado<AceleracionPalanca> = AceleracionPalanca::default();
        "SXDeadZone" => sx_zona_muerta: Doble = 0.25;
        "SZDeadZone" => sz_zona_muerta: Doble = 0.25;
        "SXMaxZone" => sx_zona_maxima: Doble = 100.0, porcentaje;
        "SZMaxZone" => sz_zona_maxima: Doble = 100.0, porcentaje;
        "SXAntiDeadZone" => sx_antizona: Doble = 0.0, porcentaje;
        "SZAntiDeadZone" => sz_antizona: Doble = 0.0, porcentaje;
        "Sensitivity" => sensibilidades: Sensibilidad = [1.0; 6];
        "ChargingType" => tipo_carga: Entero = 0;
        "MouseAcceleration" => aceleracion_raton: BoolTexto = false;
        "ButtonMouseVerticalScale" => escala_vertical_raton_botones: Doble = 100.0, |v: f64| v.clamp(0.0, 500.0) * 0.01 * 100.0;
        "LaunchProgram" => programa_al_cargar: Texto = String::new();
        "DinputOnly" => solo_dinput: BoolTexto = false;
        "StartTouchpadOff" => empezar_tactil_apagado: BoolTexto = false;
        "TouchpadOutputMode" => salida_tactil: Enumerado<SalidaPanel> = SalidaPanel::Mouse;
        "SATriggers" => gatillos_giroscopio: Texto = "-1".into();
        "SATriggerCond" => condicion_giroscopio: Condicion = true;
        "SASteeringWheelEmulationAxis" => eje_volante: Enumerado<EjeVolante> = EjeVolante::None;
        "SASteeringWheelEmulationRange" => rango_volante: Entero = 360;
        "SASteeringWheelFuzz" => difuminado_volante: Entero = 0, |v: i32| v.clamp(0, 100);
        "SASteeringWheelSmoothingOptions" => suavizado_volante: Anidado<SuavizadoVolante> = SuavizadoVolante::default();
        "TouchDisInvTriggers" => gatillos_desactivar_tactil: ListaEnteros = vec![-1];
        "GyroSensitivity" => sensibilidad_giroscopio: Entero = 100;
        "GyroSensVerticalScale" => escala_vertical_giroscopio: Entero = 100;
        "GyroInvert" => invertir_giroscopio: Entero = 0;
        "GyroTriggerTurns" => gatillo_alterna_giroscopio: BoolTexto = true;
        "GyroControlsSettings" => controles_giroscopio: Anidado<ControlesGiroscopio> = ControlesGiroscopio::default();
        "GyroMouseSmoothingSettings" => suavizado_giroscopio_raton: Anidado<SuavizadoGiroscopio> = SuavizadoGiroscopio::default();
        "GyroMouseHAxis" => eje_horizontal_giroscopio_raton: Entero = 0, |v: i32| v.clamp(0, 1);
        "GyroMouseDeadZone" => zona_muerta_giroscopio_raton: Entero = 10;
        "GyroMouseMinThreshold" => umbral_giroscopio_raton: Doble = 1.0, |v: f64| v.clamp(1.0, 40.0);
        "GyroMouseToggle" => alterna_giroscopio_raton: BoolTexto = false;
        "GyroMouseJitterCompensation" => compensar_temblor_giroscopio_raton: BoolTexto = true;
        "GyroOutputMode" => salida_giroscopio: Enumerado<SalidaGiroscopio> = SalidaGiroscopio::Controls;
        "GyroMouseStickTriggers" => gatillos_giroscopio_stick: Texto = "-1".into();
        "GyroMouseStickTriggerCond" => condicion_giroscopio_stick: Condicion = true;
        "GyroMouseStickTriggerTurns" => gatillo_alterna_giroscopio_stick: BoolTexto = true;
        "GyroMouseStickHAxis" => eje_horizontal_giroscopio_stick: Entero = 0, |v: i32| v.clamp(0, 1);
        "GyroMouseStickDeadZone" => zona_muerta_giroscopio_stick: Entero = 30;
        "GyroMouseStickMaxZone" => zona_maxima_giroscopio_stick: Entero = 830, |v: i32| v.max(1);
        "GyroMouseStickOutputStick" => stick_giroscopio: Enumerado<PalancaSalida> = PalancaSalida::None;
        "GyroMouseStickOutputStickAxes" => ejes_giroscopio_stick: Enumerado<EjesSalida> = EjesSalida::None;
        "GyroMouseStickAntiDeadX" => antizona_giroscopio_stick_x: Doble = 0.4, |v: f64| v.clamp(0.0, 1.0);
        "GyroMouseStickAntiDeadY" => antizona_giroscopio_stick_y: Doble = 0.4, |v: f64| v.clamp(0.0, 1.0);
        "GyroMouseStickInvert" => invertir_giroscopio_stick: Natural = 0;
        "GyroMouseStickToggle" => alterna_giroscopio_stick: BoolTexto = false;
        "GyroMouseStickMaxOutput" => salida_maxima_giroscopio_stick: Doble = 100.0, |v: f64| v.clamp(0.0, 100.0);
        "GyroMouseStickMaxOutputEnabled" => salida_maxima_giroscopio_stick_activa: BoolTexto = false;
        "GyroMouseStickVerticalScale" => escala_vertical_giroscopio_stick: Entero = 100;
        "GyroMouseStickJitterCompensation" => compensar_temblor_giroscopio_stick: BoolTexto = false;
        "GyroMouseStickSmoothingSettings" => suavizado_giroscopio_stick: Anidado<SuavizadoGiroscopio> = SuavizadoGiroscopio::default();
        "GyroSwipeSettings" => deslizar_giroscopio: Anidado<DeslizarGiroscopio> = DeslizarGiroscopio::default();
        "BTPollRate" => sondeo_bluetooth: Entero = 4, |v: i32| v.clamp(1, 16);
        "LSOutputCurveMode" => ls_curva: ModoCurva = Curva::Lineal;
        "LSOutputCurveCustom" => ls_curva_personalizada: TextoOpcional = None;
        "RSOutputCurveMode" => rs_curva: ModoCurva = Curva::Lineal;
        "RSOutputCurveCustom" => rs_curva_personalizada: TextoOpcional = None;
        "LSSquareStick" => ls_cuadrado: BoolTexto = false;
        "RSSquareStick" => rs_cuadrado: BoolTexto = false;
        "SquareStickRoundness" => ls_redondez: Doble = 5.0;
        "SquareRStickRoundness" => rs_redondez: Doble = 5.0;
        "LSAntiSnapback" => ls_antirrebote: BoolTexto = false;
        "RSAntiSnapback" => rs_antirrebote: BoolTexto = false;
        "LSAntiSnapbackDelta" => ls_antirrebote_delta: Doble = 135.0;
        "RSAntiSnapbackDelta" => rs_antirrebote_delta: Doble = 135.0;
        "LSAntiSnapbackTimeout" => ls_antirrebote_tiempo: Entero = 50;
        "RSAntiSnapbackTimeout" => rs_antirrebote_tiempo: Entero = 50;
        "LSOutputMode" => ls_modo: Enumerado<ModoPalanca> = ModoPalanca::Controls;
        "RSOutputMode" => rs_modo: Enumerado<ModoPalanca> = ModoPalanca::Controls;
        "LSOutputSettings" => ls_salida: Anidado<SalidaPalanca> = SalidaPalanca::default();
        "RSOutputSettings" => rs_salida: Anidado<SalidaPalanca> = SalidaPalanca::default();
        "DualSenseControllerSettings" => dualsense: Anidado<AjustesDualSense> = AjustesDualSense::default();
        "L2OutputCurveMode" => l2_curva: ModoCurva = Curva::Lineal;
        "L2OutputCurveCustom" => l2_curva_personalizada: TextoOpcional = None;
        "L2TwoStageMode" => l2_dos_fases: Enumerado<DosFases> = DosFases::Disabled;
        "R2TwoStageMode" => r2_dos_fases: Enumerado<DosFases> = DosFases::Disabled;
        "L2HipFireTime" => l2_tiempo_cadera: Entero = 100, |v: i32| v.clamp(0, 5000);
        "R2HipFireTime" => r2_tiempo_cadera: Entero = 100, |v: i32| v.clamp(0, 5000);
        "L2TriggerEffect" => l2_efecto: Enumerado<EfectoGatillo> = EfectoGatillo::None;
        "R2TriggerEffect" => r2_efecto: Enumerado<EfectoGatillo> = EfectoGatillo::None;
        "R2OutputCurveMode" => r2_curva: ModoCurva = Curva::Lineal;
        "R2OutputCurveCustom" => r2_curva_personalizada: TextoOpcional = None;
        "SXOutputCurveMode" => sx_curva: ModoCurva = Curva::Lineal;
        "SXOutputCurveCustom" => sx_curva_personalizada: TextoOpcional = None;
        "SZOutputCurveMode" => sz_curva: ModoCurva = Curva::Lineal;
        "SZOutputCurveCustom" => sz_curva_personalizada: TextoOpcional = None;
        "TrackballMode" => bola: BoolTexto = false;
        "TrackballFriction" => friccion_bola: Doble = 10.0;
        "TouchRelMouseRotation" => rotacion_raton_tactil: Doble = 0.0, rotacion_raton_tactil;
        "TouchRelMouseMinThreshold" => umbral_raton_tactil: Doble = 1.0, |v: f64| v.clamp(1.0, 40.0);
        "TouchpadAbsMouseSettings" => raton_absoluto_tactil: Anidado<RatonAbsolutoPanel> = RatonAbsolutoPanel::default();
        "TouchpadMouseStick" => tactil_como_stick: Anidado<PanelComoStick> = PanelComoStick::default();
        "TouchpadButtonMode" => activacion_tactil: Enumerado<ActivacionPanel> = ActivacionPanel::Click;
        "AbsMouseRegionSettings" => region_raton_absoluto: Anidado<RegionRatonAbsoluto> = RegionRatonAbsoluto::default();
        "OutputContDevice" => mando_virtual: Enumerado<MandoVirtual> = MandoVirtual::X360;
    }
}

/// Un perfil completo.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Perfil {
    pub ajustes: Ajustes,
    /// `DS4OutputTriggerMode`: solo se escribe si no es el valor por defecto.
    pub salida_gatillos: Option<SalidaGatillos>,
    /// Las acciones especiales que usa el perfil, por su nombre.
    pub acciones: Vec<String>,
    pub mapeos: Mapeos,
}

/// Por qué no se puede cargar un perfil. La versión en C# tampoco lo carga y se queda con los
/// valores por defecto ("Failed to load").
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorPerfil(pub String);

impl std::fmt::Display for ErrorPerfil {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ErrorPerfil {}

/// El encabezado que escribe la app al guardar: comentarios con la fecha y la versión y los
/// atributos de versión del elemento raíz.
#[derive(Debug, Clone)]
pub struct Encabezado {
    /// La fecha ya escrita, como la escribe C# en español: `20/11/2026 18:00:36`.
    pub fecha: String,
    pub version_app: String,
}

impl Perfil {
    /// Lee un perfil como `LoadProfileNew`: primero lo migra si es de una versión antigua.
    pub fn leer(texto: &str) -> Result<Perfil, ErrorPerfil> {
        let raiz = migracion::migrar_perfil(texto).map_err(|e| ErrorPerfil(e.0))?;
        Self::desde_elemento(&raiz)
    }

    fn desde_elemento(raiz: &Elemento) -> Result<Perfil, ErrorPerfil> {
        if raiz.nombre != "DS4Windows" {
            return Err(ErrorPerfil(format!("el elemento raíz es {}", raiz.nombre)));
        }
        let mut perfil = Perfil::default();
        perfil.ajustes.leer_campos(raiz).map_err(ErrorPerfil)?;
        let a = &mut perfil.ajustes;

        // Colores antiguos por componentes, solo si no hay `Color`. C# lee el verde y el azul
        // del rojo (un fallo heredado), así que se reproduce igual.
        let vacio = |n: &str| raiz.hijo(n).is_none_or(|e| e.texto_campo().is_empty());
        let componente = |n: &str| {
            raiz.hijo(n)
                .map(|e| e.texto_campo())
                .filter(|t| !t.is_empty())
        };
        let byte = |t: &str| numeros::entero_en::<u8>(t).unwrap_or(0);
        if vacio("Color") {
            let rojo = componente("Red");
            if let Some(r) = &rojo {
                a.color.0 = byte(r);
            }
            if componente("Green").is_some() {
                a.color.1 = rojo.as_deref().map_or(0, byte);
            }
            if componente("Blue").is_some() {
                a.color.2 = rojo.as_deref().map_or(0, byte);
            }
        }
        for (nombre, color, partes) in [
            (
                "LowColor",
                &mut a.color_bajo,
                ["LowRed", "LowGreen", "LowBlue"],
            ),
            (
                "ChargingColor",
                &mut a.color_carga,
                ["ChargingRed", "ChargingGreen", "ChargingBlue"],
            ),
        ] {
            if vacio(nombre) {
                if let Some(t) = componente(partes[0]) {
                    color.0 = byte(&t);
                }
                if let Some(t) = componente(partes[1]) {
                    color.1 = byte(&t);
                }
                if let Some(t) = componente(partes[2]) {
                    color.2 = byte(&t);
                }
            }
        }

        // `ShiftModifier` ya no se usa, pero si no es un entero el archivo no se carga.
        if let Some(e) = raiz.hijo("ShiftModifier") {
            numeros::entero_en::<i32>(&e.texto()).ok_or_else(|| {
                ErrorPerfil(format!("ShiftModifier no es un entero: {:?}", e.texto()))
            })?;
        }
        // Opciones antiguas que mandan sobre el modo de salida del panel y del giroscopio.
        if let Some(e) = raiz.hijo("UseTPforControls") {
            a.salida_tactil = if numeros::bool_net(&e.texto()).unwrap_or(false) {
                SalidaPanel::Controls
            } else {
                SalidaPanel::Mouse
            };
        }
        if let Some(e) = raiz.hijo("UseSAforMouse") {
            a.salida_giroscopio = if numeros::bool_net(&e.texto()).unwrap_or(false) {
                SalidaGiroscopio::Mouse
            } else {
                SalidaGiroscopio::Controls
            };
        }

        if let Some(e) = raiz.hijo("DS4OutputTriggerMode") {
            use crate::formato::Formato;
            let modo = Enumerado::<SalidaGatillos>::leer(e).map_err(ErrorPerfil)?;
            perfil.salida_gatillos = Some(modo).filter(|m| *m != SalidaGatillos::Default);
        }
        if let Some(e) = raiz.hijo("ProfileActions") {
            use crate::formato::Formato;
            perfil.acciones = AccionesPerfil::leer(e).map_err(ErrorPerfil)?;
        }
        perfil.mapeos =
            Mapeos::leer(raiz.hijo("Control"), raiz.hijo("ShiftControl")).map_err(ErrorPerfil)?;
        Ok(perfil)
    }

    /// El archivo tal como lo guarda la app. Sin encabezado, como en las pruebas de C#.
    pub fn escribir(&self, encabezado: Option<&Encabezado>) -> String {
        use crate::formato::Formato;
        let mut w = Escritor::new();
        w.crudo("<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n");
        let atributos: Vec<(&str, &str)> = match encabezado {
            Some(e) => {
                w.crudo(&format!(
                    "<!-- DS5Control Configuration Data. {} -->\r\n",
                    e.fecha
                ));
                w.crudo(&format!(
                    "<!-- Made with DS5Control version {} -->\r\n",
                    e.version_app
                ));
                vec![
                    ("app_version", e.version_app.as_str()),
                    ("config_version", migracion::VERSION_PERFIL_TEXTO),
                ]
            }
            None => Vec::new(),
        };
        w.crudo("\r\n");
        w.abrir("DS4Windows", &atributos);
        self.ajustes.escribir_campos(&mut w);
        if let Some(modo) = self
            .salida_gatillos
            .filter(|m| *m != SalidaGatillos::Default)
        {
            Enumerado::<SalidaGatillos>::escribir(&modo, "DS4OutputTriggerMode", &mut w);
        }
        AccionesPerfil::escribir(&self.acciones, "ProfileActions", &mut w);
        self.mapeos.escribir(&mut w);
        w.cerrar();
        w.terminar()
    }
}
