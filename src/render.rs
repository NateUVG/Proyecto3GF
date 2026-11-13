use crate::cielo::Cielo;
use crate::geometria::Malla;
use crate::shaders::{Fragmento, Global, Shader, sombrear};
use crate::vector::{Mat4, Vec3, Vec4};
use std::sync::Mutex;
use std::thread;

const ALTO_FRANJA: usize = 16;

pub struct Camara {
    pub pos: Vec3,
    pub objetivo: Vec3,
    pub arriba: Vec3,
    pub fov: f32,
    pub cerca: f32,
    pub lejos: f32,
}

pub struct Instancia<'a> {
    pub malla: &'a Malla,
    pub modelo: Mat4,
    pub centro: Vec3,
    pub radio: f32,
    pub shader: Shader,
    pub param: f32,
    pub oculta: bool,
}

pub struct Resplandor {
    pub pos: Vec3,
    pub radio: f32,
    pub visibilidad: f32,
}

#[derive(Default, Clone, Copy)]
pub struct Estadisticas {
    pub objetos_total: usize,
    pub objetos_visibles: usize,
    pub triangulos_enviados: usize,
    pub triangulos_dibujados: usize,
}

#[derive(Clone, Copy)]
struct Atributos {
    pos: Vec3,
    normal: Vec3,
    local: Vec3,
    color: Vec3,
    extra: f32,
}

impl Atributos {
    fn escalar(self, k: f32) -> Atributos {
        Atributos {
            pos: self.pos * k,
            normal: self.normal * k,
            local: self.local * k,
            color: self.color * k,
            extra: self.extra * k,
        }
    }

    fn mezclar(self, o: Atributos, t: f32) -> Atributos {
        Atributos {
            pos: self.pos.mezclar(o.pos, t),
            normal: self.normal.mezclar(o.normal, t),
            local: self.local.mezclar(o.local, t),
            color: self.color.mezclar(o.color, t),
            extra: self.extra + (o.extra - self.extra) * t,
        }
    }
}

#[derive(Clone, Copy)]
struct VerticeClip {
    c: Vec4,
    a: Atributos,
}

#[derive(Clone, Copy)]
struct VerticePantalla {
    x: f32,
    y: f32,
    inv_w: f32,
    a: Atributos,
}

#[derive(Default)]
struct Lote {
    clip: Vec<VerticeClip>,
    codigos: Vec<u8>,
    pantalla: Vec<VerticePantalla>,
    triangulos: Vec<[u32; 3]>,
    dibujados: usize,
    shader_centro: Vec3,
    shader_param: f32,
    shader: Option<Shader>,
}

#[derive(Clone, Copy)]
struct Punto {
    x: u32,
    inv_w: f32,
    color: u32,
}

pub struct Renderer {
    pub ancho: usize,
    pub alto: usize,
    pub color: Vec<u32>,
    profundidad: Vec<f32>,
    lotes: Vec<Lote>,
    franjas: Vec<Vec<(u32, u32)>>,
    puntos: Vec<Vec<(u32, Punto)>>,
    hilos: usize,
    pub descarte: bool,
    pub estadisticas: Estadisticas,
}

fn distancia_plano(c: Vec4, plano: usize) -> f32 {
    match plano {
        0 => c.w + c.x,
        1 => c.w - c.x,
        2 => c.w + c.y,
        3 => c.w - c.y,
        4 => c.w + c.z,
        _ => c.w - c.z,
    }
}

fn codigo_fuera(c: Vec4) -> u8 {
    let mut codigo = 0;
    for plano in 0..6 {
        if distancia_plano(c, plano) < 0.0 {
            codigo |= 1 << plano;
        }
    }
    codigo
}

fn a_pantalla(v: &VerticeClip, ancho: f32, alto: f32) -> VerticePantalla {
    let inv_w = 1.0 / v.c.w;
    VerticePantalla {
        x: (v.c.x * inv_w + 1.0) * 0.5 * ancho,
        y: (1.0 - v.c.y * inv_w) * 0.5 * alto,
        inv_w,
        a: v.a.escalar(inv_w),
    }
}

fn recortar(entrada: [VerticeClip; 3], salida: &mut Vec<VerticeClip>, auxiliar: &mut Vec<VerticeClip>) {
    salida.clear();
    salida.extend_from_slice(&entrada);
    for plano in 0..6 {
        auxiliar.clear();
        for i in 0..salida.len() {
            let actual = salida[i];
            let siguiente = salida[(i + 1) % salida.len()];
            let da = distancia_plano(actual.c, plano);
            let db = distancia_plano(siguiente.c, plano);
            if da >= 0.0 {
                auxiliar.push(actual);
            }
            if (da >= 0.0) != (db >= 0.0) {
                let t = da / (da - db);
                auxiliar.push(VerticeClip {
                    c: actual.c.mezclar(siguiente.c, t),
                    a: actual.a.mezclar(siguiente.a, t),
                });
            }
        }
        std::mem::swap(salida, auxiliar);
        if salida.is_empty() {
            return;
        }
    }
}

fn agregar_triangulo(lote: &mut Lote, a: u32, b: u32, c: u32) {
    let (pa, pb, pc) = (
        &lote.pantalla[a as usize],
        &lote.pantalla[b as usize],
        &lote.pantalla[c as usize],
    );
    let area = (pb.x - pa.x) * (pc.y - pa.y) - (pb.y - pa.y) * (pc.x - pa.x);
    if area < 0.0 {
        lote.triangulos.push([a, c, b]);
        lote.dibujados += 1;
    }
}

fn etapa_geometria(lote: &mut Lote, inst: &Instancia, vp: &Mat4, ancho: f32, alto: f32) {
    lote.clip.clear();
    lote.codigos.clear();
    lote.pantalla.clear();
    lote.triangulos.clear();
    lote.dibujados = 0;
    lote.shader = Some(inst.shader);
    lote.shader_centro = inst.centro;
    lote.shader_param = inst.param;

    let mvp = vp.por(&inst.modelo);
    for v in &inst.malla.vertices {
        let c = mvp.punto_clip(v.pos);
        let vertice = VerticeClip {
            c,
            a: Atributos {
                pos: inst.modelo.punto(v.pos),
                normal: inst.modelo.direccion(v.normal),
                local: v.pos,
                color: v.color,
                extra: v.extra,
            },
        };
        lote.clip.push(vertice);
        lote.codigos.push(codigo_fuera(c));
        lote.pantalla.push(a_pantalla(&vertice, ancho, alto));
    }

    let mut poligono = Vec::with_capacity(12);
    let mut auxiliar = Vec::with_capacity(12);
    for &[a, b, c] in &inst.malla.indices {
        let (ca, cb, cc) = (
            lote.codigos[a as usize],
            lote.codigos[b as usize],
            lote.codigos[c as usize],
        );
        if ca & cb & cc != 0 {
            continue;
        }
        if ca | cb | cc == 0 {
            agregar_triangulo(lote, a, b, c);
            continue;
        }
        recortar(
            [lote.clip[a as usize], lote.clip[b as usize], lote.clip[c as usize]],
            &mut poligono,
            &mut auxiliar,
        );
        if poligono.len() < 3 {
            continue;
        }
        let base = lote.pantalla.len() as u32;
        for v in &poligono {
            lote.pantalla.push(a_pantalla(v, ancho, alto));
        }
        for k in 1..poligono.len() as u32 - 1 {
            agregar_triangulo(lote, base, base + k, base + k + 1);
        }
    }
}

struct Franja<'a> {
    y_inicio: usize,
    color: &'a mut [u32],
    profundidad: &'a mut [f32],
    triangulos: &'a [(u32, u32)],
    puntos: &'a [(u32, Punto)],
}

struct VistaCielo<'a> {
    cielo: &'a Cielo,
    adelante: Vec3,
    derecha: Vec3,
    arriba: Vec3,
    media_alto: f32,
    media_ancho: f32,
}

struct Halo {
    inv_w: f32,
    x: f32,
    y: f32,
    radio: f32,
    visibilidad: f32,
}

fn rasterizar(f: &mut Franja, ancho: usize, lotes: &[Lote], global: &Global) {
    let filas = f.color.len() / ancho;
    let (y_min_franja, y_max_franja) = (f.y_inicio as i32, (f.y_inicio + filas - 1) as i32);
    for &(l, t) in f.triangulos {
        let lote = &lotes[l as usize];
        let [ia, ib, ic] = lote.triangulos[t as usize];
        let (a, b, c) = (
            &lote.pantalla[ia as usize],
            &lote.pantalla[ib as usize],
            &lote.pantalla[ic as usize],
        );
        let area = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
        if area <= 0.0 {
            continue;
        }
        let inv_area = 1.0 / area;

        let x_min = ((a.x.min(b.x).min(c.x) - 0.5).floor() as i32).max(0);
        let x_max = ((a.x.max(b.x).max(c.x) - 0.5).ceil() as i32).min(ancho as i32 - 1);
        let y_min = ((a.y.min(b.y).min(c.y) - 0.5).floor() as i32).max(y_min_franja);
        let y_max = ((a.y.max(b.y).max(c.y) - 0.5).ceil() as i32).min(y_max_franja);
        if x_min > x_max || y_min > y_max {
            continue;
        }

        let (dx0, dy0) = (-(c.y - b.y), c.x - b.x);
        let (dx1, dy1) = (-(a.y - c.y), a.x - c.x);
        let (dx2, dy2) = (-(b.y - a.y), b.x - a.x);
        let (px, py) = (x_min as f32 + 0.5, y_min as f32 + 0.5);
        let mut fila0 = dx0 * (px - b.x) + dy0 * (py - b.y);
        let mut fila1 = dx1 * (px - c.x) + dy1 * (py - c.y);
        let mut fila2 = dx2 * (px - a.x) + dy2 * (py - a.y);
        let shader = lote.shader.unwrap_or(Shader::Nave);

        for y in y_min..=y_max {
            let base = (y as usize - f.y_inicio) * ancho;
            let (mut e0, mut e1, mut e2) = (fila0, fila1, fila2);
            for x in x_min..=x_max {
                if e0 >= -0.001 && e1 >= -0.001 && e2 >= -0.001 {
                    let (l0, l1, l2) = (e0 * inv_area, e1 * inv_area, e2 * inv_area);
                    let inv_w = l0 * a.inv_w + l1 * b.inv_w + l2 * c.inv_w;
                    let i = base + x as usize;
                    if inv_w > f.profundidad[i] {
                        f.profundidad[i] = inv_w;
                        let w = 1.0 / inv_w;
                        let (k0, k1, k2) = (l0 * w, l1 * w, l2 * w);
                        let mezcla = |pa: Vec3, pb: Vec3, pc: Vec3| pa * k0 + pb * k1 + pc * k2;
                        let fragmento = Fragmento {
                            pos: mezcla(a.a.pos, b.a.pos, c.a.pos),
                            normal: mezcla(a.a.normal, b.a.normal, c.a.normal),
                            local: mezcla(a.a.local, b.a.local, c.a.local),
                            color: mezcla(a.a.color, b.a.color, c.a.color),
                            extra: a.a.extra * k0 + b.a.extra * k1 + c.a.extra * k2,
                        };
                        let salida = sombrear(shader, &fragmento, global, lote.shader_centro, lote.shader_param);
                        f.color[i] = empaquetar(salida);
                    }
                }
                e0 += dx0;
                e1 += dx1;
                e2 += dx2;
            }
            fila0 += dy0;
            fila1 += dy1;
            fila2 += dy2;
        }
    }
}

fn empaquetar(c: Vec3) -> u32 {
    let canal = |x: f32| (x.clamp(0.0, 1.0) * 255.0) as u32;
    (canal(c.x) << 16) | (canal(c.y) << 8) | canal(c.z)
}

fn pintar_cielo(f: &mut Franja, ancho: usize, alto: usize, vista: &VistaCielo) {
    let filas = f.color.len() / ancho;
    for fila in 0..filas {
        let y = f.y_inicio + fila;
        let ny = 1.0 - (y as f32 + 0.5) / alto as f32 * 2.0;
        let dir_fila = vista.adelante + vista.arriba * (ny * vista.media_alto);
        for x in 0..ancho {
            let i = fila * ancho + x;
            if f.profundidad[i] == 0.0 {
                let nx = (x as f32 + 0.5) / ancho as f32 * 2.0 - 1.0;
                f.color[i] = vista.cielo.muestrear(dir_fila + vista.derecha * (nx * vista.media_ancho));
            }
        }
    }
    for &(y, p) in f.puntos {
        let i = (y as usize - f.y_inicio) * ancho + p.x as usize;
        if p.inv_w > f.profundidad[i] {
            f.color[i] = p.color;
        }
    }
}

fn pintar_halo(f: &mut Franja, ancho: usize, h: &Halo) {
    let filas = f.color.len() / ancho;
    let r2 = h.radio * h.radio;
    for fila in 0..filas {
        let dy = (f.y_inicio + fila) as f32 + 0.5 - h.y;
        for x in 0..ancho {
            let dx = x as f32 + 0.5 - h.x;
            let d2 = dx * dx + dy * dy;
            let i = fila * ancho + x;
            if d2 > r2 * 70.0 || f.profundidad[i] > h.inv_w {
                continue;
            }
            let k = r2 / (d2 + r2 * 0.6);
            let fuerza = (k * 0.9 * h.visibilidad - 0.012).max(0.0).min(1.0);
            if fuerza > 0.0 {
                let c = f.color[i];
                let r = ((c >> 16) & 255) as f32 + 255.0 * fuerza;
                let g = ((c >> 8) & 255) as f32 + 190.0 * fuerza;
                let b = (c & 255) as f32 + 90.0 * fuerza;
                f.color[i] = ((r.min(255.0) as u32) << 16) | ((g.min(255.0) as u32) << 8) | b.min(255.0) as u32;
            }
        }
    }
}

impl Renderer {
    pub fn new(ancho: usize, alto: usize) -> Renderer {
        let mut r = Renderer {
            ancho: 0,
            alto: 0,
            color: Vec::new(),
            profundidad: Vec::new(),
            lotes: Vec::new(),
            franjas: Vec::new(),
            puntos: Vec::new(),
            hilos: thread::available_parallelism().map_or(4, |n| n.get()),
            descarte: true,
            estadisticas: Estadisticas::default(),
        };
        r.redimensionar(ancho, alto);
        r
    }

    pub fn redimensionar(&mut self, ancho: usize, alto: usize) {
        self.ancho = ancho;
        self.alto = alto;
        self.color = vec![0; ancho * alto];
        self.profundidad = vec![0.0; ancho * alto];
        let n = alto.div_ceil(ALTO_FRANJA);
        self.franjas = vec![Vec::new(); n];
        self.puntos = vec![Vec::new(); n];
    }

    pub fn vista_proyeccion(&self, camara: &Camara) -> Mat4 {
        let aspecto = self.ancho as f32 / self.alto as f32;
        let vista = Mat4::mirar(camara.pos, camara.objetivo, camara.arriba);
        let proyeccion = Mat4::perspectiva(camara.fov, aspecto, camara.cerca, camara.lejos);
        proyeccion.por(&vista)
    }

    fn agregar_punto(&mut self, x: i32, y: i32, inv_w: f32, color: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.ancho && (y as usize) < self.alto {
            self.puntos[y as usize / ALTO_FRANJA].push((
                y as u32,
                Punto {
                    x: x as u32,
                    inv_w,
                    color,
                },
            ));
        }
    }

    fn proyectar(&self, vp: &Mat4, p: Vec3) -> Option<(f32, f32, f32)> {
        let c = vp.punto_clip(p);
        if c.w <= 0.0 || distancia_plano(c, 4) < 0.0 {
            return None;
        }
        let inv_w = 1.0 / c.w;
        Some((
            (c.x * inv_w + 1.0) * 0.5 * self.ancho as f32,
            (1.0 - c.y * inv_w) * 0.5 * self.alto as f32,
            inv_w,
        ))
    }

    pub fn a_pantalla(&self, camara: &Camara, p: Vec3) -> Option<(f32, f32)> {
        self.proyectar(&self.vista_proyeccion(camara), p).map(|(x, y, _)| (x, y))
    }

    fn preparar_puntos(&mut self, vp: &Mat4, camara: &Camara, cielo: &Cielo, puntos: &[(Vec3, Vec3)]) {
        for lista in &mut self.puntos {
            lista.clear();
        }
        for e in &cielo.estrellas {
            if let Some((x, y, _)) = self.proyectar(vp, camara.pos + e.dir * 3000.0) {
                let (xi, yi) = (x as i32, y as i32);
                let color = empaquetar(e.color);
                self.agregar_punto(xi, yi, 1e-7, color);
                if e.grande {
                    let tenue = empaquetar(e.color * 0.55);
                    self.agregar_punto(xi + 1, yi, 1e-7, tenue);
                    self.agregar_punto(xi - 1, yi, 1e-7, tenue);
                    self.agregar_punto(xi, yi + 1, 1e-7, tenue);
                    self.agregar_punto(xi, yi - 1, 1e-7, tenue);
                }
            }
        }
        for &(p, color) in puntos {
            if let Some((x, y, inv_w)) = self.proyectar(vp, p) {
                self.agregar_punto(x as i32, y as i32, inv_w, empaquetar(color));
            }
        }
    }

    fn es_visible(vp: &Mat4, centro: Vec3, radio: f32) -> bool {
        let c = centro;
        let m = &vp.m;
        for plano in 0..6 {
            let eje = plano / 2;
            let signo = if plano % 2 == 0 { 1.0 } else { -1.0 };
            let n = Vec3::new(
                m[0][3] + signo * m[0][eje],
                m[1][3] + signo * m[1][eje],
                m[2][3] + signo * m[2][eje],
            );
            let d = m[3][3] + signo * m[3][eje];
            if n.dot(c) + d < -radio * n.largo() {
                return false;
            }
        }
        true
    }

    pub fn dibujar(
        &mut self,
        instancias: &[Instancia],
        camara: &Camara,
        global: &Global,
        cielo: &Cielo,
        puntos: &[(Vec3, Vec3)],
        resplandor: Option<Resplandor>,
    ) {
        let vp = self.vista_proyeccion(camara);
        self.preparar_puntos(&vp, camara, cielo, puntos);
        let (ancho_f, alto_f) = (self.ancho as f32, self.alto as f32);

        let mut visibles: Vec<&Instancia> = instancias
            .iter()
            .filter(|i| !i.oculta && (!self.descarte || Renderer::es_visible(&vp, i.centro, i.radio)))
            .collect();
        visibles.sort_by(|a, b| {
            let da = (a.centro - camara.pos).largo() - a.radio;
            let db = (b.centro - camara.pos).largo() - b.radio;
            da.total_cmp(&db)
        });

        if self.lotes.len() < visibles.len() {
            self.lotes.resize_with(visibles.len(), Lote::default);
        }

        let hilos = self.hilos;
        {
            let trabajos = Mutex::new(self.lotes.iter_mut().zip(visibles.iter()));
            thread::scope(|s| {
                for _ in 0..hilos {
                    s.spawn(|| {
                        loop {
                            let siguiente = trabajos.lock().unwrap().next();
                            let Some((lote, inst)) = siguiente else { break };
                            etapa_geometria(lote, inst, &vp, ancho_f, alto_f);
                        }
                    });
                }
            });
        }

        for franja in &mut self.franjas {
            franja.clear();
        }
        let n_franjas = self.franjas.len() as i32;
        let mut dibujados = 0;
        for (l, lote) in self.lotes[..visibles.len()].iter().enumerate() {
            dibujados += lote.dibujados;
            for (t, &[a, b, c]) in lote.triangulos.iter().enumerate() {
                let (ya, yb, yc) = (
                    lote.pantalla[a as usize].y,
                    lote.pantalla[b as usize].y,
                    lote.pantalla[c as usize].y,
                );
                let y_min = (ya.min(yb).min(yc) - 0.5).floor() as i32;
                let y_max = (ya.max(yb).max(yc) - 0.5).ceil() as i32;
                let f0 = (y_min.max(0) / ALTO_FRANJA as i32).min(n_franjas);
                let f1 = (y_max.max(0) / ALTO_FRANJA as i32).min(n_franjas - 1);
                for f in f0..=f1 {
                    self.franjas[f as usize].push((l as u32, t as u32));
                }
            }
        }

        self.estadisticas = Estadisticas {
            objetos_total: instancias.len(),
            objetos_visibles: visibles.len(),
            triangulos_enviados: visibles.iter().map(|i| i.malla.indices.len()).sum(),
            triangulos_dibujados: dibujados,
        };

        let adelante = (camara.objetivo - camara.pos).normalizado();
        let derecha = adelante.cross(camara.arriba).normalizado();
        let arriba = derecha.cross(adelante);
        let media_alto = (camara.fov * 0.5).tan();
        let vista = VistaCielo {
            cielo,
            adelante,
            derecha,
            arriba,
            media_alto,
            media_ancho: media_alto * ancho_f / alto_f,
        };
        let halo = resplandor.and_then(|r| {
            let c = vp.punto_clip(r.pos);
            if c.w <= 0.0 || r.visibilidad <= 0.0 {
                return None;
            }
            let distancia = (r.pos - camara.pos).largo();
            let radio = r.radio / distancia / media_alto * alto_f * 0.5;
            Some(Halo {
                inv_w: 1.0 / c.w,
                x: (c.x / c.w + 1.0) * 0.5 * ancho_f,
                y: (1.0 - c.y / c.w) * 0.5 * alto_f,
                radio,
                visibilidad: r.visibilidad,
            })
        });

        let (ancho, alto) = (self.ancho, self.alto);
        self.profundidad.fill(0.0);
        let lotes = &self.lotes[..visibles.len()];
        let tareas = Mutex::new(
            self.color
                .chunks_mut(ancho * ALTO_FRANJA)
                .zip(self.profundidad.chunks_mut(ancho * ALTO_FRANJA))
                .zip(self.franjas.iter().zip(self.puntos.iter()))
                .enumerate(),
        );
        thread::scope(|s| {
            for _ in 0..hilos {
                s.spawn(|| {
                    loop {
                        let siguiente = tareas.lock().unwrap().next();
                        let Some((i, ((color, profundidad), (triangulos, puntos)))) = siguiente else {
                            break;
                        };
                        let mut franja = Franja {
                            y_inicio: i * ALTO_FRANJA,
                            color,
                            profundidad,
                            triangulos,
                            puntos,
                        };
                        rasterizar(&mut franja, ancho, lotes, global);
                        pintar_cielo(&mut franja, ancho, alto, &vista);
                        if let Some(h) = &halo {
                            pintar_halo(&mut franja, ancho, h);
                        }
                    }
                });
            }
        });
    }
}
