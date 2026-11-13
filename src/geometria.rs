use crate::vector::Vec3;
use std::collections::HashMap;

#[derive(Clone, Copy)]
pub struct Vertice {
    pub pos: Vec3,
    pub normal: Vec3,
    pub color: Vec3,
    pub extra: f32,
}

pub struct Malla {
    pub vertices: Vec<Vertice>,
    pub indices: Vec<[u32; 3]>,
}

impl Malla {
    pub fn radio(&self) -> f32 {
        self.vertices.iter().map(|v| v.pos.largo()).fold(0.0, f32::max)
    }
}

pub struct Icosfera {
    pub direcciones: Vec<Vec3>,
    pub indices: Vec<[u32; 3]>,
}

impl Icosfera {
    pub fn base() -> Icosfera {
        let t = (1.0 + 5f32.sqrt()) / 2.0;
        let puntos = [
            (-1.0, t, 0.0),
            (1.0, t, 0.0),
            (-1.0, -t, 0.0),
            (1.0, -t, 0.0),
            (0.0, -1.0, t),
            (0.0, 1.0, t),
            (0.0, -1.0, -t),
            (0.0, 1.0, -t),
            (t, 0.0, -1.0),
            (t, 0.0, 1.0),
            (-t, 0.0, -1.0),
            (-t, 0.0, 1.0),
        ];
        Icosfera {
            direcciones: puntos
                .iter()
                .map(|&(x, y, z)| Vec3::new(x, y, z).normalizado())
                .collect(),
            indices: vec![
                [0, 11, 5],
                [0, 5, 1],
                [0, 1, 7],
                [0, 7, 10],
                [0, 10, 11],
                [1, 5, 9],
                [5, 11, 4],
                [11, 10, 2],
                [10, 7, 6],
                [7, 1, 8],
                [3, 9, 4],
                [3, 4, 2],
                [3, 2, 6],
                [3, 6, 8],
                [3, 8, 9],
                [4, 9, 5],
                [2, 4, 11],
                [6, 2, 10],
                [8, 6, 7],
                [9, 8, 1],
            ],
        }
    }

    pub fn subdividir(&self) -> Icosfera {
        let mut direcciones = self.direcciones.clone();
        let mut indices = Vec::with_capacity(self.indices.len() * 4);
        let mut medios: HashMap<(u32, u32), u32> = HashMap::new();
        let mut medio = |a: u32, b: u32, direcciones: &mut Vec<Vec3>| -> u32 {
            let clave = (a.min(b), a.max(b));
            *medios.entry(clave).or_insert_with(|| {
                let m = (direcciones[a as usize] + direcciones[b as usize]).normalizado();
                direcciones.push(m);
                direcciones.len() as u32 - 1
            })
        };
        for &[a, b, c] in &self.indices {
            let ab = medio(a, b, &mut direcciones);
            let bc = medio(b, c, &mut direcciones);
            let ca = medio(c, a, &mut direcciones);
            indices.push([a, ab, ca]);
            indices.push([b, bc, ab]);
            indices.push([c, ca, bc]);
            indices.push([ab, bc, ca]);
        }
        Icosfera { direcciones, indices }
    }
}

pub fn anillo(bandas: &[(f32, f32)], radio: f32, segmentos: u32) -> Malla {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for &(adentro, afuera) in bandas {
        for (lado, normal) in [(0, Vec3::new(0.0, 1.0, 0.0)), (1, Vec3::new(0.0, -1.0, 0.0))] {
            let base = vertices.len() as u32;
            for i in 0..segmentos {
                let a = i as f32 / segmentos as f32 * std::f32::consts::TAU;
                let d = Vec3::new(a.cos(), 0.0, a.sin());
                for r in [adentro, afuera] {
                    vertices.push(Vertice {
                        pos: d * (r * radio),
                        normal,
                        color: Vec3::new(1.0, 1.0, 1.0),
                        extra: 0.0,
                    });
                }
            }
            for i in 0..segmentos {
                let j = (i + 1) % segmentos;
                let (i0, o0, i1, o1) = (base + 2 * i, base + 2 * i + 1, base + 2 * j, base + 2 * j + 1);
                if lado == 0 {
                    indices.push([i0, i1, o1]);
                    indices.push([i0, o1, o0]);
                } else {
                    indices.push([i0, o1, i1]);
                    indices.push([i0, o0, o1]);
                }
            }
        }
    }
    Malla { vertices, indices }
}

pub fn octaedro() -> Malla {
    let puntos = [
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, -1.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(-1.0, 0.0, 0.0),
        Vec3::new(0.0, 0.0, 1.0),
        Vec3::new(0.0, 0.0, -1.0),
    ];
    let vertices = puntos
        .iter()
        .map(|&p| Vertice {
            pos: p,
            normal: p,
            color: Vec3::new(1.0, 1.0, 1.0),
            extra: 0.0,
        })
        .collect();
    let mut indices = Vec::new();
    for polo in [0u32, 1] {
        for [a, b] in [[2u32, 4], [4, 3], [3, 5], [5, 2]] {
            let (pp, pa, pb) = (puntos[polo as usize], puntos[a as usize], puntos[b as usize]);
            if (pa - pp).cross(pb - pp).dot(pp + pa + pb) >= 0.0 {
                indices.push([polo, a, b]);
            } else {
                indices.push([polo, b, a]);
            }
        }
    }
    Malla { vertices, indices }
}

pub fn llama(segmentos: u32, anillos: u32) -> Malla {
    let mut vertices = vec![Vertice {
        pos: Vec3::new(0.0, -0.3, 0.0),
        normal: Vec3::CERO,
        color: Vec3::new(1.0, 1.0, 1.0),
        extra: 0.0,
    }];
    for i in 0..=anillos {
        let y = -0.3 + 1.3 * i as f32 / anillos as f32;
        let a = y.max(0.0);
        let radio = (1.0 - a).max(0.0).powf(0.55) * (1.0 + 0.3 * a);
        for j in 0..segmentos {
            let angulo = j as f32 / segmentos as f32 * std::f32::consts::TAU;
            vertices.push(Vertice {
                pos: Vec3::new(angulo.cos() * radio, y, angulo.sin() * radio),
                normal: Vec3::CERO,
                color: Vec3::new(1.0, 1.0, 1.0),
                extra: 0.0,
            });
        }
    }
    let anillo = |i: u32, j: u32| 1 + i * segmentos + j % segmentos;
    let mut indices = Vec::new();
    for j in 0..segmentos {
        indices.push([0, anillo(0, j), anillo(0, j + 1)]);
    }
    for i in 0..anillos {
        for j in 0..segmentos {
            indices.push([anillo(i, j), anillo(i + 1, j), anillo(i + 1, j + 1)]);
            indices.push([anillo(i, j), anillo(i + 1, j + 1), anillo(i, j + 1)]);
        }
    }
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

pub fn niveles_icosfera(maximo: usize) -> Vec<Icosfera> {
    let mut niveles = vec![Icosfera::base()];
    for i in 0..maximo {
        let siguiente = niveles[i].subdividir();
        niveles.push(siguiente);
    }
    niveles
}

pub fn malla_de_esfera(esfera: &Icosfera, superficie: impl Fn(Vec3) -> (f32, f32)) -> Malla {
    let mut vertices: Vec<Vertice> = esfera
        .direcciones
        .iter()
        .map(|&d| {
            let (radio, extra) = superficie(d);
            Vertice {
                pos: d * radio,
                normal: Vec3::CERO,
                color: Vec3::new(1.0, 1.0, 1.0),
                extra,
            }
        })
        .collect();
    for &[a, b, c] in &esfera.indices {
        let (pa, pb, pc) = (
            vertices[a as usize].pos,
            vertices[b as usize].pos,
            vertices[c as usize].pos,
        );
        let n = (pb - pa).cross(pc - pa);
        vertices[a as usize].normal = vertices[a as usize].normal + n;
        vertices[b as usize].normal = vertices[b as usize].normal + n;
        vertices[c as usize].normal = vertices[c as usize].normal + n;
    }
    for v in &mut vertices {
        v.normal = v.normal.normalizado();
    }
    Malla {
        vertices,
        indices: esfera.indices.clone(),
    }
}
