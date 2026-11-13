mod cielo;
mod config;
mod efectos;
mod geometria;
mod hud;
mod mundo;
mod nave;
mod obj;
mod planeta;
mod render;
mod ruido;
mod shaders;
mod vector;

use config::*;
use efectos::Estela;
use hud::Lienzo;
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use mundo::Mundo;
use nave::{Entrada, Modo, Persecucion, Simulacion};
use render::{Camara, Renderer};
use shaders::Global;
use std::time::Instant;
use vector::Vec3;

const BLANCO: u32 = 0xF2F6FF;
const AMARILLO: u32 = 0xFFD84A;
const CELESTE: u32 = 0x7FD4FF;

struct Opciones {
    captura: Option<String>,
    vista: String,
    tiempo: f32,
    ancho: usize,
    alto: usize,
    medir: usize,
    planeta: usize,
    semilla: u64,
    sin_ayudas: bool,
    sin_descarte: bool,
    sin_lod: bool,
    empuje: f32,
}

fn leer_opciones() -> Opciones {
    let mut o = Opciones {
        captura: None,
        vista: "general".to_string(),
        tiempo: 0.0,
        ancho: ANCHO,
        alto: ALTO,
        medir: 1,
        planeta: 0,
        semilla: SEMILLA,
        sin_ayudas: false,
        sin_descarte: false,
        sin_lod: false,
        empuje: 0.0,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    for par in args.chunks(2) {
        let [nombre, valor] = par else { continue };
        match nombre.as_str() {
            "--captura" => o.captura = Some(valor.clone()),
            "--vista" => o.vista = valor.clone(),
            "--tiempo" => o.tiempo = valor.parse().unwrap_or(0.0),
            "--ancho" => o.ancho = valor.parse().unwrap_or(ANCHO),
            "--alto" => o.alto = valor.parse().unwrap_or(ALTO),
            "--medir" => o.medir = valor.parse().unwrap_or(1),
            "--planeta" => o.planeta = valor.parse::<usize>().unwrap_or(0).min(PLANETAS.len() - 1),
            "--semilla" => o.semilla = valor.parse().unwrap_or(SEMILLA),
            "--sin-ayudas" => o.sin_ayudas = valor == "si",
            "--sin-descarte" => o.sin_descarte = valor == "si",
            "--sin-lod" => o.sin_lod = valor == "si",
            "--empuje" => o.empuje = valor.parse().unwrap_or(0.0),
            _ => {}
        }
    }
    o
}

fn leer_entrada(ventana: &Window, local: bool) -> Entrada {
    let eje = |mas: Key, menos: Key| {
        ventana.is_key_down(mas) as i32 as f32 - ventana.is_key_down(menos) as i32 as f32
    };
    let flechas_v = eje(Key::Down, Key::Up);
    let extra_v = eje(Key::R, Key::F);
    Entrada {
        avance: eje(Key::W, Key::S),
        lateral: eje(Key::D, Key::A),
        vertical: if local { (flechas_v + extra_v).clamp(-1.0, 1.0) } else { 0.0 },
        cabeceo: if local { 0.0 } else { flechas_v },
        guinada: eje(Key::Left, Key::Right),
        alabeo: eje(Key::E, Key::Q),
        freno: ventana.is_key_down(Key::Space),
        turbo: ventana.is_key_down(Key::LeftShift) || ventana.is_key_down(Key::RightShift),
    }
}

fn colocar_vista(sim: &mut Simulacion, vista: &str, planeta: usize) {
    let def = &PLANETAS[planeta];
    let centro = def.posicion(sim.tiempo);
    let hacia_sol = (Vec3::CERO - centro).normalizado();
    let lado = hacia_sol.cross(Vec3::new(0.0, 1.0, 0.0)).normalizado();
    let n = &mut sim.nave;
    match vista {
        "acercamiento" => {
            n.pos = centro + (hacia_sol * 0.8 + lado * 0.6 + Vec3::new(0.0, 0.25, 0.0)).normalizado() * (def.radio * 2.05).max(def.radio_limite() * 1.9);
            n.adelante = (centro - n.pos).normalizado();
            n.arriba = (Vec3::new(0.0, 1.0, 0.0) - n.adelante * n.adelante.y).normalizado();
        }
        "superficie" | "rasante" => {
            let altura = if vista == "rasante" { 1.8 } else { 3.4 } + def.radio_limite() - def.radio;
            let radial = (hacia_sol * 0.9 + lado * 0.4 + Vec3::new(0.0, 0.3, 0.0)).normalizado();
            n.pos = centro + radial * (def.radio + altura);
            n.arriba = radial;
            n.adelante = lado.cross(radial).normalizado();
            n.altura_objetivo = altura;
            n.modo = Modo::Local(planeta);
        }
        "luna" => {
            let l = &LUNAS[0];
            let c = l.posicion(sim.tiempo);
            let hacia = (Vec3::CERO - c).normalizado();
            n.pos = c + (hacia + lado * 0.5).normalizado() * (l.radio * 4.5);
            n.adelante = (c - n.pos).normalizado();
            n.arriba = (Vec3::new(0.0, 1.0, 0.0) - n.adelante * n.adelante.y).normalizado();
            n.local_anulado = true;
        }
        "sol" => {
            n.pos = Vec3::new(0.0, 0.0, RADIO_SOL + 12.0);
        }
        "junto_anillo" => {
            let normal = def.normal_anillo();
            let afuera = (hacia_sol - normal * hacia_sol.dot(normal)).normalizado();
            let medio = (ANILLOS[0].0 + ANILLOS[ANILLOS.len() - 1].1) * 0.5 * def.radio;
            n.pos = centro + afuera * medio + normal * 3.5;
            n.arriba = normal;
            n.adelante = normal.cross(afuera);
        }
        "anillo" => {
            let normal = def.normal_anillo();
            let afuera = (hacia_sol - normal * hacia_sol.dot(normal)).normalizado();
            let medio = (ANILLOS[0].0 + ANILLOS[ANILLOS.len() - 1].1) * 0.5 * def.radio;
            n.pos = centro + afuera * medio + normal * ALTURA_ANILLO;
            n.arriba = normal;
            n.adelante = normal.cross(afuera);
            n.altura_objetivo = ALTURA_ANILLO;
            n.modo = Modo::Anillo(planeta);
        }
        "sol_lejos" => {
            n.pos = Vec3::new(20.0, 25.0, RADIO_SOL * 3.6);
            n.adelante = (Vec3::CERO - n.pos).normalizado();
            n.arriba = (Vec3::new(0.0, 1.0, 0.0) - n.adelante * n.adelante.y).normalizado();
        }
        _ => {}
    }
}

fn dibujar_marcadores(r: &mut Renderer, mundo: &Mundo, camara: &Camara, t: f32, modo: Modo) {
    let mut objetivos: Vec<(String, Vec3, f32)> = vec![("SOL".to_string(), Vec3::CERO, RADIO_SOL)];
    for (i, p) in mundo.planetas.iter().enumerate() {
        if modo != Modo::Local(i) {
            objetivos.push((p.def.nombre.to_string(), p.def.posicion(t), p.def.radio));
        }
    }
    let vistos: Vec<_> = objetivos
        .iter()
        .filter_map(|(nombre, pos, radio)| {
            let (x, y) = r.a_pantalla(camara, *pos)?;
            let d = (*pos - camara.pos).largo();
            Some((nombre.clone(), x as i32, y as i32, d, *radio))
        })
        .collect();
    let (ancho, alto) = (r.ancho, r.alto);
    let mut lienzo = Lienzo { color: &mut r.color, ancho, alto };
    let fov = FOV_GRADOS.to_radians();
    for (nombre, x, y, d, radio) in vistos {
        if d < radio * 3.2 {
            continue;
        }
        let px = (radio / d / (fov * 0.5).tan() * alto as f32 * 0.5) as i32 + 10;
        let texto = format!("{nombre} {}", d as i32);
        lienzo.marcador(x, y, px, CELESTE);
        lienzo.texto(x - Lienzo::ancho_texto(&texto, 1) / 2, y + px + 6, &texto, CELESTE, 1);
    }
}

fn dibujar_hud(r: &mut Renderer, sim: &Simulacion, ms: f32, niveles: &[usize], lod: bool, ayuda: bool) {
    let e = r.estadisticas;
    let descarte = r.descarte;
    let (ancho, alto) = (r.ancho, r.alto);
    let mut l = Lienzo { color: &mut r.color, ancho, alto };

    l.panel(10, 10, 300, 92);
    l.texto(18, 18, &format!("{:.0} FPS  {ms:.1} MS", 1000.0 / ms.max(0.01)), BLANCO, 2);
    l.texto(18, 42, &format!("TRIANGULOS ENVIADOS {}", e.triangulos_enviados), BLANCO, 1);
    l.texto(18, 54, &format!("TRIANGULOS DIBUJADOS {}", e.triangulos_dibujados), BLANCO, 1);
    l.texto(18, 66, &format!("OBJETOS {} DE {}", e.objetos_visibles, e.objetos_total), BLANCO, 1);
    let lista: Vec<String> = niveles.iter().map(|n| n.to_string()).collect();
    l.texto(18, 78, &format!("DETALLE PLANETAS {}", lista.join(" ")), BLANCO, 1);
    l.texto(18, 90, &format!("(1) DESCARTE {}  (2) LOD {}", si_no(descarte), si_no(lod)), AMARILLO, 1);

    let velocidad = sim.nave.vel.largo();
    let modo = match sim.nave.modo {
        Modo::Vuelo => "VUELO ESPACIAL".to_string(),
        Modo::Local(i) => format!("ORBITA LOCAL - {}", PLANETAS[i].nombre),
        Modo::Anillo(i) => format!("ANILLO - {}", PLANETAS[i].nombre),
    };
    let y_base = alto as i32 - 40;
    l.panel(10, y_base - 8, 300, 40);
    l.texto(18, y_base, &modo, AMARILLO, 2);
    l.texto(18, y_base + 20, &format!("VELOCIDAD {velocidad:.1}   (H) AYUDA"), BLANCO, 1);

    if !matches!(sim.nave.modo, Modo::Anillo(_)) && sim.nave.anillo_cercano(sim.tiempo).is_some() {
        let aviso = "G  RECORRER EL ANILLO";
        let ancho_aviso = Lienzo::ancho_texto(aviso, 2);
        let x = ancho as i32 / 2 - ancho_aviso / 2;
        let y = alto as i32 - 90;
        l.panel(x - 10, y - 8, ancho_aviso + 20, 30);
        l.texto(x, y, aviso, AMARILLO, 2);
    }

    if ayuda {
        let mut lineas: Vec<&str> = match sim.nave.modo {
            Modo::Vuelo => vec![
                "W/S  ACELERAR Y RETROCEDER",
                "FLECHAS  GIRAR LA NAVE",
                "A/D  DESPLAZAR  Q/E  ALABEO",
                "SHIFT  TURBO  ESPACIO  FRENO",
                "MOUSE  ARRASTRAR PARA MIRAR",
                "ACERCATE A UN PLANETA PARA RODEARLO",
            ],
            Modo::Local(_) => vec![
                "W/S  AVANZAR ALREDEDOR DEL PLANETA",
                "A/D  DESPLAZAR  IZQ/DER  GIRAR",
                "ABAJO/ARRIBA O R/F  SUBIR/BAJAR",
                "SUBE PARA VOLVER AL ESPACIO",
                "L  SALIR DE ORBITA LOCAL",
                "MOUSE  ARRASTRAR PARA MIRAR",
            ],
            Modo::Anillo(_) => vec![
                "W/S  RECORRER EL ANILLO",
                "A/D  DESPLAZAR  IZQ/DER  GIRAR",
                "CRUZA EL BORDE PARA IR AL OTRO LADO",
                "ABAJO O R  SUBIR",
                "G  SOLTAR EL ANILLO",
                "MOUSE  ARRASTRAR PARA MIRAR",
            ],
        };
        if let Modo::Local(i) = sim.nave.modo {
            if PLANETAS[i].anillos {
                lineas.push("ACERCATE AL ANILLO Y PULSA G");
            }
        }
        let x = ancho as i32 - 296;
        l.panel(x - 8, 10, 294, 16 + lineas.len() as i32 * 12);
        for (i, linea) in lineas.iter().enumerate() {
            l.texto(x, 18 + i as i32 * 12, linea, BLANCO, 1);
        }
    }
}

fn si_no(valor: bool) -> &'static str {
    if valor { "SI" } else { "NO" }
}

fn capturar(mundo: &Mundo, o: &Opciones, ruta: &str) -> Result<(), String> {
    let mut sim = Simulacion::new(o.tiempo);
    colocar_vista(&mut sim, &o.vista, o.planeta);
    sim.nave.empuje = o.empuje;
    sim.nave.vel = sim.nave.adelante * (40.0 * o.empuje);
    let mut estela = Estela::new();
    let paso = 1.0 / 60.0;
    for k in 0..60 {
        let t = sim.tiempo - (60 - k) as f32 * paso;
        estela.avanzar(&mundo.toberas_mundo(&sim.nave), -sim.nave.adelante, o.empuje, t, paso);
    }
    let persecucion = Persecucion::new(&sim.nave);
    let mut r = Renderer::new(o.ancho, o.alto);
    r.descarte = !o.sin_descarte;
    let lod = !o.sin_lod;
    let mut camara = persecucion.camara(&sim.nave);
    mundo.limitar_camara(sim.tiempo, &mut camara);
    let global = Global {
        tiempo: sim.tiempo,
        camara: camara.pos,
        sol: Vec3::CERO,
        ruido: &mundo.ruido,
    };
    let inicio = Instant::now();
    let mut niveles = Vec::new();
    for _ in 0..o.medir {
        let (lista, usados) = mundo.instancias(sim.tiempo, &sim.nave, &estela.particulas, &camara, o.alto as f32, lod, r.descarte);
        niveles = usados;
        let resplandor = mundo.resplandor(sim.tiempo, &camara);
        r.dibujar(&lista, &camara, &global, &mundo.cielo, &mundo.orbitas, Some(resplandor));
    }
    let ms = inicio.elapsed().as_secs_f32() * 1000.0 / o.medir as f32;
    if !o.sin_ayudas {
        dibujar_marcadores(&mut r, mundo, &camara, sim.tiempo, sim.nave.modo);
        dibujar_hud(&mut r, &sim, ms, &niveles, lod, true);
    }
    println!(
        "{}: {ms:.1} ms, enviados {}, dibujados {}, objetos {}/{}, niveles {niveles:?}",
        o.vista,
        r.estadisticas.triangulos_enviados,
        r.estadisticas.triangulos_dibujados,
        r.estadisticas.objetos_visibles,
        r.estadisticas.objetos_total
    );
    let mut datos = format!("P6\n{} {}\n255\n", o.ancho, o.alto).into_bytes();
    for c in &r.color {
        datos.extend_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, *c as u8]);
    }
    std::fs::write(ruta, datos).map_err(|e| e.to_string())
}

fn jugar(mundo: &Mundo) -> Result<(), String> {
    let mut ventana = Window::new(
        "Sistema solar - renderer por software",
        ANCHO,
        ALTO,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| e.to_string())?;
    ventana.set_target_fps(60);

    let mut r = Renderer::new(ANCHO, ALTO);
    let mut sim = Simulacion::new(0.0);
    let mut estela = Estela::new();
    let mut persecucion = Persecucion::new(&sim.nave);
    let mut anterior = Instant::now();
    let mut mouse_previo: Option<(f32, f32)> = None;
    let (mut lod, mut ayuda) = (true, true);
    let mut ms_suave = 16.0;

    while ventana.is_open() && !ventana.is_key_down(Key::Escape) {
        let ahora = Instant::now();
        let dt = (ahora - anterior).as_secs_f32();
        anterior = ahora;

        let (w, h) = ventana.get_size();
        if (w, h) != (r.ancho, r.alto) && w > 0 && h > 0 {
            r.redimensionar(w, h);
        }

        if ventana.is_key_pressed(Key::Key1, KeyRepeat::No) {
            r.descarte = !r.descarte;
        }
        if ventana.is_key_pressed(Key::Key2, KeyRepeat::No) {
            lod = !lod;
        }
        if ventana.is_key_pressed(Key::H, KeyRepeat::No) {
            ayuda = !ayuda;
        }
        if ventana.is_key_pressed(Key::G, KeyRepeat::No) {
            sim.nave.alternar_anillo(sim.tiempo);
        }
        if ventana.is_key_pressed(Key::L, KeyRepeat::No) {
            sim.nave.alternar_local(&mundo.ruido, sim.tiempo);
        }

        let local = sim.nave.modo != Modo::Vuelo;
        sim.avanzar(dt, &leer_entrada(&ventana, local), &mundo.ruido);
        estela.avanzar(&mundo.toberas_mundo(&sim.nave), -sim.nave.adelante, sim.nave.empuje, sim.tiempo, dt);

        let arrastre = if ventana.get_mouse_down(MouseButton::Left) {
            let actual = ventana.get_mouse_pos(MouseMode::Pass);
            let delta = match (mouse_previo, actual) {
                (Some(p), Some(a)) => (a.0 - p.0, a.1 - p.1),
                _ => (0.0, 0.0),
            };
            mouse_previo = actual;
            Some(delta)
        } else {
            mouse_previo = None;
            None
        };
        persecucion.actualizar(&sim.nave, dt, arrastre);

        let mut camara = persecucion.camara(&sim.nave);
        mundo.limitar_camara(sim.tiempo, &mut camara);
        let global = Global {
            tiempo: sim.tiempo,
            camara: camara.pos,
            sol: Vec3::CERO,
            ruido: &mundo.ruido,
        };
        let inicio = Instant::now();
        let (lista, niveles) = mundo.instancias(sim.tiempo, &sim.nave, &estela.particulas, &camara, r.alto as f32, lod, r.descarte);
        let resplandor = mundo.resplandor(sim.tiempo, &camara);
        r.dibujar(&lista, &camara, &global, &mundo.cielo, &mundo.orbitas, Some(resplandor));
        ms_suave += (inicio.elapsed().as_secs_f32() * 1000.0 - ms_suave) * 0.08;

        dibujar_marcadores(&mut r, mundo, &camara, sim.tiempo, sim.nave.modo);
        dibujar_hud(&mut r, &sim, ms_suave, &niveles, lod, ayuda);
        ventana
            .update_with_buffer(&r.color, r.ancho, r.alto)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn main() {
    let opciones = leer_opciones();
    let resultado = Mundo::new(opciones.semilla).and_then(|mundo| match &opciones.captura {
        Some(ruta) => capturar(&mundo, &opciones, ruta),
        None => jugar(&mundo),
    });
    if let Err(e) = resultado {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
