use crate::geometria::{Malla, Vertice};
use crate::vector::Vec3;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

fn cargar_materiales(ruta: &Path) -> HashMap<String, Vec3> {
    let mut materiales = HashMap::new();
    let Ok(texto) = fs::read_to_string(ruta) else {
        return materiales;
    };
    let mut actual = String::new();
    for linea in texto.lines() {
        let partes: Vec<&str> = linea.split_whitespace().collect();
        match partes.as_slice() {
            ["newmtl", nombre] => actual = nombre.to_string(),
            ["Kd", r, g, b] => {
                let n = |t: &str| t.parse().unwrap_or(1.0);
                materiales.insert(actual.clone(), Vec3::new(n(r), n(g), n(b)));
            }
            _ => {}
        }
    }
    materiales
}

pub fn cargar(ruta: &str, largo_deseado: f32, material_extra: &str, apoyado: bool) -> Result<Malla, String> {
    let texto = fs::read_to_string(ruta).map_err(|e| format!("no se pudo abrir {ruta}: {e}"))?;
    let mut posiciones: Vec<Vec3> = Vec::new();
    let mut caras: Vec<(Vec<usize>, Vec3, f32)> = Vec::new();
    let mut materiales = HashMap::new();
    let mut color = Vec3::new(0.8, 0.8, 0.8);
    let mut extra = 0.0;
    for linea in texto.lines() {
        let mut partes = linea.split_whitespace();
        match partes.next() {
            Some("v") => {
                let n: Vec<f32> = partes.take(3).map(|p| p.parse().unwrap_or(0.0)).collect();
                posiciones.push(Vec3::new(n[0], n[1], n[2]));
            }
            Some("mtllib") => {
                if let (Some(nombre), Some(carpeta)) = (partes.next(), Path::new(ruta).parent()) {
                    materiales = cargar_materiales(&carpeta.join(nombre));
                }
            }
            Some("usemtl") => {
                let nombre = partes.next().unwrap_or("");
                color = materiales.get(nombre).copied().unwrap_or(color);
                extra = if !material_extra.is_empty() && nombre.starts_with(material_extra) { 1.0 } else { 0.0 };
            }
            Some("f") => {
                let indices = partes
                    .filter_map(|p| p.split('/').next()?.parse::<usize>().ok())
                    .map(|i| i - 1)
                    .collect();
                caras.push((indices, color, extra));
            }
            _ => {}
        }
    }
    if posiciones.is_empty() {
        return Err(format!("{ruta} no tiene vertices"));
    }

    let (mut minimo, mut maximo) = (posiciones[0], posiciones[0]);
    for p in &posiciones {
        minimo = Vec3::new(minimo.x.min(p.x), minimo.y.min(p.y), minimo.z.min(p.z));
        maximo = Vec3::new(maximo.x.max(p.x), maximo.y.max(p.y), maximo.z.max(p.z));
    }
    let mut centro = (minimo + maximo) * 0.5;
    if apoyado {
        centro.y = 0.0;
    }
    let tamano = maximo - minimo;
    let escala = largo_deseado / tamano.x.max(tamano.y).max(tamano.z);

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for (cara, color, extra) in &caras {
        for k in 1..cara.len().saturating_sub(1) {
            let puntos = [cara[0], cara[k], cara[k + 1]].map(|i| (posiciones[i] - centro) * escala);
            let normal = (puntos[1] - puntos[0]).cross(puntos[2] - puntos[0]).normalizado();
            let base = vertices.len() as u32;
            for pos in puntos {
                vertices.push(Vertice {
                    pos,
                    normal,
                    color: *color,
                    extra: *extra,
                });
            }
            indices.push([base, base + 1, base + 2]);
        }
    }
    Ok(Malla { vertices, indices })
}
