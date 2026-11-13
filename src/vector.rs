use std::ops::{Add, Mul, Neg, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const CERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    pub fn dot(self, o: Vec3) -> f32 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3::new(
            self.y * o.z - self.z * o.y,
            self.z * o.x - self.x * o.z,
            self.x * o.y - self.y * o.x,
        )
    }

    pub fn largo(self) -> f32 {
        self.dot(self).sqrt()
    }

    pub fn normalizado(self) -> Vec3 {
        let l = self.largo();
        if l > 1e-9 { self * (1.0 / l) } else { self }
    }

    pub fn mezclar(self, o: Vec3, t: f32) -> Vec3 {
        self + (o - self) * t
    }

    pub fn por(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x * o.x, self.y * o.y, self.z * o.z)
    }

    pub fn girar(self, eje: Vec3, angulo: f32) -> Vec3 {
        let (s, c) = angulo.sin_cos();
        self * c + eje.cross(self) * s + eje * (eje.dot(self) * (1.0 - c))
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, k: f32) -> Vec3 {
        Vec3::new(self.x * k, self.y * k, self.z * k)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn mezclar(self, o: Vec4, t: f32) -> Vec4 {
        Vec4 {
            x: self.x + (o.x - self.x) * t,
            y: self.y + (o.y - self.y) * t,
            z: self.z + (o.z - self.z) * t,
            w: self.w + (o.w - self.w) * t,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Mat4 {
    pub m: [[f32; 4]; 4],
}

impl Mat4 {
    pub fn identidad() -> Mat4 {
        Mat4 {
            m: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }
    }

    pub fn traslacion(t: Vec3) -> Mat4 {
        let mut r = Mat4::identidad();
        r.m[3][0] = t.x;
        r.m[3][1] = t.y;
        r.m[3][2] = t.z;
        r
    }

    pub fn rotacion_y(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        let mut r = Mat4::identidad();
        r.m[0][0] = c;
        r.m[0][2] = -s;
        r.m[2][0] = s;
        r.m[2][2] = c;
        r
    }

    pub fn rotacion_x(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        let mut r = Mat4::identidad();
        r.m[1][1] = c;
        r.m[1][2] = s;
        r.m[2][1] = -s;
        r.m[2][2] = c;
        r
    }

    pub fn rotacion_z(a: f32) -> Mat4 {
        let (s, c) = a.sin_cos();
        let mut r = Mat4::identidad();
        r.m[0][0] = c;
        r.m[0][1] = s;
        r.m[1][0] = -s;
        r.m[1][1] = c;
        r
    }

    pub fn desde_ejes(x: Vec3, y: Vec3, z: Vec3, origen: Vec3) -> Mat4 {
        Mat4 {
            m: [
                [x.x, x.y, x.z, 0.0],
                [y.x, y.y, y.z, 0.0],
                [z.x, z.y, z.z, 0.0],
                [origen.x, origen.y, origen.z, 1.0],
            ],
        }
    }

    pub fn perspectiva(fov_y: f32, aspecto: f32, cerca: f32, lejos: f32) -> Mat4 {
        let f = 1.0 / (fov_y * 0.5).tan();
        Mat4 {
            m: [
                [f / aspecto, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, (lejos + cerca) / (cerca - lejos), -1.0],
                [0.0, 0.0, 2.0 * lejos * cerca / (cerca - lejos), 0.0],
            ],
        }
    }

    pub fn mirar(ojo: Vec3, objetivo: Vec3, arriba: Vec3) -> Mat4 {
        let z = (ojo - objetivo).normalizado();
        let x = arriba.cross(z).normalizado();
        let y = z.cross(x);
        Mat4 {
            m: [
                [x.x, y.x, z.x, 0.0],
                [x.y, y.y, z.y, 0.0],
                [x.z, y.z, z.z, 0.0],
                [-x.dot(ojo), -y.dot(ojo), -z.dot(ojo), 1.0],
            ],
        }
    }

    pub fn por(&self, o: &Mat4) -> Mat4 {
        let mut r = [[0.0; 4]; 4];
        for c in 0..4 {
            for f in 0..4 {
                r[c][f] = self.m[0][f] * o.m[c][0]
                    + self.m[1][f] * o.m[c][1]
                    + self.m[2][f] * o.m[c][2]
                    + self.m[3][f] * o.m[c][3];
            }
        }
        Mat4 { m: r }
    }

    pub fn punto_clip(&self, p: Vec3) -> Vec4 {
        let m = &self.m;
        Vec4 {
            x: m[0][0] * p.x + m[1][0] * p.y + m[2][0] * p.z + m[3][0],
            y: m[0][1] * p.x + m[1][1] * p.y + m[2][1] * p.z + m[3][1],
            z: m[0][2] * p.x + m[1][2] * p.y + m[2][2] * p.z + m[3][2],
            w: m[0][3] * p.x + m[1][3] * p.y + m[2][3] * p.z + m[3][3],
        }
    }

    pub fn punto(&self, p: Vec3) -> Vec3 {
        let c = self.punto_clip(p);
        Vec3::new(c.x, c.y, c.z)
    }

    pub fn direccion(&self, v: Vec3) -> Vec3 {
        let m = &self.m;
        Vec3::new(
            m[0][0] * v.x + m[1][0] * v.y + m[2][0] * v.z,
            m[0][1] * v.x + m[1][1] * v.y + m[2][1] * v.z,
            m[0][2] * v.x + m[1][2] * v.y + m[2][2] * v.z,
        )
    }

    pub fn direccion_inversa(&self, v: Vec3) -> Vec3 {
        let m = &self.m;
        Vec3::new(
            m[0][0] * v.x + m[0][1] * v.y + m[0][2] * v.z,
            m[1][0] * v.x + m[1][1] * v.y + m[1][2] * v.z,
            m[2][0] * v.x + m[2][1] * v.y + m[2][2] * v.z,
        )
    }
}

pub fn suave(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
