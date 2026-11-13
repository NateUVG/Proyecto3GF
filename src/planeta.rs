use crate::config::{
    ALTURA_PICO, ANILLOS, PISOS_ARBOL, FRAGMENTOS_ROTO, VUELTAS_GUIRNALDA, GROSOR_ROTO, NIVEL_MAXIMO, NUCLEO_ROTO, PLANETAS, REDONDEZ_CUBO, SEPARACION_ROTO,
    VELOCIDAD_TIEMPO,
};
use crate::geometria::{Icosfera, Malla, Vertice, malla_de_esfera};
use crate::ruido::{Azar, Perlin};
use crate::vector::{Mat4, Vec3, suave};
use std::collections::HashMap;
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, PartialEq)]
pub enum Terreno {
    Tecno,
    Lava,
    Hielo,
    Dulce,
    Roto,
    Navidad,
}

pub struct DefPlaneta {
    pub nombre: &'static str,
    pub radio: f32,
    pub radio_orbita: f32,
    pub periodo_orbita: f32,
    pub periodo_rotacion: f32,
    pub inclinacion_orbita: f32,
    pub inclinacion_eje: f32,
    pub fase: f32,
    pub amplitud: f32,
    pub frecuencia: f32,
    pub terreno: Terreno,
    pub cantidad: usize,
    pub tamano_objetos: f32,
    pub anillos: bool,
}

pub struct DefLuna {
    pub padre: usize,
    pub radio: f32,
    pub distancia: f32,
    pub periodo: f32,
    pub inclinacion: f32,
    pub profundidad: f32,
}

impl DefLuna {
    fn angulo(&self, t: f32) -> f32 {
        TAU * t * VELOCIDAD_TIEMPO / self.periodo
    }

    pub fn radio_orbita(&self) -> f32 {
        self.distancia * PLANETAS[self.padre].radio_limite()
    }

    pub fn posicion(&self, t: f32) -> Vec3 {
        let a = self.angulo(t);
        let orbita = Mat4::rotacion_x(self.inclinacion).punto(Vec3::new(a.cos(), 0.0, a.sin()) * self.radio_orbita());
        PLANETAS[self.padre].posicion(t) + orbita
    }

    pub fn modelo(&self, t: f32) -> Mat4 {
        let a = self.angulo(t);
        Mat4::traslacion(PLANETAS[self.padre].posicion(t))
            .por(&Mat4::rotacion_x(self.inclinacion))
            .por(&Mat4::rotacion_y(-a))
            .por(&Mat4::traslacion(Vec3::new(self.radio_orbita(), 0.0, 0.0)))
    }

    pub fn perfil(&self, ruido: &Perlin, d: Vec3) -> (f32, f32) {
        let (f1, _, _) = ruido.voronoi(d * 2.4 + Vec3::new(6.0, 3.0, 1.0));
        let k = f1 / 0.45;
        let cuenco = if k < 1.0 { -(1.0 - k * k) } else { 0.0 };
        let borde = 0.35 * (-((k - 1.0) * 5.0).powi(2)).exp();
        (self.profundidad * (cuenco + borde), k)
    }

    pub fn radio_en(&self, ruido: &Perlin, direccion_local: Vec3) -> f32 {
        self.radio + self.perfil(ruido, direccion_local).0
    }
}

pub struct Decoracion {
    pub local: Mat4,
    pub pos: Vec3,
    pub arriba: Vec3,
    pub escala: f32,
    pub variante: usize,
}

pub fn coordenadas_cubo(d: Vec3) -> (f32, f32, f32) {
    let (ax, ay, az) = (d.x.abs(), d.y.abs(), d.z.abs());
    if ax >= ay && ax >= az {
        (d.y / ax, d.z / ax, if d.x > 0.0 { 0.0 } else { 1.0 })
    } else if ay >= az {
        (d.x / ay, d.z / ay, if d.y > 0.0 { 2.0 } else { 3.0 })
    } else {
        (d.x / az, d.y / az, if d.z > 0.0 { 4.0 } else { 5.0 })
    }
}

pub fn placa_cubo(ruido: &Perlin, d: Vec3, celdas: f32) -> (f32, f32, f32, f32) {
    let (u, v, cara) = coordenadas_cubo(d);
    let (gu, gv) = ((u + 1.0) * 0.5 * celdas, (v + 1.0) * 0.5 * celdas);
    let id = ruido.ruido(Vec3::new(gu.floor() * 1.37 + 0.31, gv.floor() * 2.11 + 0.47, cara * 3.73 + 0.5));
    (gu, gv, cara, id)
}

pub fn forma_navidad(d: Vec3) -> f32 {
    let abertura = (1.0 / (1.0 + ALTURA_PICO)).asin();
    let angulo = d.y.clamp(-1.0, 1.0).acos() + abertura;
    if angulo < PI / 2.0 { 1.0 / angulo.sin() } else { 1.0 }
}

pub fn guirnalda(d: Vec3) -> (f32, f32) {
    let forma = forma_navidad(d);
    let y = d.y * forma;
    let angulo = d.z.atan2(d.x) / TAU;
    let vuelta = angulo + y * VUELTAS_GUIRNALDA;
    let cercana = vuelta.round();
    let ancho = (1.0 - d.y * d.y).max(0.0).sqrt() * forma;
    let distancia = (vuelta - cercana).abs() * TAU * ancho;
    let extremos = suave(-0.85, -0.65, y) * (1.0 - suave(1.75, 1.95, y));
    ((-(distancia / 0.2).powi(2)).exp() * extremos, angulo - cercana)
}

pub fn semilla_roto(k: usize) -> Vec3 {
    let y = 1.0 - (k as f32 + 0.5) * 2.0 / FRAGMENTOS_ROTO as f32;
    let r = (1.0 - y * y).sqrt();
    let a = k as f32 * 2.399_963 + 0.7;
    Vec3::new(r * a.cos(), y, r * a.sin())
}

pub fn fragmento_roto(ruido: &Perlin, u: Vec3) -> (usize, f32) {
    let mut mejor = (0, f32::MIN);
    let mut segundo = f32::MIN;
    for k in 0..FRAGMENTOS_ROTO {
        let s = semilla_roto(k);
        let valor = u.dot(s) + 0.07 * ruido.fbm(u * 3.2 + s * 5.0, 2);
        if valor > mejor.1 {
            segundo = mejor.1;
            mejor = (k, valor);
        } else if valor > segundo {
            segundo = valor;
        }
    }
    (mejor.0, mejor.1 - segundo)
}

impl DefPlaneta {
    fn centro_fragmento(&self, k: usize) -> Vec3 {
        semilla_roto(k) * (self.radio * SEPARACION_ROTO)
    }

    fn impacto_roto(&self, d: Vec3) -> (f32, Vec3) {
        let mut mejor = (0.0, Vec3::CERO);
        for k in 0..FRAGMENTOS_ROTO {
            let c = self.centro_fragmento(k);
            let b = d.dot(c);
            let t = b + (b * b - c.dot(c) + self.radio * self.radio).max(0.0).sqrt();
            if t > mejor.0 {
                mejor = (t, c);
            }
        }
        mejor
    }

    fn relieve_roto(&self, ruido: &Perlin, d: Vec3) -> f32 {
        self.amplitud * ruido.fbm(d * self.frecuencia + Vec3::new(3.3, 7.1, 2.9), 3)
    }

    pub fn malla_rota(&self, ruido: &Perlin, esfera: &Icosfera, nucleo: &Icosfera) -> Malla {
        let radio = self.radio;
        let adentro = radio * (1.0 - GROSOR_ROTO);
        let dir = &esfera.direcciones;
        let celdas: Vec<f32> = dir.iter().map(|&u| fragmento_roto(ruido, u).1).collect();
        let duenos: Vec<usize> = esfera
            .indices
            .iter()
            .map(|&[a, b, c]| fragmento_roto(ruido, (dir[a as usize] + dir[b as usize] + dir[c as usize]).normalizado()).0)
            .collect();
        let vertice = |pos: Vec3, region: f32, extra: f32| Vertice {
            pos,
            normal: Vec3::CERO,
            color: Vec3::new(region, 0.0, 0.0),
            extra,
        };
        let mut vertices: Vec<Vertice> = Vec::new();
        let mut indices = Vec::new();
        for k in 0..FRAGMENTOS_ROTO {
            let c = self.centro_fragmento(k);
            let caras: Vec<[u32; 3]> = esfera
                .indices
                .iter()
                .zip(&duenos)
                .filter(|&(_, &d)| d == k)
                .map(|(&t, _)| t)
                .collect();
            let mut bordes: HashMap<(u32, u32), (u32, u32)> = HashMap::new();
            for &[a, b, cc] in &caras {
                for (x, y) in [(a, b), (b, cc), (cc, a)] {
                    let clave = (x.min(y), x.max(y));
                    if bordes.remove(&clave).is_none() {
                        bordes.insert(clave, (x, y));
                    }
                }
            }
            let mut vecinos: HashMap<u32, Vec<u32>> = HashMap::new();
            for &(x, y) in bordes.keys() {
                vecinos.entry(x).or_default().push(y);
                vecinos.entry(y).or_default().push(x);
            }
            let mut contorno: HashMap<u32, Vec3> = vecinos.keys().map(|&s| (s, dir[s as usize])).collect();
            for _ in 0..6 {
                let previo = contorno.clone();
                for (s, lista) in &vecinos {
                    let suma = lista.iter().fold(previo[s] * 2.0, |acc, v| acc + previo[v]);
                    contorno.insert(*s, suma.normalizado());
                }
            }
            let mut propios: HashMap<u32, (u32, u32)> = HashMap::new();
            for &[a, b, cc] in &caras {
                let mut ids = [(0, 0); 3];
                for (j, s) in [a, b, cc].into_iter().enumerate() {
                    ids[j] = *propios.entry(s).or_insert_with(|| {
                        let u = contorno.get(&s).copied().unwrap_or(dir[s as usize]);
                        let d = (c + u * radio).normalizado();
                        let afuera = c + u * (radio + self.relieve_roto(ruido, d));
                        let hondo = c + u * (adentro + 0.6 * ruido.fbm(u * 4.0, 2));
                        vertices.push(vertice(afuera, 0.0, celdas[s as usize]));
                        vertices.push(vertice(hondo, 1.0, 0.0));
                        (vertices.len() as u32 - 2, vertices.len() as u32 - 1)
                    });
                }
                indices.push([ids[0].0, ids[1].0, ids[2].0]);
                indices.push([ids[0].1, ids[2].1, ids[1].1]);
            }
            for (x, y) in bordes.into_values() {
                let (ox, ix) = propios[&x];
                let (oy, iy) = propios[&y];
                let p = [ox, oy, iy, ix].map(|i| vertices[i as usize].pos);
                for orden in [[0, 1, 2, 3], [0, 3, 2, 1]] {
                    let base = vertices.len() as u32;
                    for q in orden {
                        vertices.push(vertice(p[q], 1.0, 0.0));
                    }
                    indices.push([base, base + 1, base + 2]);
                    indices.push([base, base + 2, base + 3]);
                }
            }
        }
        let base = vertices.len() as u32;
        vertices.extend(nucleo.direcciones.iter().map(|&u| vertice(u * (radio * NUCLEO_ROTO), 2.0, 0.0)));
        indices.extend(nucleo.indices.iter().map(|&[a, b, c]| [a + base, b + base, c + base]));
        for &[a, b, c] in &indices {
            let (pa, pb, pc) = (vertices[a as usize].pos, vertices[b as usize].pos, vertices[c as usize].pos);
            let n = (pb - pa).cross(pc - pa);
            for i in [a, b, c] {
                vertices[i as usize].normal = vertices[i as usize].normal + n;
            }
        }
        for v in &mut vertices {
            v.normal = v.normal.normalizado();
        }
        Malla { vertices, indices }
    }

    pub fn perfil(&self, ruido: &Perlin, d: Vec3) -> (f32, f32) {
        match self.terreno {
            Terreno::Tecno => {
                let p = REDONDEZ_CUBO;
                let s = (d.x.abs().powf(p) + d.y.abs().powf(p) + d.z.abs().powf(p)).powf(1.0 / p);
                let base = self.radio / s - self.radio;
                let (gu, gv, _, id) = placa_cubo(ruido, d, self.frecuencia);
                let (fu, fv) = (gu.fract(), gv.fract());
                let borde = suave(0.0, 0.1, fu.min(1.0 - fu)) * suave(0.0, 0.1, fv.min(1.0 - fv));
                let placa = if id > 0.04 { borde } else { 0.0 };
                (base + self.amplitud * placa, placa)
            }
            Terreno::Lava => {
                let n = ruido.fbm(d * self.frecuencia + Vec3::new(8.2, 1.4, 5.9), 4);
                let cresta = (1.0 - n.abs() * 3.2).max(0.0);
                let h = cresta * cresta;
                (self.amplitud * h, h)
            }
            Terreno::Hielo => {
                let (f1, _, _) = ruido.voronoi(d * self.frecuencia + Vec3::new(2.0, 9.0, 4.0));
                let h = 1.0 - suave(0.0, 0.85, f1);
                (self.amplitud * h, h)
            }
            Terreno::Dulce => {
                let a = d.z.atan2(d.x) + d.y * 3.0;
                let h = (a * self.frecuencia).sin() * (1.0 - d.y * d.y);
                (self.amplitud * (h * 0.5 + 0.5), h)
            }
            Terreno::Navidad => {
                let forma = forma_navidad(d);
                let altura = d.y * forma;
                let s = (altura * PISOS_ARBOL).fract();
                let peso = suave(0.02, 0.15, forma - 1.0) * (1.0 - suave(1.7, 1.95, altura));
                let piso = (1.0 - s) * suave(0.0, 0.08, s) * peso;
                let cinta = guirnalda(d).0;
                (self.radio * (forma - 1.0 + 0.12 * piso) + self.amplitud * cinta, cinta)
            }
            Terreno::Roto => {
                let (t, _) = self.impacto_roto(d);
                (t - self.radio + self.relieve_roto(ruido, d), fragmento_roto(ruido, d).1)
            }
        }
    }

    pub fn radio_limite(&self) -> f32 {
        match self.terreno {
            Terreno::Tecno => self.radio * 3f32.powf(0.5 - 1.0 / REDONDEZ_CUBO) + self.amplitud,
            Terreno::Roto => self.radio * (1.0 + SEPARACION_ROTO) + self.amplitud,
            Terreno::Navidad => self.radio * (1.0 + ALTURA_PICO) + self.amplitud,
            _ => self.radio + self.amplitud * 1.05,
        }
    }

    pub fn normal_superficie(&self, d: Vec3) -> Vec3 {
        match self.terreno {
            Terreno::Tecno => {
                let k = REDONDEZ_CUBO - 1.0;
                let g = |c: f32| c.signum() * c.abs().powf(k);
                Vec3::new(g(d.x), g(d.y), g(d.z)).normalizado()
            }
            Terreno::Roto => {
                let (t, c) = self.impacto_roto(d);
                (d * t - c).normalizado()
            }
            Terreno::Navidad => {
                let punto = |v: Vec3| {
                    let v = v.normalizado();
                    v * forma_navidad(v)
                };
                let auxiliar = if d.y.abs() < 0.9 { Vec3::new(0.0, 1.0, 0.0) } else { Vec3::new(1.0, 0.0, 0.0) };
                let e1 = auxiliar.cross(d).normalizado() * 1e-3;
                let e2 = d.cross(e1);
                let n = (punto(d + e1) - punto(d - e1)).cross(punto(d + e2) - punto(d - e2)).normalizado();
                if n.dot(d) < 0.0 { -n } else { n }
            }
            _ => d,
        }
    }

    fn admite_objeto(&self, ruido: &Perlin, d: Vec3) -> bool {
        let extra = self.perfil(ruido, d).1;
        let densidad = ruido.fbm(d * 2.6 + Vec3::new(31.0, 17.0, 5.0), 3);
        match self.terreno {
            Terreno::Tecno => {
                (extra < 0.001 || extra > 0.999) && self.normal_superficie(d).dot(d) > 0.97 && densidad > -0.12
            }
            Terreno::Lava => extra > 0.35,
            Terreno::Hielo => extra > 0.25 && densidad > -0.1,
            Terreno::Dulce => d.y < 0.5 && extra > 0.1,
            Terreno::Roto => extra > 0.12 && densidad > -0.2,
            Terreno::Navidad => extra < 0.05 && d.y * forma_navidad(d) < 1.6 && densidad > -0.15,
        }
    }

    pub fn distribuir(&self, ruido: &Perlin, semilla: u64) -> Vec<Decoracion> {
        let mut azar = Azar::new(semilla ^ (self.radio_orbita as u64).wrapping_mul(0x100_0193));
        let separacion = 0.09f32.cos();
        let mut direcciones: Vec<Vec3> = Vec::with_capacity(self.cantidad);
        let mut lista = Vec::with_capacity(self.cantidad);
        for _ in 0..self.cantidad * 40 {
            if lista.len() == self.cantidad {
                break;
            }
            let z = azar.unidad() * 2.0 - 1.0;
            let a = azar.unidad() * TAU;
            let r = (1.0 - z * z).sqrt();
            let d = Vec3::new(r * a.cos(), z, r * a.sin());
            let (giro, tamano) = (azar.unidad() * TAU, azar.unidad());
            if !self.admite_objeto(ruido, d) || direcciones.iter().any(|o| o.dot(d) > separacion) {
                continue;
            }
            direcciones.push(d);
            let variante = (azar.unidad() < 0.35) as usize;
            let escala = self.tamano_objetos * (0.7 + 0.7 * tamano);
            let arriba = self.normal_superficie(d);
            let auxiliar = if arriba.y.abs() < 0.9 { Vec3::new(0.0, 1.0, 0.0) } else { Vec3::new(1.0, 0.0, 0.0) };
            let tangente = auxiliar.cross(arriba).normalizado().girar(arriba, giro);
            let otra = tangente.cross(arriba);
            let pos = d * self.radio_en(ruido, d) - arriba * (0.03 * escala);
            lista.push(Decoracion {
                local: Mat4::desde_ejes(tangente * escala, arriba * escala, otra * escala, pos),
                pos,
                arriba,
                escala,
                variante,
            });
        }
        lista
    }

    pub fn posicion(&self, t: f32) -> Vec3 {
        let a = self.fase + TAU * t * VELOCIDAD_TIEMPO / self.periodo_orbita;
        Mat4::rotacion_x(self.inclinacion_orbita).punto(Vec3::new(a.cos(), 0.0, a.sin()) * self.radio_orbita)
    }

    pub fn rotacion(&self, t: f32) -> Mat4 {
        let a = TAU * t * VELOCIDAD_TIEMPO / self.periodo_rotacion;
        Mat4::rotacion_z(self.inclinacion_eje).por(&Mat4::rotacion_y(a))
    }

    pub fn radio_en(&self, ruido: &Perlin, direccion_local: Vec3) -> f32 {
        self.radio + self.perfil(ruido, direccion_local).0
    }

    pub fn marco_anillo(&self, t: f32) -> Mat4 {
        Mat4::traslacion(self.posicion(t)).por(&Mat4::rotacion_z(self.inclinacion_eje))
    }

    pub fn normal_anillo(&self) -> Vec3 {
        Mat4::rotacion_z(self.inclinacion_eje).direccion(Vec3::new(0.0, 1.0, 0.0))
    }

    pub fn coordenadas_anillo(&self, p: Vec3, t: f32) -> (f32, f32, Vec3) {
        let n = self.normal_anillo();
        let relativa = p - self.posicion(t);
        let h = relativa.dot(n);
        let plano = relativa - n * h;
        (h, plano.largo(), plano.normalizado())
    }

    pub fn sobre_anillo(&self, r: f32, margen: f32) -> bool {
        let adentro = ANILLOS[0].0 * self.radio;
        let afuera = ANILLOS[ANILLOS.len() - 1].1 * self.radio;
        self.anillos && r > adentro - margen && r < afuera + margen
    }
}

pub struct Planeta {
    pub def: &'static DefPlaneta,
    pub niveles: Vec<Malla>,
    pub decoraciones: Vec<Decoracion>,
}

impl Planeta {
    pub fn new(def: &'static DefPlaneta, esferas: &[Icosfera], ruido: &Perlin, semilla: u64) -> Planeta {
        let niveles = match def.terreno {
            Terreno::Roto => esferas[..=6]
                .iter()
                .enumerate()
                .map(|(i, e)| def.malla_rota(ruido, e, &esferas[i.min(4)]))
                .collect(),
            _ => esferas[..=NIVEL_MAXIMO]
                .iter()
                .map(|e| {
                    malla_de_esfera(e, |d| {
                        let (altura, extra) = def.perfil(ruido, d);
                        (def.radio + altura, extra)
                    })
                })
                .collect(),
        };
        Planeta {
            def,
            niveles,
            decoraciones: def.distribuir(ruido, semilla),
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::config::LUNAS;

    #[test]
    fn la_luna_acompana_a_su_planeta() {
        for luna in LUNAS {
            let mut anterior = luna.posicion(0.0);
            for t in [3.0, 20.0, 51.0, 300.0] {
                let centro = PLANETAS[luna.padre].posicion(t);
                let p = luna.posicion(t);
                assert!(((p - centro).largo() - luna.radio_orbita()).abs() < 1e-3);
                assert!((luna.modelo(t).punto(Vec3::CERO) - p).largo() < 1e-2);
                assert!((p - anterior).largo() > 0.1);
                anterior = p;
            }
        }
    }

    #[test]
    fn la_misma_semilla_da_las_mismas_ubicaciones() {
        let ruido = Perlin::new(2026);
        for def in PLANETAS {
            let (a, b, c) = (def.distribuir(&ruido, 7), def.distribuir(&ruido, 7), def.distribuir(&ruido, 8));
            assert!(a.len() >= def.cantidad / 2, "{} solo tiene {}", def.nombre, a.len());
            assert!(a.iter().zip(&b).all(|(x, y)| x.pos == y.pos));
            assert!(a.iter().zip(&c).any(|(x, y)| x.pos != y.pos));
        }
    }

    #[test]
    fn los_objetos_quedan_perpendiculares_mientras_el_planeta_rota() {
        let ruido = Perlin::new(2026);
        for def in PLANETAS {
            for d in def.distribuir(&ruido, 7).iter().take(20) {
                for t in [0.0, 13.0, 77.0] {
                    let rotacion = def.rotacion(t);
                    let modelo = rotacion.por(&d.local);
                    let arriba = modelo.direccion(Vec3::new(0.0, 1.0, 0.0)).normalizado();
                    let normal = rotacion.direccion(def.normal_superficie(d.pos.normalizado()));
                    assert!(arriba.dot(normal) > 0.9999);
                    let superficie = def.radio_en(&ruido, d.pos.normalizado());
                    assert!((d.pos.largo() - superficie).abs() < 0.1);
                }
            }
        }
    }
}
