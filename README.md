# Sistema solar - renderer por software

Proyecto final de Gráficas por Computadora

**Natanael Giron:** 

**Video::** .

## Ejecutar 

Requiere Rust. Desde la carpeta local:

```
cargo run 
```

Pruebas: `cargo test --release`. Otra semilla de distribución: `cargo run --release -- --semilla 7`.

Los parámetros que se pueden cambiar (radios, órbitas, velocidades, semilla, cantidad
de objetos, intensidad de los shaders) están en `src/config.rs`.

## Controles

| Tecla | Vuelo espacial | Órbita local (cerca de un planeta) |
|---|---|---|
| W / S | acelerar / retroceder | avanzar / retroceder alrededor del planeta |
| A / D | desplazar | desplazar |
| Izquierda / Derecha | girar | girar |
| Abajo / Arriba | subir / bajar la nariz | subir / bajar de altura (también R / F) |
| Q / E | alabeo | - |
| Shift / Espacio | turbo / freno | turbo / freno |
| Arrastrar con el mouse | mirar alrededor | mirar alrededor |

Cerca de los anillos de Dulce aparece "G RECORRER EL ANILLO": `G`  lo que nos pega al anillo y se
recorre como una pista; al cruzar el borde pasas al otro lado. `G` otra vez (o subir) para salir.

`L` entra o sale de la órbita local (o suelta el anillo), `H` muestra la ayuda, `1` activa el descarte,
`2` activa los niveles de detalle y `Esc` sale.

Recorrido: volar hacia un planeta marcado, al acercarse entra solo la órbita local

## Características a evaluar

| Característica | Puntos | Dónde verla | Código |
|---|---|---|---|
| 6 planetas: Tecno (cubo redondeado), Lava, Hielo, Dulce, Roto (pedazos alrededor de un núcleo) y Navidad (árbol con pisos) | 40 | todo el sistema | `planeta.rs`, `config.rs` |
| 8 tipos de modelo propios: antena, torre, roca, cristal, hongo, pino con luces, regalo y estrella | 20 | Tecno (antena y torre), Navidad (pino, regalo y la estrella en la punta) y uno en cada otro planeta | `blender/superficie.py` |
| Distribución procedural con semilla | 10 | `--semilla` cambia las ubicaciones | `DefPlaneta::distribuir` |
| Objetos orientados sobre la superficie curva mientras rota | 5 | de cerca | `DefPlaneta::distribuir`, `Mundo::instancias` |
| 4 shaders: Sol, Tecno, Lava y Hielo (además, Dulce) | 24 | cada astro | `shaders.rs` |
| Shader animado: plasma del Sol (también grietas de Lava y pulsos de Tecno) | 8 | Sol, Lava | `shaders.rs` |
| Deformación de geometría con ruido y otras funciones (Tecno: superelipsoide) | 10 | relieve de cada planeta | `DefPlaneta::perfil` |
| Subdivisión procedural: icosfera | 10 | contador de triángulos | `Icosfera::subdividir` |
| Extra: planeta Roto: corteza partida en pedazos con capas de tierra, núcleo de energía y rocas flotando adentro | - | órbita más lejana | `DefPlaneta::malla_rota`, `shaders::roto` |
| Extra: planeta Navidad: esfera con un cono tangente y pisos, guirnalda en espiral con focos que se encienden en cadena, nieve que destella y copos cayendo | - | órbita más lejana | `forma_navidad`, `guirnalda`, `shaders::navidad`, `Mundo::instancias` |
| Extra: efectos de la nave: llamas en las 3 toberas según el empuje (azules con turbo), estela de chispas y polvo espacial que pasa rayado según la velocidad | - | al volar | `Mundo::instancias`, `efectos::Estela`, `efectos::polvo`, `shaders::llama` |
| Extra: luna de Tecno (órbita encadenada: planeta * órbita * giro) | - | junto a Tecno | `DefLuna::modelo` |
| Extra: anillos de caramelo de Dulce con la sombra del planeta | - | Dulce | `geometria::anillo`, `shaders::anillo` |
| Extra: erupciones de chispas en Lava (tiro parabólico según el tiempo) | - | Lava, cada ~9 s por volcán | `efectos::chispas` |
| Extra: recorrer los anillos de Dulce  | - | Dulce, tecla `G` | `Nave::paso_anillo` |
| Modo de control local (en Tecno la gravedad sigue la cara del cubo) | 8 | al acercarse a un planeta | `Nave::paso_local` |
| Aceleración y giro progresivos, a paso fijo | 8 | todo el vuelo | `Nave::paso_vuelo`, `Simulacion::avanzar` |
| Colisión con planetas y Sol | 4 | intentar atravesarlos | `Nave::resolver_colision` |
| Descarte de objetos fuera de vista | 7 | tecla `1` | `Renderer::es_visible` |
| Niveles de detalle según la distancia | 8 | tecla `2` | `nivel_por_distancia` |

Total : 162 puntos técnicos.

### Comparación de trabajo del renderizador

Triángulos enviados a la etapa de geometría, en una toma a 1280x720:

| Vista | Descarte y detalle | Sin descarte | Sin detalle | Sin ambos |
|---|---|---|---|---|
| Inicial | 34 962 | 52 622 | 1 441 594 | 1 460 510 |
| Sobre Tecno | 26 246 | 96 134 | 352 646 | 1 460 510 |

## Cómo funciona

`render.rs` transforma con modelo, vista y proyección, recorta contra los planos en
espacio de clip, descarta caras traseras e interpola con corrección de perspectiva,
con un z-buffer de 1/w. Cada fragmento llama a un shader de `shaders.rs`. Se usa minifb
solo para la ventana, las entradas y mostrar el framebuffer.


