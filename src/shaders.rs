use crate::config::{INTENSIDAD_LAVA, INTENSIDAD_NEON, INTENSIDAD_SOL, PLACAS_TECNO};
use crate::config::FOCOS_POR_VUELTA;
use crate::planeta::{forma_navidad, guirnalda, placa_cubo};
use crate::ruido::Perlin;
use crate::vector::{Vec3, suave};
use std::f32::consts::TAU;

#[derive(Clone, Copy, PartialEq)]
pub enum Shader {
    Sol,
    Tecno,
    Lava,
    Hielo,
    Dulce,
    Nave,
    Decoracion,
    Anillo,
    Luna,
    Chispa,
    Roto,
    Navidad,
    Copo,
    Llama,
    Polvo,
}

pub struct Global<'a> {
    pub tiempo: f32,
    pub camara: Vec3,
    pub sol: Vec3,
    pub ruido: &'a Perlin,
}

pub struct Fragmento {
    pub pos: Vec3,
    pub normal: Vec3,
    pub local: Vec3,
    pub color: Vec3,
    pub extra: f32,
}

const fn rgb(r: f32, g: f32, b: f32) -> Vec3 {
    Vec3::new(r, g, b)
}

fn iluminar(albedo: Vec3, n: Vec3, l: Vec3, v: Vec3) -> Vec3 {
    let d = n.dot(l) * 0.5 + 0.5;
    let difusa = d * d;
    let ambiente = rgb(0.20, 0.22, 0.42);
    let sol = rgb(1.25, 1.15, 0.95);
    let borde = (1.0 - n.dot(v).max(0.0)).powi(3);
    albedo.por(ambiente + sol * difusa) + rgb(0.35, 0.55, 1.0) * (borde * 0.45)
}

pub fn sombrear(shader: Shader, f: &Fragmento, g: &Global, centro: Vec3, param: f32) -> Vec3 {
    match shader {
        Shader::Sol => sol(f, g),
        Shader::Tecno => tecno(f, g),
        Shader::Lava => lava(f, g),
        Shader::Hielo => hielo(f, g),
        Shader::Dulce => dulce(f, g),
        Shader::Nave => nave(f, g, param),
        Shader::Decoracion => decoracion(f, g),
        Shader::Anillo => anillo(f, g, centro, param),
        Shader::Luna => luna(f, g),
        Shader::Chispa => chispa(f, g, param),
        Shader::Roto => roto(f, g, param),
        Shader::Navidad => navidad(f, g),
        Shader::Copo => copo(f, g),
        Shader::Llama => llama(f, g, param),
        Shader::Polvo => rgb(0.75, 0.85, 1.0) * param,
    }
}

fn sol(f: &Fragmento, g: &Global) -> Vec3 {
    let p = f.local.normalizado();
    let t = g.tiempo;
    let q = p * 2.2;
    let desvio = g.ruido.fbm(q + Vec3::new(t * 0.05, -t * 0.04, t * 0.03), 3);
    let plasma = g
        .ruido
        .fbm(q * 1.9 + Vec3::new(desvio * 2.6, t * 0.10, -desvio * 2.6), 4);
    let celdas = suave(-0.10, 0.06, plasma);
    let v = (g.camara - f.pos).normalizado();
    let frente = f.normal.normalizado().dot(v).max(0.0);
    let claro = rgb(1.0, 0.86, 0.22).mezclar(rgb(1.0, 0.97, 0.62), suave(0.55, 1.0, frente));
    let oscuro = rgb(1.0, 0.56, 0.06);
    let borde = rgb(0.92, 0.18, 0.06);
    let base = oscuro.mezclar(claro, celdas);
    base.mezclar(borde, (1.0 - suave(0.05, 0.42, frente)) * 0.9) * INTENSIDAD_SOL
}

fn tecno(f: &Fragmento, g: &Global) -> Vec3 {
    let d = f.local.normalizado();
    let n = f.normal.normalizado();
    let l = (g.sol - f.pos).normalizado();
    let hacia_camara = g.camara - f.pos;
    let distancia = hacia_camara.largo();
    let v = hacia_camara * (1.0 / distancia);
    let t = g.tiempo;

    let (gu, gv, cara, id) = placa_cubo(g.ruido, d, PLACAS_TECNO);
    let (fu, fv) = (gu.fract(), gv.fract());
    let alta = f.extra;
    let junta = 1.0 - suave(0.0, 0.03, fu.min(1.0 - fu).min(fv.min(1.0 - fv)));
    let albedo = rgb(0.16, 0.20, 0.30).mezclar(rgb(0.62, 0.67, 0.78), alta) * (1.0 - 0.6 * junta);
    let mitad = (l + v).normalizado();
    let metal = iluminar(albedo, n, l, v) + rgb(1.0, 1.0, 1.0) * (n.dot(mitad).max(0.0).powi(60) * 0.45);

    let (cu, cv) = (gu * 4.0, gv * 4.0);
    let fila = g.ruido.ruido(Vec3::new(cv.floor() * 1.7 + 0.3, cara * 2.3 + 0.5, 0.7));
    let columna = g.ruido.ruido(Vec3::new(cu.floor() * 1.9 + 0.6, cara * 1.3 + 0.2, 4.1));
    let ancho = |x: f32| 1.0 - suave(0.05, 0.11, (x.fract() - 0.5).abs());
    let en_fila = if fila > 0.2 { ancho(cv) } else { 0.0 };
    let en_columna = if columna > 0.2 { ancho(cu) } else { 0.0 };
    let cerca = 1.0 - suave(45.0, 95.0, distancia);
    let traza = (en_fila.max(en_columna) * (1.0 - alta) * cerca * INTENSIDAD_NEON).min(1.0);
    let avance = if en_fila >= en_columna { cu * 0.25 + fila * 7.0 } else { cv * 0.25 + columna * 7.0 };
    let pulso = suave(0.78, 1.0, (avance - t * 0.9).rem_euclid(1.0));
    let neon = rgb(0.15, 0.85, 1.0) * ((0.5 + 1.4 * pulso) * INTENSIDAD_NEON.max(1.0));

    let al_centro = ((fu - 0.5).powi(2) + (fv - 0.5).powi(2)).sqrt();
    let parpadeo = 0.5 + 0.5 * (t * 2.6 + id * 40.0).sin();
    let foco = (1.0 - suave(0.06, 0.09, al_centro)) * alta;
    let luz = rgb(1.0, 0.55, 0.15).mezclar(rgb(1.0, 0.3, 0.7), suave(-0.2, 0.2, id)) * (0.5 + 0.9 * parpadeo);

    metal.mezclar(neon, traza).mezclar(luz, foco)
}

fn nave(f: &Fragmento, g: &Global, empuje: f32) -> Vec3 {
    if f.extra > 0.5 {
        let pulso = 0.85 + 0.15 * (g.tiempo * 30.0).sin();
        return f.color.mezclar(rgb(1.0, 0.95, 0.6), empuje) * (0.8 + 0.8 * empuje * pulso);
    }
    let n = f.normal.normalizado();
    let l = (g.sol - f.pos).normalizado();
    let v = (g.camara - f.pos).normalizado();
    let mitad = (l + v).normalizado();
    let brillo = n.dot(mitad).max(0.0).powi(40);
    iluminar(f.color * 0.8, n, l, v) + rgb(1.0, 0.98, 0.9) * (brillo * 0.35)
}

fn lava(f: &Fragmento, g: &Global) -> Vec3 {
    let dir = f.local.normalizado();
    let n = f.normal.normalizado();
    let l = (g.sol - f.pos).normalizado();
    let v = (g.camara - f.pos).normalizado();
    let t = g.tiempo;

    let desvio = g.ruido.fbm(dir * 7.0, 2) * 0.09;
    let q = dir * 3.6 + Vec3::new(desvio, -desvio, desvio);
    let (f1, f2, id) = g.ruido.voronoi(q);
    let grieta = 1.0 - suave(0.015, 0.11, f2 - f1);
    let flujo = g.ruido.fbm(dir * 9.0 + Vec3::new(t * 0.22, -t * 0.16, t * 0.1), 3);
    let pulso = 0.72 + 0.28 * (t * 1.7 + id * TAU + flujo * 5.0).sin();

    let costra = rgb(0.17, 0.11, 0.12).mezclar(rgb(0.30, 0.19, 0.17), id);
    let solida = iluminar(costra, n, l, v);
    let fundida = rgb(1.0, 0.28, 0.04).mezclar(rgb(1.0, 0.86, 0.28), suave(0.15, 0.55, grieta * pulso + flujo * 0.4));
    let valle = 1.0 - suave(0.02, 0.2, f.extra);
    let brillo = (grieta * pulso).max(valle * 0.8 * (0.55 + flujo)) * INTENSIDAD_LAVA;
    solida.mezclar(fundida * INTENSIDAD_LAVA.max(1.0), brillo.clamp(0.0, 1.0))
}

fn hielo(f: &Fragmento, g: &Global) -> Vec3 {
    let dir = f.local.normalizado();
    let l = (g.sol - f.pos).normalizado();
    let v = (g.camara - f.pos).normalizado();

    let (_, _, id) = g.ruido.voronoi(dir * 11.0);
    let faceta = Vec3::new((id * 91.7).sin(), (id * 47.3).sin(), (id * 23.9).sin());
    let n = (f.normal.normalizado() + faceta * 0.22).normalizado();

    let frente = n.dot(v).max(0.0);
    let reflejo = n * (2.0 * n.dot(v)) - v;
    let cielo = rgb(0.05, 0.22, 0.52).mezclar(rgb(0.72, 0.92, 1.0), reflejo.y * 0.5 + 0.5);
    let interior = rgb(0.16, 0.52, 0.86).mezclar(rgb(0.62, 0.90, 0.98), f.extra);
    let fresnel = 0.08 + 0.92 * (1.0 - frente).powi(4);
    let difusa = n.dot(l).max(0.0) * 0.72 + 0.28;
    let mitad = (l + v).normalizado();
    let destello = n.dot(mitad).max(0.0).powi(160);
    interior.mezclar(cielo, fresnel) * difusa + rgb(1.0, 1.0, 1.0) * destello
}

fn dulce(f: &Fragmento, g: &Global) -> Vec3 {
    let dir = f.local.normalizado();
    let n = f.normal.normalizado();
    let l = (g.sol - f.pos).normalizado();
    let v = (g.camara - f.pos).normalizado();

    let a = dir.z.atan2(dir.x) + dir.y * 3.0;
    let raya = suave(-0.08, 0.08, (a * 8.0).sin());
    let mut albedo = rgb(1.0, 0.36, 0.60).mezclar(rgb(1.0, 0.94, 0.86), raya);

    let chorreado = 0.56 + 0.07 * (dir.z.atan2(dir.x) * 9.0).sin();
    let glaseado = suave(chorreado - 0.015, chorreado + 0.015, dir.y);
    if glaseado > 0.0 {
        let (d, _, id) = g.ruido.voronoi(dir * 24.0);
        let confite = if id < 0.33 {
            rgb(1.0, 0.85, 0.2)
        } else if id < 0.66 {
            rgb(0.3, 0.85, 0.95)
        } else {
            rgb(1.0, 0.98, 0.95)
        };
        let chocolate = rgb(0.34, 0.17, 0.10).mezclar(confite, 1.0 - suave(0.12, 0.17, d));
        albedo = albedo.mezclar(chocolate, glaseado);
    }

    let mitad = (l + v).normalizado();
    let brillo = n.dot(mitad).max(0.0).powi(34);
    iluminar(albedo, n, l, v) + rgb(1.0, 1.0, 1.0) * (brillo * 0.45)
}

fn decoracion(f: &Fragmento, g: &Global) -> Vec3 {
    let n = f.normal.normalizado();
    let l = (g.sol - f.pos).normalizado();
    let v = (g.camara - f.pos).normalizado();
    if f.extra > 0.5 {
        let latido = 0.82 + 0.18 * (g.tiempo * 2.2 + f.local.y * 2.5).sin();
        let cara = 0.65 + 0.35 * n.dot(l).max(0.0);
        return f.color * (latido * cara) + rgb(0.25, 0.25, 0.25) * (1.0 - n.dot(v).max(0.0)).powi(2);
    }
    iluminar(f.color, n, l, v)
}

fn anillo(f: &Fragmento, g: &Global, centro: Vec3, radio: f32) -> Vec3 {
    let l = (g.sol - f.pos).normalizado();
    let v = (g.camara - f.pos).normalizado();
    let mut n = f.normal.normalizado();
    if n.dot(v) < 0.0 {
        n = -n;
    }
    let r = f.local.largo() / radio;
    let base = if r < 1.7 {
        rgb(1.0, 0.42, 0.66)
    } else if r < 2.08 {
        rgb(0.62, 0.95, 0.82)
    } else {
        rgb(0.78, 0.62, 1.0)
    };
    let surco = suave(-0.3, 0.3, (r * 40.0).sin());
    let albedo = base.mezclar(rgb(1.0, 0.97, 0.92), surco * 0.45);

    let hacia_planeta = f.pos - centro;
    let b = hacia_planeta.dot(l);
    let c = hacia_planeta.dot(hacia_planeta) - radio * radio;
    let sombra = b < 0.0 && b * b - c > 0.0;
    let luz = iluminar(albedo, n, l, v);
    if sombra { luz * 0.38 } else { luz }
}

fn luna(f: &Fragmento, g: &Global) -> Vec3 {
    let dir = f.local.normalizado();
    let n = f.normal.normalizado();
    let l = (g.sol - f.pos).normalizado();
    let v = (g.camara - f.pos).normalizado();
    let polvo = g.ruido.fbm(dir * 9.0, 3) * 0.5 + 0.5;
    let roca = rgb(0.74, 0.71, 0.86).mezclar(rgb(0.86, 0.84, 0.94), polvo);
    let fondo = 1.0 - suave(0.6, 0.95, f.extra);
    iluminar(roca.mezclar(rgb(0.50, 0.46, 0.66), fondo * 0.8), n, l, v)
}

fn chispa(f: &Fragmento, g: &Global, edad: f32) -> Vec3 {
    let parpadeo = 0.7 + 0.3 * (g.tiempo * 37.0 + f.pos.x * 13.1 + f.pos.z * 7.7).sin();
    let caliente = rgb(1.0, 1.0, 0.82).mezclar(rgb(1.0, 0.62, 0.12), suave(0.0, 0.4, edad));
    let color = caliente.mezclar(rgb(0.85, 0.16, 0.04), suave(0.5, 1.0, edad));
    color * (parpadeo * (1.15 - 0.45 * edad))
}

fn roto(f: &Fragmento, g: &Global, radio: f32) -> Vec3 {
    let p = f.local;
    let d = p.normalizado();
    let n = f.normal.normalizado();
    let l = (g.sol - f.pos).normalizado();
    let v = (g.camara - f.pos).normalizado();
    let t = g.tiempo;
    let latido = 0.75 + 0.25 * (t * 1.6).sin();
    let energia = rgb(0.62, 0.30, 1.0);
    if f.color.x > 1.5 {
        let remolino = g.ruido.fbm(d * 2.5 + Vec3::new(t * 0.2, -t * 0.15, t * 0.1), 3);
        let vetas = 1.0 - suave(0.0, 0.25, remolino.abs());
        let borde = (1.0 - n.dot(v).max(0.0)).powi(2);
        return energia.mezclar(rgb(0.85, 1.0, 1.0), vetas * 0.8 + borde * 0.3) * (1.1 + 0.4 * latido);
    }
    if f.color.x > 0.5 {
        let profundo = 1.0 - suave(radio * 0.5, radio * 1.05, p.largo());
        let capa = (p.largo() * 2.2 + g.ruido.fbm(d * 3.0, 2) * 2.5).sin();
        let tierra = rgb(0.46, 0.33, 0.27)
            .mezclar(rgb(0.66, 0.52, 0.38), suave(-0.3, 0.3, capa))
            .mezclar(rgb(0.30, 0.24, 0.34), suave(0.6, 0.95, capa));
        let (f1, f2, _) = g.ruido.voronoi(p * 0.55);
        let veta = (1.0 - suave(0.0, 0.08, f2 - f1)) * profundo;
        let hacia_nucleo = n.dot(-d).max(0.0);
        let luz = iluminar(tierra, n, l, v) * 0.75 + tierra.por(energia) * (hacia_nucleo * profundo * 1.4 * latido);
        return luz.mezclar(energia * (1.0 + 0.6 * latido), veta);
    }
    let manchas = g.ruido.fbm(d * 5.0, 3);
    let pasto = rgb(0.30, 0.68, 0.48).mezclar(rgb(0.55, 0.82, 0.50), suave(-0.2, 0.3, manchas));
    let orilla = 1.0 - suave(0.02, 0.09, f.extra);
    let albedo = pasto.mezclar(rgb(0.58, 0.55, 0.66), orilla.max(suave(0.35, 0.5, manchas)));
    iluminar(albedo, n, l, v)
}

fn navidad(f: &Fragmento, g: &Global) -> Vec3 {
    let d = f.local.normalizado();
    let n = f.normal.normalizado();
    let l = (g.sol - f.pos).normalizado();
    let v = (g.camara - f.pos).normalizado();
    let t = g.tiempo;
    let mitad = (l + v).normalizado();

    let agujas = g.ruido.fbm(d * 38.0, 2);
    let bosque = rgb(0.06, 0.28, 0.17).mezclar(rgb(0.14, 0.46, 0.26), suave(-0.3, 0.4, agujas));
    let altura = d.y * forma_navidad(d);
    let manto = g.ruido.fbm(d * 4.5 + Vec3::new(1.3, 8.1, 4.4), 3) - altura * 0.18 + 0.05;
    let cubierta = suave(-0.04, 0.08, manto);
    let (_, _, id) = g.ruido.voronoi(d * 70.0);
    let titileo = 0.5 + 0.5 * (t * 3.0 + id * 60.0).sin();
    let destello = if id > 0.9 { n.dot(mitad).max(0.0).powi(12) * titileo * 1.6 } else { 0.0 };
    let suelo = iluminar(bosque.mezclar(rgb(0.92, 0.95, 1.0), cubierta), n, l, v) + rgb(1.0, 1.0, 1.0) * (destello * cubierta);

    let (cinta, avance) = guirnalda(d);
    let franja = suave(-0.2, 0.2, (avance * 90.0).sin());
    let oropel = rgb(0.82, 0.10, 0.14).mezclar(rgb(1.0, 0.78, 0.26), franja);
    let brillo = n.dot(mitad).max(0.0).powi(24) * 0.8;
    let adorno = iluminar(oropel, n, l, v) + rgb(1.0, 0.95, 0.8) * brillo;
    let color = suelo.mezclar(adorno, suave(0.35, 0.6, cinta));

    let k = avance * FOCOS_POR_VUELTA;
    let foco = (1.0 - suave(0.14, 0.26, (k.fract() - 0.5).abs())) * suave(0.55, 0.85, cinta);
    let paleta = [rgb(1.0, 0.2, 0.2), rgb(0.25, 1.0, 0.35), rgb(1.0, 0.8, 0.25), rgb(0.35, 0.6, 1.0)];
    let turno = (k.floor() as i32 + (t * 3.0) as i32).rem_euclid(4) as usize;
    let encendido = 0.6 + 0.9 * suave(0.0, 0.3, (t * 3.0).fract());
    color.mezclar(paleta[turno] * encendido * 1.3, foco)
}

fn copo(f: &Fragmento, g: &Global) -> Vec3 {
    rgb(0.92, 0.96, 1.0) * (0.85 + 0.15 * (g.tiempo * 4.0 + f.pos.x * 3.0).sin())
}

fn llama(f: &Fragmento, g: &Global, empuje: f32) -> Vec3 {
    let t = g.tiempo;
    let lengua = g.ruido.fbm(f.local * 2.5 + Vec3::new(0.0, -t * 14.0, t * 3.0), 3);
    let a = (f.local.y.clamp(0.0, 1.0) + lengua * 0.25).clamp(0.0, 1.0);
    let frente = f.normal.normalizado().dot((g.camara - f.pos).normalizado()).abs();
    let nucleo = suave(0.35, 0.95, frente) * (1.0 - suave(0.1, 0.7, a));
    let turbo = suave(0.75, 1.0, empuje);
    let medio = rgb(1.0, 0.68, 0.18).mezclar(rgb(0.4, 0.75, 1.0), turbo);
    let cola = rgb(0.75, 0.12, 0.04).mezclar(rgb(0.2, 0.25, 0.9), turbo);
    let color = medio.mezclar(cola, suave(0.2, 1.0, a)).mezclar(rgb(1.0, 0.97, 0.88), nucleo);
    let parpadeo = 0.88 + 0.12 * (t * 50.0).sin();
    color * ((1.45 - 0.75 * a) * (0.78 + 0.22 * frente) * parpadeo)
}
