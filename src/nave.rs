use crate::config::*;
use crate::render::Camara;
use crate::ruido::Perlin;
use crate::vector::{Mat4, Vec3};

#[derive(Clone, Copy, Default)]
pub struct Entrada {
    pub avance: f32,
    pub lateral: f32,
    pub vertical: f32,
    pub cabeceo: f32,
    pub guinada: f32,
    pub alabeo: f32,
    pub freno: bool,
    pub turbo: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Modo {
    Vuelo,
    Local(usize),
    Anillo(usize),
}

pub struct Nave {
    pub pos: Vec3,
    pub vel: Vec3,
    pub adelante: Vec3,
    pub arriba: Vec3,
    pub giro: Vec3,
    pub empuje: f32,
    pub modo: Modo,
    pub altura_objetivo: f32,
    pub local_anulado: bool,
    pub lado_anillo: f32,
    borde_volteo: f32,
    rapidez_volteo: f32,
}

fn respuesta(k: f32, dt: f32) -> f32 {
    1.0 - (-k * dt).exp()
}

fn girar_hacia(v: Vec3, objetivo: Vec3, respaldo: Vec3) -> (Vec3, f32) {
    let angulo = v.dot(objetivo).clamp(-1.0, 1.0).acos();
    let eje = v.cross(objetivo);
    let eje = if eje.largo() > 1e-3 { eje.normalizado() } else { respaldo };
    (eje, angulo)
}

impl Nave {
    pub fn new() -> Nave {
        let (x, y, z) = POSICION_INICIAL;
        Nave {
            pos: Vec3::new(x, y, z),
            vel: Vec3::CERO,
            adelante: Vec3::new(0.0, -0.06, -1.0).normalizado(),
            arriba: Vec3::new(0.0, 1.0, -0.06).normalizado(),
            giro: Vec3::CERO,
            empuje: 0.0,
            modo: Modo::Vuelo,
            altura_objetivo: 0.0,
            local_anulado: false,
            lado_anillo: 1.0,
            borde_volteo: 0.0,
            rapidez_volteo: 0.0,
        }
    }

    pub fn derecha(&self) -> Vec3 {
        self.adelante.cross(self.arriba)
    }

    pub fn modelo(&self) -> Mat4 {
        Mat4::desde_ejes(self.adelante, self.arriba, self.derecha(), self.pos)
    }

    pub fn alternar_local(&mut self, ruido: &Perlin, tiempo: f32) {
        match self.modo {
            Modo::Local(_) => {
                self.modo = Modo::Vuelo;
                self.local_anulado = true;
            }
            Modo::Anillo(_) => self.modo = Modo::Vuelo,
            Modo::Vuelo => {
                self.local_anulado = false;
                for (i, def) in PLANETAS.iter().enumerate() {
                    if (self.pos - def.posicion(tiempo)).largo() < def.radio * ZONA_SALIDA {
                        self.entrar_local(i, ruido, tiempo);
                    }
                }
            }
        }
    }

    pub fn anillo_cercano(&self, tiempo: f32) -> Option<usize> {
        PLANETAS.iter().position(|def| {
            let (h, r, _) = def.coordenadas_anillo(self.pos, tiempo);
            h.abs() < DISTANCIA_AGARRE && def.sobre_anillo(r, 1.0)
        })
    }

    pub fn alternar_anillo(&mut self, tiempo: f32) {
        if let Modo::Anillo(_) = self.modo {
            self.modo = Modo::Vuelo;
            return;
        }
        if let Some(i) = self.anillo_cercano(tiempo) {
            let (h, _, _) = PLANETAS[i].coordenadas_anillo(self.pos, tiempo);
            self.modo = Modo::Anillo(i);
            self.lado_anillo = if h < 0.0 { -1.0 } else { 1.0 };
            self.altura_objetivo = ALTURA_ANILLO;
            self.borde_volteo = 0.0;
        }
    }

    fn superficie_local(&self, i: usize, ruido: &Perlin, tiempo: f32) -> (Vec3, f32) {
        let def = &PLANETAS[i];
        let radial = self.pos - def.posicion(tiempo);
        let distancia = radial.largo();
        let rotacion = def.rotacion(tiempo);
        let local = rotacion.direccion_inversa(radial * (1.0 / distancia));
        (rotacion.direccion(def.normal_superficie(local)), distancia - def.radio_en(ruido, local))
    }

    fn entrar_local(&mut self, i: usize, ruido: &Perlin, tiempo: f32) {
        self.modo = Modo::Local(i);
        self.altura_objetivo = self.superficie_local(i, ruido, tiempo).1;
    }

    fn orientar(&mut self) {
        self.arriba = self.arriba.normalizado();
        self.adelante = (self.adelante - self.arriba * self.adelante.dot(self.arriba)).normalizado();
    }

    fn paso_vuelo(&mut self, e: &Entrada, dt: f32) {
        let objetivo = Vec3::new(e.cabeceo, e.guinada, e.alabeo) * VELOCIDAD_GIRO;
        self.giro = self.giro.mezclar(objetivo, respuesta(RESPUESTA_GIRO, dt));
        let derecha = self.derecha();
        let (adelante, arriba) = (self.adelante, self.arriba);
        self.adelante = adelante
            .girar(derecha, self.giro.x * dt)
            .girar(arriba, self.giro.y * dt);
        self.arriba = arriba
            .girar(derecha, self.giro.x * dt)
            .girar(self.adelante, self.giro.z * dt);
        self.orientar();

        let fuerza = EMPUJE * if e.turbo { FACTOR_TURBO } else { 1.0 };
        let empuje = self.adelante * e.avance + self.derecha() * (e.lateral * 0.6);
        self.vel = self.vel + empuje * (fuerza * dt);
        let resistencia = if e.freno { RESISTENCIA_FRENO } else { RESISTENCIA };
        self.vel = self.vel * (-resistencia * dt).exp();
    }

    fn paso_local(&mut self, e: &Entrada, i: usize, ruido: &Perlin, tiempo: f32, dt: f32) {
        let (n, altura) = self.superficie_local(i, ruido, tiempo);

        self.giro = self
            .giro
            .mezclar(Vec3::new(0.0, e.guinada * VELOCIDAD_GIRO, 0.0), respuesta(RESPUESTA_GIRO, dt));
        let (eje, angulo) = girar_hacia(self.arriba, n, self.derecha());
        let paso = angulo * respuesta(RESPUESTA_ALINEACION, dt);
        self.arriba = self.arriba.girar(eje, paso);
        self.adelante = self.adelante.girar(eje, paso);
        self.adelante = self.adelante.girar(self.arriba, self.giro.y * dt);
        self.orientar();

        let velocidad_radial = self.vel.dot(n);
        let tangente = self.vel - n * velocidad_radial;
        let rapidez = tangente.largo();
        let tangente = if rapidez > 1e-4 {
            tangente.normalizado() * rapidez
        } else {
            tangente
        };

        let frente = (self.adelante - n * self.adelante.dot(n)).normalizado();
        let lado = frente.cross(n);
        let fuerza = EMPUJE_LOCAL * if e.turbo { 2.0 } else { 1.0 };
        let mut tangente = tangente + (frente * e.avance + lado * e.lateral) * (fuerza * dt);
        let resistencia = if e.freno { RESISTENCIA_FRENO * 2.0 } else { RESISTENCIA_LOCAL };
        tangente = tangente * (-resistencia * dt).exp();

        self.altura_objetivo = (self.altura_objetivo + e.vertical * VELOCIDAD_ALTURA * dt).max(MARGEN_COLISION + 0.3);
        let deseada = (self.altura_objetivo - altura) * 2.0 + e.vertical * VELOCIDAD_ALTURA;
        let radial_nueva = velocidad_radial + (deseada - velocidad_radial) * respuesta(RESPUESTA_ALTURA, dt);
        self.vel = tangente + n * radial_nueva;
    }

    fn paso_anillo(&mut self, e: &Entrada, i: usize, tiempo: f32, dt: f32) {
        let def = &PLANETAS[i];
        let n = def.normal_anillo();
        let (h, r, hacia_afuera) = def.coordenadas_anillo(self.pos, tiempo);

        let radial = self.vel.dot(hacia_afuera);
        let adentro = ANILLOS[0].0 * def.radio;
        let afuera = ANILLOS[ANILLOS.len() - 1].1 * def.radio;
        if self.borde_volteo == 0.0 {
            let borde = if r > afuera + 0.6 && radial > 0.0 {
                1.0
            } else if r < adentro - 0.6 && radial < 0.0 {
                -1.0
            } else {
                0.0
            };
            if borde != 0.0 {
                self.lado_anillo = -self.lado_anillo;
                self.borde_volteo = borde;
                self.rapidez_volteo = radial.abs().max(3.0);
                self.vel = self.vel - hacia_afuera * radial;
            }
        } else if h * self.lado_anillo > self.altura_objetivo * 0.6 {
            let radial = self.vel.dot(hacia_afuera);
            self.vel = self.vel + hacia_afuera * (-self.borde_volteo * self.rapidez_volteo - radial);
            self.borde_volteo = 0.0;
        }

        let arriba_deseado = n * self.lado_anillo;
        let borde = hacia_afuera.cross(n).normalizado();
        let (eje, angulo) = girar_hacia(self.arriba, arriba_deseado, borde);
        let paso = angulo * respuesta(RESPUESTA_ALINEACION * 1.6, dt);
        self.arriba = self.arriba.girar(eje, paso);
        self.adelante = self.adelante.girar(eje, paso);

        self.giro = self
            .giro
            .mezclar(Vec3::new(0.0, e.guinada * VELOCIDAD_GIRO, 0.0), respuesta(RESPUESTA_GIRO, dt));
        self.adelante = self.adelante.girar(self.arriba, self.giro.y * dt);
        self.orientar();

        let mut plano = self.vel - n * self.vel.dot(n);
        let pista = n.cross(hacia_afuera);
        let curva = plano.dot(pista) / r.max(1.0) * dt;
        plano = plano.girar(n, curva);
        self.adelante = self.adelante.girar(n, curva);
        self.orientar();

        let frente = (self.adelante - n * self.adelante.dot(n)).normalizado();
        let derecha = frente.cross(arriba_deseado);
        let fuerza = EMPUJE_LOCAL * if e.turbo { 2.0 } else { 1.0 };
        plano = plano + (frente * e.avance + derecha * e.lateral) * (fuerza * dt);
        let resistencia = if e.freno { RESISTENCIA_FRENO * 2.0 } else { RESISTENCIA_LOCAL };
        plano = plano * (-resistencia * dt).exp();

        self.altura_objetivo = (self.altura_objetivo + e.vertical * VELOCIDAD_ALTURA * dt).max(ALTURA_MINIMA_ANILLO);
        let vertical = self.vel.dot(n);
        let deseada = (self.altura_objetivo * self.lado_anillo - h) * 3.0;
        let vertical = vertical + (deseada - vertical) * respuesta(RESPUESTA_ALTURA, dt);
        if self.borde_volteo != 0.0 {
            plano = plano - hacia_afuera * plano.dot(hacia_afuera);
        }
        self.vel = plano + n * vertical;

        if self.altura_objetivo > SALIDA_ANILLO {
            self.modo = Modo::Vuelo;
        }
    }

    fn resolver_colision(&mut self, centro: Vec3, radio_superficie: f32) {
        let r = self.pos - centro;
        let d = r.largo();
        let minimo = radio_superficie + MARGEN_COLISION;
        if d < minimo {
            let n = r * (1.0 / d);
            self.pos = centro + n * minimo;
            let hacia_adentro = self.vel.dot(n);
            if hacia_adentro < 0.0 {
                self.vel = self.vel - n * hacia_adentro;
            }
        }
    }

    pub fn paso(&mut self, e: &Entrada, ruido: &Perlin, tiempo: f32, dt: f32) {
        let mut cercano = None;
        let siguiente = tiempo + dt;
        for (i, def) in PLANETAS.iter().enumerate() {
            let centro = def.posicion(tiempo);
            if (self.pos - centro).largo() < def.radio * ZONA_SALIDA {
                self.pos = self.pos + (def.posicion(siguiente) - centro);
                cercano = Some(i);
            }
        }

        match cercano {
            None => {
                self.modo = Modo::Vuelo;
                self.local_anulado = false;
            }
            Some(i) => {
                let def = &PLANETAS[i];
                let dentro = (self.pos - def.posicion(siguiente)).largo() < def.radio * ZONA_ENTRADA;
                if self.modo == Modo::Vuelo && !self.local_anulado && dentro {
                    self.entrar_local(i, ruido, siguiente);
                }
            }
        }

        match self.modo {
            Modo::Vuelo => self.paso_vuelo(e, dt),
            Modo::Local(i) => self.paso_local(e, i, ruido, siguiente, dt),
            Modo::Anillo(i) => self.paso_anillo(e, i, siguiente, dt),
        }
        self.pos = self.pos + self.vel * dt;
        let objetivo_empuje = if e.avance.abs() > 0.0 || e.lateral.abs() > 0.0 {
            if e.turbo { 1.0 } else { 0.7 }
        } else {
            0.12
        };
        self.empuje += (objetivo_empuje - self.empuje) * respuesta(4.0, dt);

        self.resolver_colision(Vec3::CERO, RADIO_SOL);
        for luna in LUNAS {
            let centro = luna.posicion(siguiente);
            let r = self.pos - centro;
            if r.largo() < luna.radio * 3.0 {
                let local = luna.modelo(siguiente).direccion_inversa(r.normalizado());
                self.resolver_colision(centro, luna.radio_en(ruido, local));
            }
        }
        for def in PLANETAS {
            let centro = def.posicion(siguiente);
            let r = self.pos - centro;
            if r.largo() < def.radio * ZONA_SALIDA {
                let local = def.rotacion(siguiente).direccion_inversa(r.normalizado());
                self.resolver_colision(centro, def.radio_en(ruido, local));
            }
        }
    }
}

pub struct Simulacion {
    pub tiempo: f32,
    acumulado: f32,
    pub nave: Nave,
}

impl Simulacion {
    pub fn new(tiempo: f32) -> Simulacion {
        Simulacion {
            tiempo,
            acumulado: 0.0,
            nave: Nave::new(),
        }
    }

    pub fn avanzar(&mut self, dt: f32, e: &Entrada, ruido: &Perlin) {
        self.acumulado += dt.min(0.25);
        while self.acumulado >= PASO_SIMULACION {
            self.nave.paso(e, ruido, self.tiempo, PASO_SIMULACION);
            self.tiempo += PASO_SIMULACION;
            self.acumulado -= PASO_SIMULACION;
        }
    }
}

pub struct Persecucion {
    relativa: Vec3,
    arriba: Vec3,
    azimut: f32,
    elevacion: f32,
    local: f32,
}

impl Persecucion {
    pub fn new(nave: &Nave) -> Persecucion {
        let mut p = Persecucion {
            relativa: Vec3::CERO,
            arriba: nave.arriba,
            azimut: 0.0,
            elevacion: 0.0,
            local: if nave.modo == Modo::Vuelo { 0.0 } else { 1.0 },
        };
        p.relativa = p.deseada(nave);
        p
    }

    fn deseada(&self, nave: &Nave) -> Vec3 {
        let turbo = 1.0 + (nave.empuje - 0.7).max(0.0);
        let distancia = DISTANCIA_CAMARA + (DISTANCIA_CAMARA_LOCAL - DISTANCIA_CAMARA) * self.local;
        let altura = ALTURA_CAMARA + (ALTURA_CAMARA_LOCAL - ALTURA_CAMARA) * self.local;
        let d = nave.adelante * (-distancia * turbo) + nave.arriba * altura;
        d.girar(nave.arriba, self.azimut).girar(nave.derecha(), self.elevacion)
    }

    pub fn actualizar(&mut self, nave: &Nave, dt: f32, arrastre: Option<(f32, f32)>) {
        match arrastre {
            Some((dx, dy)) => {
                self.azimut -= dx * 0.008;
                self.elevacion = (self.elevacion - dy * 0.006).clamp(-1.1, 1.1);
            }
            None => {
                let k = (-1.8 * dt).exp();
                self.azimut *= k;
                self.elevacion *= k;
            }
        }
        let meta_local = if nave.modo == Modo::Vuelo { 0.0 } else { 1.0 };
        self.local += (meta_local - self.local) * respuesta(2.0, dt);
        let t = respuesta(RESPUESTA_CAMARA, dt);
        self.relativa = self.relativa.mezclar(self.deseada(nave), t);
        let (eje, angulo) = girar_hacia(self.arriba, nave.arriba, nave.derecha());
        self.arriba = self.arriba.girar(eje, angulo * t).normalizado();
    }

    pub fn camara(&self, nave: &Nave) -> Camara {
        Camara {
            pos: nave.pos + self.relativa,
            objetivo: nave.pos
                + nave.adelante * (1.6 + (MIRADA_ADELANTE_LOCAL - 1.6) * self.local)
                + nave.arriba * (0.5 - (0.5 + MIRADA_ABAJO_LOCAL) * self.local),
            arriba: self.arriba,
            fov: FOV_GRADOS.to_radians(),
            cerca: PLANO_CERCA,
            lejos: PLANO_LEJOS,
        }
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    fn volar(fps: f32, segundos: f32, e: Entrada) -> Vec3 {
        let ruido = Perlin::new(SEMILLA);
        let mut s = Simulacion::new(0.0);
        for _ in 0..(segundos * fps).round() as usize {
            s.avanzar(1.0 / fps, &e, &ruido);
        }
        s.nave.pos
    }

    #[test]
    fn misma_trayectoria_con_distintos_fps() {
        let e = Entrada {
            avance: 1.0,
            guinada: 0.5,
            ..Entrada::default()
        };
        let a = volar(30.0, 4.0, e);
        let b = volar(144.0, 4.0, e);
        assert!((a - b).largo() < 0.5, "diferencia {}", (a - b).largo());
    }

    #[test]
    fn la_nave_entra_en_modo_local_rodea_y_sale() {
        let ruido = Perlin::new(SEMILLA);
        for (i, def) in PLANETAS.iter().enumerate() {
            let mut s = Simulacion::new(0.0);
            let centro = def.posicion(0.0);
            let polo = def.normal_anillo();
            let lado = polo.cross(Vec3::new(0.0, 0.0, 1.0)).normalizado();
            s.nave.pos = centro + (polo * 4.0 + lado * 0.6) * def.radio;
            s.nave.adelante = (centro - s.nave.pos).normalizado();
            s.nave.arriba = lado.cross(s.nave.adelante).normalizado();

            let avanzar = Entrada { avance: 1.0, ..Entrada::default() };
            let mut entro = false;
            let mut minimo = f32::MAX;
            for _ in 0..120 * 40 {
                s.avanzar(PASO_SIMULACION, &avanzar, &ruido);
                let r = s.nave.pos - def.posicion(s.tiempo);
                let local = def.rotacion(s.tiempo).direccion_inversa(r.normalizado());
                minimo = minimo.min(r.largo() - def.radio_en(&ruido, local));
                entro |= s.nave.modo == Modo::Local(i);
            }
            assert!(entro, "{}: nunca entro en modo local", def.nombre);
            assert!(minimo > 0.0, "{}: atraveso la superficie: {minimo}", def.nombre);
            assert_eq!(s.nave.modo, Modo::Local(i), "{}: se salio solo", def.nombre);

            let subir = Entrada { vertical: 1.0, ..Entrada::default() };
            for _ in 0..120 * 12 {
                s.avanzar(PASO_SIMULACION, &subir, &ruido);
            }
            assert_eq!(s.nave.modo, Modo::Vuelo, "{}: no pudo alejarse", def.nombre);
        }
    }

    #[test]
    fn la_nave_no_atraviesa_la_luna() {
        let ruido = Perlin::new(SEMILLA);
        let luna = &LUNAS[0];
        let mut s = Simulacion::new(0.0);
        s.nave.pos = luna.posicion(0.0) + Vec3::new(0.0, 0.0, luna.radio * 2.5);
        s.nave.adelante = Vec3::new(0.0, 0.0, -1.0);
        s.nave.local_anulado = true;
        let avanzar = Entrada { avance: 1.0, ..Entrada::default() };
        for _ in 0..120 * 3 {
            s.avanzar(PASO_SIMULACION, &avanzar, &ruido);
            let r = s.nave.pos - luna.posicion(s.tiempo);
            let local = luna.modelo(s.tiempo).direccion_inversa(r.normalizado());
            assert!(r.largo() >= luna.radio_en(&ruido, local) + MARGEN_COLISION - 0.05);
        }
    }

    #[test]
    fn la_nave_recorre_el_anillo_pasa_al_otro_lado_y_sale() {
        let ruido = Perlin::new(SEMILLA);
        let (i, def) = PLANETAS.iter().enumerate().find(|(_, d)| d.anillos).unwrap();
        let mut s = Simulacion::new(0.0);
        let n = def.normal_anillo();
        let afuera = (Vec3::new(1.0, 0.0, 0.0) - n * n.x).normalizado();
        let medio = (ANILLOS[0].0 + ANILLOS[ANILLOS.len() - 1].1) * 0.5 * def.radio;
        s.nave.pos = def.posicion(0.0) + afuera * medio + n * 1.5;
        s.nave.arriba = n;
        s.nave.adelante = n.cross(afuera);

        let quieto = Entrada::default();
        for _ in 0..120 {
            s.avanzar(PASO_SIMULACION, &quieto, &ruido);
        }
        assert_ne!(s.nave.modo, Modo::Anillo(i), "entro al anillo sin pulsar la tecla");
        assert_eq!(s.nave.anillo_cercano(s.tiempo), Some(i));
        s.nave.alternar_anillo(s.tiempo);
        assert_eq!(s.nave.modo, Modo::Anillo(i));

        let avanzar = Entrada { avance: 1.0, ..Entrada::default() };
        for _ in 0..120 * 20 {
            s.avanzar(PASO_SIMULACION, &avanzar, &ruido);
            let (h, r, _) = def.coordenadas_anillo(s.nave.pos, s.tiempo);
            assert_eq!(s.nave.modo, Modo::Anillo(i));
            assert!(h > 0.0 && def.sobre_anillo(r, 0.8), "h {h} r {r}");
        }

        let afuera_entrada = Entrada { lateral: 1.0, ..Entrada::default() };
        let afuera_del_anillo = ANILLOS[ANILLOS.len() - 1].1 * def.radio;
        for paso in 0..120 * 12 {
            let e = if s.nave.lado_anillo > 0.0 && paso < 120 * 6 { afuera_entrada } else { Entrada::default() };
            s.avanzar(PASO_SIMULACION, &e, &ruido);
            let (h, r, _) = def.coordenadas_anillo(s.nave.pos, s.tiempo);
            assert!(h.abs() > 0.15 || r > afuera_del_anillo, "atraveso la banda: h {h} r {r}");
        }
        let (h, r, _) = def.coordenadas_anillo(s.nave.pos, s.tiempo);
        assert_eq!(s.nave.lado_anillo, -1.0, "no paso al otro lado");
        assert!(h < 0.0 && def.sobre_anillo(r, 1.0), "h {h} r {r}");
        assert!(s.nave.arriba.dot(n) < -0.9, "la nave no se volteo");

        let subir = Entrada { vertical: 1.0, ..Entrada::default() };
        for _ in 0..120 * 3 {
            s.avanzar(PASO_SIMULACION, &subir, &ruido);
        }
        assert_ne!(s.nave.modo, Modo::Anillo(i), "no pudo salir del anillo");

        s.nave.alternar_anillo(s.tiempo);
        s.nave.alternar_anillo(s.tiempo);
        assert_ne!(s.nave.modo, Modo::Anillo(i), "la tecla no suelta el anillo");
    }

    #[test]
    fn el_planeta_orbita_y_rota_segun_el_tiempo() {
        let def = &PLANETAS[0];
        assert!((def.posicion(5.0) - def.posicion(0.0)).largo() > 1.0);
        let p = Vec3::new(1.0, 0.0, 0.0);
        assert!((def.rotacion(5.0).direccion(p) - def.rotacion(0.0).direccion(p)).largo() > 0.1);
    }
}
