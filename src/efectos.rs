use crate::config::*;
use crate::ruido::Azar;
use crate::vector::Vec3;
use std::f32::consts::TAU;

pub struct Chispa {
    pub pos: Vec3,
    pub direccion: Vec3,
    pub largo: f32,
    pub edad: f32,
}

pub struct Particula {
    pub pos: Vec3,
    pub vel: Vec3,
    pub nacimiento: f32,
}

pub struct Estela {
    pub particulas: Vec<Particula>,
    acumulado: f32,
    azar: Azar,
}

impl Estela {
    pub fn new() -> Estela {
        Estela {
            particulas: Vec::new(),
            acumulado: 0.0,
            azar: Azar::new(SEMILLA ^ 0xE57E),
        }
    }

    pub fn avanzar(&mut self, toberas: &[Vec3], atras: Vec3, empuje: f32, t: f32, dt: f32) {
        self.particulas.retain(|p| t - p.nacimiento < VIDA_ESTELA);
        for p in &mut self.particulas {
            p.pos = p.pos + p.vel * dt;
        }
        if empuje < 0.05 {
            self.acumulado = 0.0;
            return;
        }
        self.acumulado += dt * RITMO_ESTELA * empuje;
        while self.acumulado >= 1.0 {
            self.acumulado -= 1.0;
            for &b in toberas {
                let a = &mut self.azar;
                let desvio = Vec3::new(a.unidad() - 0.5, a.unidad() - 0.5, a.unidad() - 0.5) * 1.2;
                self.particulas.push(Particula {
                    pos: b,
                    vel: atras * (VELOCIDAD_ESTELA * (0.6 + empuje)) + desvio,
                    nacimiento: t,
                });
            }
        }
    }
}

pub fn polvo(camara: Vec3) -> Vec<Vec3> {
    let base = |c: f32| (c / CELDA_POLVO).floor() as i64;
    let (bx, by, bz) = (base(camara.x), base(camara.y), base(camara.z));
    let mut lista = Vec::new();
    for i in -2..=2 {
        for j in -2..=2 {
            for k in -2..=2 {
                let (x, y, z) = (bx + i, by + j, bz + k);
                let mut azar = Azar::new((x.wrapping_mul(73_856_093) ^ y.wrapping_mul(19_349_663) ^ z.wrapping_mul(83_492_791)) as u64);
                let desvio = Vec3::new(azar.unidad(), azar.unidad(), azar.unidad());
                lista.push((Vec3::new(x as f32, y as f32, z as f32) + desvio) * CELDA_POLVO);
            }
        }
    }
    lista
}

fn ejes(arriba: Vec3) -> (Vec3, Vec3) {
    let auxiliar = if arriba.y.abs() < 0.9 { Vec3::new(0.0, 1.0, 0.0) } else { Vec3::new(1.0, 0.0, 0.0) };
    let a = auxiliar.cross(arriba).normalizado();
    (a, arriba.cross(a))
}

pub fn chispas(volcanes: &[(Vec3, Vec3)], t: f32) -> Vec<Chispa> {
    let mut lista = Vec::new();
    for (v, &(base, arriba)) in volcanes.iter().enumerate() {
        let ciclo = (t + v as f32 * DESFASE_VOLCANES).rem_euclid(CICLO_ERUPCION);
        let (e1, e2) = ejes(arriba);
        for k in 0..CHISPAS_POR_VOLCAN {
            let tau = (t + k as f32 * VIDA_CHISPA / CHISPAS_POR_VOLCAN as f32).rem_euclid(VIDA_CHISPA);
            let salida = ciclo - tau;
            if !(0.0..=ERUPCION_ACTIVA).contains(&salida) {
                continue;
            }
            let lanzamiento = ((t - tau) / VIDA_CHISPA).round() as i64 as u64;
            let mut azar = Azar::new((v as u64) << 40 ^ (k as u64) << 20 ^ lanzamiento);
            let (h1, h2, h3) = (azar.unidad(), azar.unidad(), azar.unidad());
            let angulo = h1 * TAU;
            let lateral = (e1 * angulo.cos() + e2 * angulo.sin()) * (VELOCIDAD_LATERAL * (0.2 + 0.8 * h2));
            let inicial = arriba * (VELOCIDAD_ERUPCION * (0.6 + 0.6 * h3)) + lateral;
            let pos = base + inicial * tau - arriba * (0.5 * GRAVEDAD_ERUPCION * tau * tau);
            if (pos - base).dot(arriba) < -0.1 {
                continue;
            }
            let velocidad = inicial - arriba * (GRAVEDAD_ERUPCION * tau);
            lista.push(Chispa {
                pos,
                direccion: velocidad.normalizado(),
                largo: LARGO_CHISPA * (1.0 + 0.35 * velocidad.largo()),
                edad: tau / VIDA_CHISPA,
            });
        }
    }
    lista
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn las_erupciones_dependen_solo_del_tiempo() {
        let volcanes = [(Vec3::new(0.0, 8.0, 0.0), Vec3::new(0.0, 1.0, 0.0))];
        let activas = chispas(&volcanes, 1.2);
        assert!(!activas.is_empty());
        let otra_vez = chispas(&volcanes, 1.2);
        assert!(activas.iter().zip(&otra_vez).all(|(a, b)| a.pos == b.pos));
        assert!(chispas(&volcanes, ERUPCION_ACTIVA + VIDA_CHISPA + 0.1).is_empty());
        assert!(activas.iter().all(|c| (c.pos - volcanes[0].0).dot(volcanes[0].1) > -0.2));
    }
}
