use crate::cielo::Cielo;
use crate::config::*;
use crate::efectos;
use crate::geometria::{Malla, anillo, llama, malla_de_esfera, niveles_icosfera, octaedro};
use crate::efectos::Particula;
use crate::nave::{Modo, Nave};
use crate::obj;
use crate::planeta::{Planeta, Terreno};
use crate::render::{Camara, Instancia, Resplandor};
use crate::ruido::{Azar, Perlin};
use crate::shaders::Shader;
use crate::vector::{Mat4, Vec3, suave};
use std::f32::consts::TAU;

pub struct Modelo {
    pub malla: Malla,
    pub radio: f32,
}

impl Modelo {
    fn cargar(ruta: &str, tamano: f32, material_extra: &str) -> Result<Modelo, String> {
        let malla = obj::cargar(ruta, tamano, material_extra, true)?;
        let radio = malla.radio();
        Ok(Modelo { malla, radio })
    }
}

pub struct Mundo {
    pub ruido: Perlin,
    pub modelos: Vec<Modelo>,
    pub sol: Vec<Malla>,
    pub planetas: Vec<Planeta>,
    pub lunas: Vec<Vec<Malla>>,
    pub anillos: Vec<Option<Malla>>,
    pub volcanes: Vec<Vec<(Vec3, Vec3)>>,
    pub nieve: Vec<Vec<(Vec3, f32)>>,
    pub chispa: Malla,
    pub llama: Malla,
    pub nave: Malla,
    pub toberas: Vec<(Vec3, f32)>,
    pub cielo: Cielo,
    pub orbitas: Vec<(Vec3, Vec3)>,
}

fn toberas(malla: &Malla) -> Vec<(Vec3, f32)> {
    let traseras: Vec<[Vec3; 3]> = malla
        .indices
        .iter()
        .map(|t| t.map(|i| malla.vertices[i as usize]))
        .filter(|v| v.iter().all(|v| v.extra > 0.5 && v.normal.x < -0.9))
        .map(|v| v.map(|v| v.pos))
        .collect();
    let mut grupos: Vec<Vec<Vec3>> = Vec::new();
    for t in traseras {
        let unidos: Vec<usize> = (0..grupos.len())
            .filter(|&g| grupos[g].iter().any(|p| t.iter().any(|q| (*p - *q).largo() < 1e-3)))
            .collect();
        let mut nuevo = t.to_vec();
        for &g in unidos.iter().rev() {
            nuevo.extend(grupos.remove(g));
        }
        grupos.push(nuevo);
    }
    grupos
        .into_iter()
        .map(|g| {
            let centro = g.iter().fold(Vec3::CERO, |a, &p| a + p) * (1.0 / g.len() as f32);
            let radio = g.iter().map(|p| Vec3::new(0.0, p.y - centro.y, p.z - centro.z).largo()).fold(0.0, f32::max);
            (centro, radio)
        })
        .filter(|t| t.0.x < 0.0)
        .collect()
}

pub fn nivel_por_distancia(radio: f32, distancia: f32, alto: f32, fov: f32) -> usize {
    let pixeles = radio / distancia.max(0.001) / (fov * 0.5).tan() * alto * 0.5;
    match pixeles {
        p if p < 10.0 => 1,
        p if p < 28.0 => 2,
        p if p < 65.0 => 3,
        p if p < 140.0 => 4,
        p if p < 300.0 => 5,
        p if p < 420.0 => 6,
        _ => NIVEL_MAXIMO,
    }
}

impl Mundo {
    pub fn new(semilla: u64) -> Result<Mundo, String> {
        let ruido = Perlin::new(SEMILLA);
        let esferas = niveles_icosfera(NIVEL_MAXIMO);
        let sol = esferas[..=6].iter().map(|e| malla_de_esfera(e, |_| (RADIO_SOL, 0.0))).collect();
        let planetas: Vec<Planeta> = PLANETAS.iter().map(|d| Planeta::new(d, &esferas, &ruido, semilla)).collect();
        let lunas = LUNAS
            .iter()
            .map(|l| esferas[..=5].iter().map(|e| malla_de_esfera(e, |d| {
                let (h, extra) = l.perfil(&ruido, d);
                (l.radio + h, extra)
            })).collect())
            .collect();
        let anillos = PLANETAS.iter().map(|d| d.anillos.then(|| anillo(ANILLOS, d.radio, 160))).collect();
        let mut orbitas = Vec::new();
        for d in PLANETAS {
            let puntos = (d.radio_orbita * 2.2) as usize;
            for i in 0..puntos {
                let t = i as f32 / puntos as f32 * d.periodo_orbita / VELOCIDAD_TIEMPO;
                orbitas.push((d.posicion(t), Vec3::new(0.42, 0.40, 0.55)));
            }
        }
        let modelos = vec![
            Modelo::cargar("assets/antena.obj", 2.2, "Luz")?,
            Modelo::cargar("assets/torre.obj", 3.6, "Luz")?,
            Modelo::cargar("assets/roca.obj", 1.5, "")?,
            Modelo::cargar("assets/cristal.obj", 2.2, "Cristal")?,
            Modelo::cargar("assets/hongo.obj", 1.5, "")?,
            Modelo::cargar("assets/pino.obj", 2.6, "Luz")?,
            Modelo::cargar("assets/regalo.obj", 1.0, "")?,
            Modelo::cargar("assets/estrella.obj", 5.5, "Luz")?,
        ];
        let volcanes = planetas
            .iter()
            .map(|p| match p.def.terreno {
                Terreno::Lava => p
                    .decoraciones
                    .iter()
                    .take(VOLCANES)
                    .map(|d| {
                        (d.pos + d.arriba * (modelos[2].radio * d.escala * 0.6), d.arriba)
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .collect();
        let nieve = planetas
            .iter()
            .map(|p| match p.def.terreno {
                Terreno::Navidad => {
                    let mut azar = Azar::new(SEMILLA ^ 0xC0F0);
                    (0..COPOS)
                        .map(|_| {
                            let z = azar.unidad() * 2.0 - 1.0;
                            let a = azar.unidad() * TAU;
                            let r = (1.0 - z * z).sqrt();
                            let d = Vec3::new(r * a.cos(), z, r * a.sin());
                            (d, p.def.radio_en(&ruido, d))
                        })
                        .collect()
                }
                _ => Vec::new(),
            })
            .collect();
        let nave = obj::cargar("assets/nave.obj", LARGO_NAVE, "Motor", false)?;
        Ok(Mundo {
            toberas: toberas(&nave),
            nave,
            nieve,
            modelos,
            volcanes,
            chispa: octaedro(),
            llama: llama(14, 10),
            ruido,
            sol,
            planetas,
            lunas,
            anillos,
            cielo: Cielo::new(),
            orbitas,
        })
    }

    pub fn toberas_mundo(&self, nave: &Nave) -> Vec<Vec3> {
        let modelo = nave.modelo();
        self.toberas.iter().map(|&(p, _)| modelo.punto(p)).collect()
    }

    pub fn instancias(
        &self,
        t: f32,
        nave: &Nave,
        estela: &[Particula],
        camara: &Camara,
        alto: f32,
        lod: bool,
        descarte: bool,
    ) -> (Vec<Instancia<'_>>, Vec<usize>) {
        let mut lista = Vec::new();
        let mut usados = Vec::new();
        let elegir = |radio: f32, centro: Vec3, maximo: usize| {
            if lod {
                nivel_por_distancia(radio, (centro - camara.pos).largo(), alto, camara.fov).min(maximo)
            } else {
                maximo
            }
        };
        let foco = alto * 0.5 / (camara.fov * 0.5).tan();
        let nivel_sol = elegir(RADIO_SOL, Vec3::CERO, self.sol.len() - 1);
        lista.push(Instancia {
            malla: &self.sol[nivel_sol],
            modelo: Mat4::rotacion_y(t * 0.02),
            centro: Vec3::CERO,
            radio: RADIO_SOL,
            shader: Shader::Sol,
            param: 0.0,
            oculta: false,
        });
        for (luna, niveles) in LUNAS.iter().zip(&self.lunas) {
            let centro = luna.posicion(t);
            let radio = luna.radio + luna.profundidad;
            let nivel = elegir(radio, centro, niveles.len() - 1);
            lista.push(Instancia {
                malla: &niveles[nivel],
                modelo: luna.modelo(t),
                centro,
                radio,
                shader: Shader::Luna,
                param: 0.0,
                oculta: false,
            });
        }
        for (indice_planeta, p) in self.planetas.iter().enumerate() {
            let centro = p.def.posicion(t);
            let radio = p.def.radio_limite();
            let nivel = elegir(radio, centro, p.niveles.len() - 1);
            usados.push(nivel);
            let transformacion = Mat4::traslacion(centro).por(&p.def.rotacion(t));
            let (shader, modelos_planeta): (Shader, &[usize]) = match p.def.terreno {
                Terreno::Tecno => (Shader::Tecno, &[0, 1]),
                Terreno::Lava => (Shader::Lava, &[2]),
                Terreno::Hielo => (Shader::Hielo, &[3]),
                Terreno::Dulce => (Shader::Dulce, &[4]),
                Terreno::Roto => (Shader::Roto, &[3, 2]),
                Terreno::Navidad => (Shader::Navidad, &[5, 6]),
            };
            lista.push(Instancia {
                malla: &p.niveles[nivel],
                modelo: transformacion,
                centro,
                radio,
                shader,
                param: p.def.radio,
                oculta: false,
            });

            if p.def.terreno == Terreno::Navidad {
                let estrella = &self.modelos[7];
                let punta = Vec3::new(0.0, p.def.radio * (1.0 + ALTURA_PICO) - 0.2, 0.0);
                lista.push(Instancia {
                    malla: &estrella.malla,
                    modelo: transformacion.por(&Mat4::traslacion(punta)).por(&Mat4::rotacion_y(t * 0.8)),
                    centro: transformacion.punto(punta),
                    radio: estrella.radio,
                    shader: Shader::Decoracion,
                    param: 0.0,
                    oculta: false,
                });
                if (camara.pos - centro).largo() < radio * 4.0 {
                    for (i, &(d, suelo)) in self.nieve[indice_planeta].iter().enumerate() {
                        let fase = (t / CAIDA_NIEVE + i as f32 * 0.618_034).fract();
                        let lado = d.cross(Vec3::new(0.3, 0.9, 0.3)).normalizado();
                        let vaiven = (t * 0.9 + i as f32 * 1.7).sin() * 0.35;
                        let local = d * (suelo + 0.1 + ALTURA_NIEVE * (1.0 - fase)) + lado * vaiven;
                        let pos = transformacion.punto(local);
                        let pixel = (pos - camara.pos).largo() / foco;
                        let tamano = TAMANO_COPO.max(pixel * 1.2);
                        lista.push(Instancia {
                            malla: &self.chispa,
                            modelo: Mat4::desde_ejes(
                                Vec3::new(tamano, 0.0, 0.0),
                                Vec3::new(0.0, tamano, 0.0),
                                Vec3::new(0.0, 0.0, tamano),
                                pos,
                            ),
                            centro: pos,
                            radio: tamano,
                            shader: Shader::Copo,
                            param: 0.0,
                            oculta: lod && TAMANO_COPO / pixel < 0.5,
                        });
                    }
                }
            }

            if p.def.terreno == Terreno::Roto {
                let roca = &self.modelos[2];
                for i in 0..ESCOMBROS_ROTO {
                    let mut azar = Azar::new(SEMILLA + i as u64);
                    let distancia = p.def.radio * (NUCLEO_ROTO + 0.1 + (1.0 - GROSOR_ROTO + SEPARACION_ROTO - NUCLEO_ROTO - 0.28) * azar.unidad());
                    let plano = Mat4::rotacion_x(azar.unidad() * 3.0 - 1.5).por(&Mat4::rotacion_z(azar.unidad() * TAU));
                    let a = azar.unidad() * TAU + t * (0.15 + 0.25 * azar.unidad());
                    let giro = Mat4::rotacion_y(t * (0.5 + azar.unidad())).por(&Mat4::rotacion_x(azar.unidad() * TAU));
                    let escala = 0.15 + 0.2 * azar.unidad();
                    let local = plano.punto(Vec3::new(a.cos(), 0.0, a.sin()) * distancia);
                    let centro_roca = transformacion.punto(local);
                    let radio_roca = roca.radio * escala;
                    lista.push(Instancia {
                        malla: &roca.malla,
                        modelo: transformacion.por(&Mat4::traslacion(local)).por(&giro).por(&Mat4::desde_ejes(
                            Vec3::new(escala, 0.0, 0.0),
                            Vec3::new(0.0, escala, 0.0),
                            Vec3::new(0.0, 0.0, escala),
                            Vec3::CERO,
                        )),
                        centro: centro_roca,
                        radio: radio_roca,
                        shader: Shader::Decoracion,
                        param: 0.0,
                        oculta: lod && radio_roca * foco / (centro_roca - camara.pos).largo() < 1.2,
                    });
                }
            }

            if let Some(malla) = &self.anillos[indice_planeta] {
                lista.push(Instancia {
                    malla,
                    modelo: p.def.marco_anillo(t),
                    centro,
                    radio: p.def.radio * ANILLOS[ANILLOS.len() - 1].1,
                    shader: Shader::Anillo,
                    param: p.def.radio,
                    oculta: false,
                });
            }

            for c in efectos::chispas(&self.volcanes[indice_planeta], t) {
                let pos = transformacion.punto(c.pos);
                let direccion = transformacion.direccion(c.direccion).normalizado();
                let distancia = (pos - camara.pos).largo();
                let pixel = distancia / foco;
                let largo = c.largo.max(pixel * 3.0);
                let ancho = ANCHO_CHISPA.max(pixel * 1.1);
                let auxiliar = if direccion.y.abs() < 0.9 { Vec3::new(0.0, 1.0, 0.0) } else { Vec3::new(1.0, 0.0, 0.0) };
                let e1 = auxiliar.cross(direccion).normalizado();
                let e2 = e1.cross(direccion);
                lista.push(Instancia {
                    malla: &self.chispa,
                    modelo: Mat4::desde_ejes(e1 * ancho, direccion * largo, e2 * ancho, pos),
                    centro: pos,
                    radio: largo,
                    shader: Shader::Chispa,
                    param: c.edad,
                    oculta: lod && c.largo / pixel < 0.7,
                });
            }

            let hacia_camara = camara.pos - centro;
            let distancia_planeta = hacia_camara.largo();
            let horizonte = p.def.radio * p.def.radio;
            for d in &p.decoraciones {
                let modelo = &self.modelos[modelos_planeta[d.variante % modelos_planeta.len()]];
                let radio_objeto = modelo.radio * d.escala;
                let base = transformacion.punto(d.pos);
                let distancia = (base - camara.pos).largo();
                let diminuta = lod && radio_objeto * foco / distancia < 1.2;
                let detras = descarte
                    && (base - centro).dot(hacia_camara) + radio_objeto * distancia_planeta < horizonte;
                lista.push(Instancia {
                    malla: &modelo.malla,
                    modelo: transformacion.por(&d.local),
                    centro: base,
                    radio: radio_objeto,
                    shader: Shader::Decoracion,
                    param: 0.0,
                    oculta: diminuta || detras,
                });
            }
        }
        lista.push(Instancia {
            malla: &self.nave,
            modelo: nave.modelo(),
            centro: nave.pos,
            radio: LARGO_NAVE,
            shader: Shader::Nave,
            param: nave.empuje,
            oculta: false,
        });
        let atras = -nave.adelante;
        let modelo_nave = nave.modelo();
        for (i, &(p, radio)) in self.toberas.iter().enumerate() {
            if nave.empuje < 0.02 {
                break;
            }
            let pos = modelo_nave.punto(p);
            let parpadeo = 0.85 + 0.15 * (t * 45.0 + i as f32 * 2.1).sin();
            let largo = LARGO_LLAMA * (0.2 + nave.empuje * (1.0 + (nave.empuje - 0.7).max(0.0) * 2.0)) * parpadeo;
            let ancho = radio * (0.7 + 0.3 * nave.empuje);
            lista.push(Instancia {
                malla: &self.llama,
                modelo: Mat4::desde_ejes(nave.arriba * ancho, atras * largo, nave.derecha() * ancho, pos),
                centro: pos + atras * (largo * 0.5),
                radio: largo,
                shader: Shader::Llama,
                param: nave.empuje,
                oculta: false,
            });
        }
        for particula in estela {
            let edad = ((t - particula.nacimiento) / VIDA_ESTELA).clamp(0.0, 1.0);
            let pixel = (particula.pos - camara.pos).largo() / foco;
            let tamano = (TAMANO_ESTELA * (1.0 - 0.6 * edad)).max(pixel * 0.8);
            lista.push(Instancia {
                malla: &self.chispa,
                modelo: Mat4::desde_ejes(
                    Vec3::new(tamano, 0.0, 0.0),
                    Vec3::new(0.0, tamano, 0.0),
                    Vec3::new(0.0, 0.0, tamano),
                    particula.pos,
                ),
                centro: particula.pos,
                radio: tamano,
                shader: Shader::Chispa,
                param: edad,
                oculta: false,
            });
        }
        let rapidez = nave.vel.largo();
        if nave.modo == Modo::Vuelo && rapidez > RAPIDEZ_POLVO {
            let direccion = nave.vel * (1.0 / rapidez);
            let largo = (rapidez * 0.03).min(3.0);
            let auxiliar = if direccion.y.abs() < 0.9 { Vec3::new(0.0, 1.0, 0.0) } else { Vec3::new(1.0, 0.0, 0.0) };
            let e1 = auxiliar.cross(direccion).normalizado();
            let e2 = e1.cross(direccion);
            let intensidad = suave(RAPIDEZ_POLVO, RAPIDEZ_POLVO * 3.0, rapidez);
            for pos in efectos::polvo(camara.pos) {
                let distancia = (pos - camara.pos).largo();
                let ancho = (distancia / foco).max(0.01);
                lista.push(Instancia {
                    malla: &self.chispa,
                    modelo: Mat4::desde_ejes(e1 * ancho, direccion * largo, e2 * ancho, pos),
                    centro: pos,
                    radio: largo,
                    shader: Shader::Polvo,
                    param: intensidad * (1.0 - distancia / (CELDA_POLVO * 2.5)).max(0.0) * 0.8,
                    oculta: false,
                });
            }
        }
        (lista, usados)
    }

    pub fn resplandor(&self, t: f32, camara: &Camara) -> Resplandor {
        let hacia_sol = Vec3::CERO - camara.pos;
        let distancia = hacia_sol.largo();
        let d = hacia_sol * (1.0 / distancia);
        let mut visibilidad: f32 = 1.0;
        for p in &self.planetas {
            let r = p.def.posicion(t) - camara.pos;
            let a_lo_largo = r.dot(d);
            if a_lo_largo > 0.0 && a_lo_largo < distancia {
                let separacion = (r - d * a_lo_largo).largo();
                let cubierto = 1.0 - ((separacion - p.def.radio) / (RADIO_SOL * 0.6)).clamp(0.0, 1.0);
                visibilidad = visibilidad.min(1.0 - cubierto);
            }
        }
        Resplandor {
            pos: Vec3::CERO,
            radio: RADIO_SOL,
            visibilidad,
        }
    }

    pub fn limitar_camara(&self, t: f32, camara: &mut Camara) {
        for luna in LUNAS {
            let centro = luna.posicion(t);
            let r = camara.pos - centro;
            let d = r.largo();
            let minimo = luna.radio + luna.profundidad + 0.4;
            if d < minimo {
                camara.pos = centro + r * (minimo / d);
            }
        }
        for p in &self.planetas {
            let centro = p.def.posicion(t);
            let r = camara.pos - centro;
            let d = r.largo();
            if d < p.def.radio * ZONA_SALIDA {
                let n = r * (1.0 / d);
                let local = p.def.rotacion(t).direccion_inversa(n);
                let minimo = p.def.radio_en(&self.ruido, local) + 0.4;
                if d < minimo {
                    camara.pos = centro + n * minimo;
                }
            }
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn la_nave_tiene_tres_toberas() {
        let nave = obj::cargar("assets/nave.obj", LARGO_NAVE, "Motor", false).unwrap();
        let lista = toberas(&nave);
        assert_eq!(lista.len(), 3, "{:?}", lista.iter().map(|t| (t.0.x, t.0.y, t.0.z, t.1)).collect::<Vec<_>>());
    }
}
