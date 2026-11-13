use crate::planeta::{DefLuna, DefPlaneta, Terreno};
use std::f32::consts::PI;

pub const ANCHO: usize = 1280;
pub const ALTO: usize = 720;
pub const FOV_GRADOS: f32 = 62.0;
pub const PLANO_CERCA: f32 = 0.1;
pub const PLANO_LEJOS: f32 = 6000.0;

pub const SEMILLA: u64 = 2026;
pub const VELOCIDAD_TIEMPO: f32 = 1.0;
pub const PASO_SIMULACION: f32 = 1.0 / 120.0;

pub const RADIO_SOL: f32 = 42.0;
pub const NIVEL_MAXIMO: usize = 7;

pub const PLANETAS: &[DefPlaneta] = &[
    DefPlaneta {
        nombre: "TECNO",
        radio: 10.0,
        radio_orbita: 135.0,
        periodo_orbita: 140.0,
        periodo_rotacion: 45.0,
        inclinacion_orbita: 0.05,
        inclinacion_eje: 0.35,
        fase: 70.0 * PI / 180.0,
        amplitud: 0.45,
        frecuencia: PLACAS_TECNO,
        terreno: Terreno::Tecno,
        cantidad: 46,
        tamano_objetos: 1.0,
        anillos: false,
    },
    DefPlaneta {
        nombre: "LAVA",
        radio: 8.0,
        radio_orbita: 92.0,
        periodo_orbita: 95.0,
        periodo_rotacion: 30.0,
        inclinacion_orbita: -0.08,
        inclinacion_eje: -0.2,
        fase: 140.0 * PI / 180.0,
        amplitud: 0.9,
        frecuencia: 1.5,
        terreno: Terreno::Lava,
        cantidad: 45,
        tamano_objetos: 0.75,
        anillos: false,
    },
    DefPlaneta {
        nombre: "HIELO",
        radio: 12.0,
        radio_orbita: 185.0,
        periodo_orbita: 200.0,
        periodo_rotacion: 60.0,
        inclinacion_orbita: 0.1,
        inclinacion_eje: 0.55,
        fase: 40.0 * PI / 180.0,
        amplitud: 1.3,
        frecuencia: 4.0,
        terreno: Terreno::Hielo,
        cantidad: 60,
        tamano_objetos: 1.2,
        anillos: false,
    },
    DefPlaneta {
        nombre: "DULCE",
        radio: 9.0,
        radio_orbita: 235.0,
        periodo_orbita: 260.0,
        periodo_rotacion: 38.0,
        inclinacion_orbita: -0.04,
        inclinacion_eje: -0.4,
        fase: 200.0 * PI / 180.0,
        amplitud: 0.55,
        frecuencia: 8.0,
        terreno: Terreno::Dulce,
        cantidad: 55,
        tamano_objetos: 0.9,
        anillos: true,
    },
    DefPlaneta {
        nombre: "ROTO",
        radio: 11.0,
        radio_orbita: 290.0,
        periodo_orbita: 320.0,
        periodo_rotacion: 70.0,
        inclinacion_orbita: 0.06,
        inclinacion_eje: 0.3,
        fase: 125.0 * PI / 180.0,
        amplitud: 0.35,
        frecuencia: 2.5,
        terreno: Terreno::Roto,
        cantidad: 40,
        tamano_objetos: 0.9,
        anillos: false,
    },
    DefPlaneta {
        nombre: "NAVIDAD",
        radio: 10.0,
        radio_orbita: 345.0,
        periodo_orbita: 380.0,
        periodo_rotacion: 55.0,
        inclinacion_orbita: -0.05,
        inclinacion_eje: 0.15,
        fase: 60.0 * PI / 180.0,
        amplitud: 0.45,
        frecuencia: 0.0,
        terreno: Terreno::Navidad,
        cantidad: 50,
        tamano_objetos: 1.0,
        anillos: false,
    },
];

pub const LUNAS: &[DefLuna] = &[DefLuna {
    padre: 0,
    radio: 2.6,
    distancia: 1.72,
    periodo: 48.0,
    inclinacion: 0.3,
    profundidad: 0.35,
}];

pub const ANILLOS: &[(f32, f32)] = &[(1.45, 1.66), (1.74, 2.04), (2.12, 2.32)];

pub const REDONDEZ_CUBO: f32 = 8.0;
pub const PLACAS_TECNO: f32 = 5.0;

pub const FRAGMENTOS_ROTO: usize = 9;
pub const SEPARACION_ROTO: f32 = 0.22;
pub const GROSOR_ROTO: f32 = 0.42;
pub const NUCLEO_ROTO: f32 = 0.3;
pub const ESCOMBROS_ROTO: usize = 28;

pub const ALTURA_PICO: f32 = 1.1;
pub const VUELTAS_GUIRNALDA: f32 = 0.9;
pub const PISOS_ARBOL: f32 = 2.2;
pub const FOCOS_POR_VUELTA: f32 = 22.0;
pub const COPOS: usize = 220;
pub const ALTURA_NIEVE: f32 = 9.0;
pub const CAIDA_NIEVE: f32 = 7.0;
pub const TAMANO_COPO: f32 = 0.07;

pub const INTENSIDAD_NEON: f32 = 1.0;
pub const INTENSIDAD_LAVA: f32 = 1.0;
pub const INTENSIDAD_SOL: f32 = 1.0;

pub const VOLCANES: usize = 6;
pub const CICLO_ERUPCION: f32 = 9.0;
pub const ERUPCION_ACTIVA: f32 = 4.5;
pub const DESFASE_VOLCANES: f32 = 2.7;
pub const CHISPAS_POR_VOLCAN: usize = 46;
pub const VIDA_CHISPA: f32 = 1.5;
pub const VELOCIDAD_ERUPCION: f32 = 6.5;
pub const VELOCIDAD_LATERAL: f32 = 2.6;
pub const GRAVEDAD_ERUPCION: f32 = 6.0;
pub const LARGO_CHISPA: f32 = 0.09;
pub const ANCHO_CHISPA: f32 = 0.035;

pub const DISTANCIA_AGARRE: f32 = 6.0;
pub const ALTURA_ANILLO: f32 = 1.0;
pub const ALTURA_MINIMA_ANILLO: f32 = 0.5;
pub const SALIDA_ANILLO: f32 = 5.0;

pub const LARGO_NAVE: f32 = 2.2;
pub const VIDA_ESTELA: f32 = 0.7;
pub const RITMO_ESTELA: f32 = 45.0;
pub const VELOCIDAD_ESTELA: f32 = 5.0;
pub const TAMANO_ESTELA: f32 = 0.055;
pub const LARGO_LLAMA: f32 = 1.3;
pub const CELDA_POLVO: f32 = 9.0;
pub const RAPIDEZ_POLVO: f32 = 8.0;
pub const POSICION_INICIAL: (f32, f32, f32) = (6.0, 18.0, 290.0);

pub const EMPUJE: f32 = 28.0;
pub const RESISTENCIA: f32 = 0.45;
pub const RESISTENCIA_FRENO: f32 = 3.0;
pub const FACTOR_TURBO: f32 = 3.0;
pub const VELOCIDAD_GIRO: f32 = 1.5;
pub const RESPUESTA_GIRO: f32 = 5.0;

pub const ZONA_ENTRADA: f32 = 2.4;
pub const ZONA_SALIDA: f32 = 3.2;
pub const EMPUJE_LOCAL: f32 = 16.0;
pub const RESISTENCIA_LOCAL: f32 = 1.5;
pub const VELOCIDAD_ALTURA: f32 = 6.0;
pub const RESPUESTA_ALTURA: f32 = 5.0;
pub const RESPUESTA_ALINEACION: f32 = 3.5;
pub const MARGEN_COLISION: f32 = 0.9;

pub const DISTANCIA_CAMARA: f32 = 7.5;
pub const ALTURA_CAMARA: f32 = 2.6;
pub const DISTANCIA_CAMARA_LOCAL: f32 = 15.0;
pub const ALTURA_CAMARA_LOCAL: f32 = 11.0;
pub const MIRADA_ADELANTE_LOCAL: f32 = 2.0;
pub const MIRADA_ABAJO_LOCAL: f32 = 7.0;
pub const RESPUESTA_CAMARA: f32 = 6.0;
