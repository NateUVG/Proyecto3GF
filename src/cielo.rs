use crate::config::SEMILLA;
use crate::ruido::{Azar, Perlin};
use crate::vector::{Vec3, suave};
use std::sync::Mutex;
use std::thread;

const LADO: usize = 512;

pub struct Estrella {
    pub dir: Vec3,
    pub color: Vec3,
    pub grande: bool,
}

pub struct Cielo {
    caras: Vec<[u8; 3]>,
    pub estrellas: Vec<Estrella>,
}

fn direccion_cara(cara: usize, u: f32, v: f32) -> Vec3 {
    match cara {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    }
    .normalizado()
}

fn nebulosa(ruido: &Perlin, d: Vec3) -> [u8; 3] {
    let desvio = ruido.fbm(d * 1.6 + Vec3::new(5.0, 1.0, 9.0), 3);
    let a = suave(-0.08, 0.5, ruido.fbm(d * 2.2 + Vec3::new(desvio, desvio, -desvio) * 1.4, 5));
    let b = suave(0.0, 0.45, ruido.fbm(d * 2.8 + Vec3::new(21.0, -desvio, 3.0), 5));
    let base = Vec3::new(0.018, 0.022, 0.085);
    let azul = Vec3::new(0.05, 0.16, 0.42) * a;
    let magenta = Vec3::new(0.30, 0.06, 0.34) * (b * b);
    let c = base + azul + magenta;
    let canal = |x: f32| (x.clamp(0.0, 1.0) * 255.0) as u8;
    [canal(c.x), canal(c.y), canal(c.z)]
}

impl Cielo {
    pub fn new() -> Cielo {
        let ruido = Perlin::new(SEMILLA + 77);
        let mut caras = vec![[0u8; 3]; 6 * LADO * LADO];
        let filas = Mutex::new(caras.chunks_mut(LADO).enumerate());
        let hilos = thread::available_parallelism().map_or(4, |n| n.get());
        thread::scope(|s| {
            for _ in 0..hilos {
                s.spawn(|| {
                    loop {
                        let siguiente = filas.lock().unwrap().next();
                        let Some((i, fila)) = siguiente else { break };
                        let (cara, y) = (i / LADO, i % LADO);
                        for (x, pixel) in fila.iter_mut().enumerate() {
                            let u = (x as f32 + 0.5) / LADO as f32 * 2.0 - 1.0;
                            let v = (y as f32 + 0.5) / LADO as f32 * 2.0 - 1.0;
                            *pixel = nebulosa(&ruido, direccion_cara(cara, u, v));
                        }
                    }
                });
            }
        });

        let mut azar = Azar::new(SEMILLA + 5);
        let tonos = [
            Vec3::new(1.0, 1.0, 1.0),
            Vec3::new(0.7, 0.85, 1.0),
            Vec3::new(1.0, 0.9, 0.6),
            Vec3::new(1.0, 0.65, 0.85),
        ];
        let estrellas = (0..5200)
            .map(|_| {
                let z = azar.unidad() * 2.0 - 1.0;
                let a = azar.unidad() * std::f32::consts::TAU;
                let r = (1.0 - z * z).sqrt();
                let tono = tonos[(azar.siguiente() % 4) as usize];
                let brillo = 0.35 + 0.65 * azar.unidad().powi(2);
                Estrella {
                    dir: Vec3::new(r * a.cos(), z, r * a.sin()),
                    color: tono * brillo,
                    grande: azar.unidad() < 0.06,
                }
            })
            .collect();
        Cielo { caras, estrellas }
    }

    fn texel(&self, cara: usize, x: usize, y: usize) -> Vec3 {
        let [r, g, b] = self.caras[cara * LADO * LADO + y * LADO + x];
        Vec3::new(r as f32, g as f32, b as f32)
    }

    pub fn muestrear(&self, d: Vec3) -> u32 {
        let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());
        let (cara, u, v) = if ax >= ay && ax >= az {
            if d.x > 0.0 {
                (0, -d.z / ax, -d.y / ax)
            } else {
                (1, d.z / ax, -d.y / ax)
            }
        } else if ay >= az {
            if d.y > 0.0 {
                (2, d.x / ay, d.z / ay)
            } else {
                (3, d.x / ay, -d.z / ay)
            }
        } else if d.z > 0.0 {
            (4, d.x / az, -d.y / az)
        } else {
            (5, -d.x / az, -d.y / az)
        };
        let maximo = (LADO - 1) as f32;
        let fx = ((u + 1.0) * 0.5 * LADO as f32 - 0.5).clamp(0.0, maximo);
        let fy = ((v + 1.0) * 0.5 * LADO as f32 - 0.5).clamp(0.0, maximo);
        let (x0, y0) = (fx as usize, fy as usize);
        let (x1, y1) = ((x0 + 1).min(LADO - 1), (y0 + 1).min(LADO - 1));
        let (tx, ty) = (fx - x0 as f32, fy - y0 as f32);
        let arriba = self.texel(cara, x0, y0).mezclar(self.texel(cara, x1, y0), tx);
        let abajo = self.texel(cara, x0, y1).mezclar(self.texel(cara, x1, y1), tx);
        let c = arriba.mezclar(abajo, ty);
        ((c.x as u32) << 16) | ((c.y as u32) << 8) | c.z as u32
    }
}
