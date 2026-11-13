use crate::vector::Vec3;

pub struct Azar {
    estado: u64,
}

impl Azar {
    pub fn new(semilla: u64) -> Azar {
        Azar { estado: semilla }
    }

    pub fn siguiente(&mut self) -> u64 {
        self.estado = self.estado.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.estado;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn unidad(&mut self) -> f32 {
        (self.siguiente() >> 40) as f32 / (1u64 << 24) as f32
    }
}

pub struct Perlin {
    p: [u8; 512],
}

const GRADIENTES: [Vec3; 12] = [
    Vec3::new(1.0, 1.0, 0.0),
    Vec3::new(-1.0, 1.0, 0.0),
    Vec3::new(1.0, -1.0, 0.0),
    Vec3::new(-1.0, -1.0, 0.0),
    Vec3::new(1.0, 0.0, 1.0),
    Vec3::new(-1.0, 0.0, 1.0),
    Vec3::new(1.0, 0.0, -1.0),
    Vec3::new(-1.0, 0.0, -1.0),
    Vec3::new(0.0, 1.0, 1.0),
    Vec3::new(0.0, -1.0, 1.0),
    Vec3::new(0.0, 1.0, -1.0),
    Vec3::new(0.0, -1.0, -1.0),
];

fn curva(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

impl Perlin {
    pub fn new(semilla: u64) -> Perlin {
        let mut azar = Azar::new(semilla);
        let mut base: [u8; 256] = std::array::from_fn(|i| i as u8);
        for i in (1..256).rev() {
            let j = (azar.siguiente() % (i as u64 + 1)) as usize;
            base.swap(i, j);
        }
        let mut p = [0u8; 512];
        for i in 0..512 {
            p[i] = base[i & 255];
        }
        Perlin { p }
    }

    fn hash(&self, x: i32, y: i32, z: i32) -> u8 {
        let a = self.p[(x & 255) as usize] as usize;
        let b = self.p[a + (y & 255) as usize] as usize;
        self.p[b + (z & 255) as usize]
    }

    pub fn ruido(&self, v: Vec3) -> f32 {
        let (xi, yi, zi) = (v.x.floor(), v.y.floor(), v.z.floor());
        let (x, y, z) = (v.x - xi, v.y - yi, v.z - zi);
        let (xi, yi, zi) = (xi as i32, yi as i32, zi as i32);
        let (u, w, t) = (curva(x), curva(y), curva(z));
        let g = |dx: i32, dy: i32, dz: i32| {
            let h = self.hash(xi + dx, yi + dy, zi + dz) as usize % 12;
            GRADIENTES[h].dot(Vec3::new(x - dx as f32, y - dy as f32, z - dz as f32))
        };
        lerp(
            lerp(
                lerp(g(0, 0, 0), g(1, 0, 0), u),
                lerp(g(0, 1, 0), g(1, 1, 0), u),
                w,
            ),
            lerp(
                lerp(g(0, 0, 1), g(1, 0, 1), u),
                lerp(g(0, 1, 1), g(1, 1, 1), u),
                w,
            ),
            t,
        )
    }

    pub fn fbm(&self, v: Vec3, octavas: u32) -> f32 {
        let (mut suma, mut amplitud, mut total, mut p) = (0.0, 1.0, 0.0, v);
        for _ in 0..octavas {
            suma += self.ruido(p) * amplitud;
            total += amplitud;
            amplitud *= 0.5;
            p = p * 2.03 + Vec3::new(17.1, 9.2, 4.7);
        }
        suma / total
    }

    pub fn voronoi(&self, v: Vec3) -> (f32, f32, f32) {
        let (xi, yi, zi) = (v.x.floor() as i32, v.y.floor() as i32, v.z.floor() as i32);
        let (mut mejor, mut segundo, mut id) = (f32::MAX, f32::MAX, 0u8);
        for dz in -1..=1 {
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let (cx, cy, cz) = (xi + dx, yi + dy, zi + dz);
                    let h1 = self.hash(cx, cy, cz);
                    let h2 = self.p[h1 as usize];
                    let h3 = self.p[h2 as usize];
                    let punto = Vec3::new(
                        cx as f32 + h1 as f32 / 255.0,
                        cy as f32 + h2 as f32 / 255.0,
                        cz as f32 + h3 as f32 / 255.0,
                    );
                    let d = punto - v;
                    let d2 = d.dot(d);
                    if d2 < mejor {
                        segundo = mejor;
                        mejor = d2;
                        id = h3;
                    } else if d2 < segundo {
                        segundo = d2;
                    }
                }
            }
        }
        (mejor.sqrt(), segundo.sqrt(), id as f32 / 255.0)
    }
}
