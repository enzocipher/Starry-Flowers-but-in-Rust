# Benchmark de optimización de escritorio

Ejecutado en Windows 11 10.0.26200, Intel Core i7-11800H y 16 GiB, con perfil `release` de Cargo. Repetir con:

```powershell
cargo test --release desktop_optimization_baseline -- --ignored --nocapture --test-threads=1
```

El helper recorre la ruta principal y reúne 2.255 diálogos. Las comparaciones usan los mismos datos y el mismo proceso; el camino anterior se mide junto al actual. Los tiempos son de microbenchmarks locales, no predicen FPS.

| Ruta | Anterior | Actual |
|---|---:|---:|
| 10.000 consultas de diálogo, mitad aciertos y mitad fallos | `Vec::contains`: 2,403 ms, 0 asignaciones | índice `HashSet`: 0,133 ms, 0 asignaciones |
| 100.000 comprobaciones de identidad de sprite estable | 300.000 asignaciones, 16,574 ms | 0 asignaciones, 1,895 ms |
| Copia del resultado de caché, 323 líneas × 10.000 aciertos | 3.240.000 asignaciones, 141,753 ms | `Arc` compartido: 0 asignaciones, 0,098 ms |

El ajuste en frío de 80 diálogos largos tomó 0,196 ms y 973 asignaciones con una métrica sintética de ancho por carácter. No usa las fuentes del juego ni reproduce kerning. Se conservó el medidor actual para mantener los cortes de línea exactos; no se atribuye una mejora a esa ruta.

La cifra de caché mide el coste de devolver las líneas almacenadas y compartirlas. La construcción de la clave y las mediciones de fuente no forman parte de ese microbenchmark. El índice `read` es transitorio: el JSON conserva el campo vectorial original, y las pruebas comprueban que el índice se reconstruye tras deserializar.

## Perfil de ejecución

Compilar y ejecutar las escenas smoke medidas:

```powershell
cargo build --release
& .\target\release\starryflowers-rust.exe --perf-smoke=settings-es
& .\target\release\starryflowers-rust.exe --perf-smoke=long-text
```

Cada ejecución dibuja 420 frames: 60 de calentamiento y 360 muestras. El JSON completo, incluidos los 360 tiempos individuales, se escribe fuera del repositorio en `%TEMP%\perf-smoke-settings-es.json` o `%TEMP%\perf-smoke-long-text.json`. Los contadores de caché se reinician al terminar el calentamiento.

| Escena | Media | p95 | p99 | Máximo | Aciertos/fallos de caché |
|---|---:|---:|---:|---:|---:|
| `settings-es` | 6,946 ms | 8,210 ms | 8,886 ms | 9,148 ms | 0 / 0 |
| `long-text` | 6,944 ms | 8,261 ms | 9,182 ms | 9,792 ms | 360 / 0 |

Estas cifras describen la ejecución observada y no una mejora. Macroquad `get_frame_time()` mide el intervalo entre frames e incluye la espera de presentación y VSync; no es una medición aislada del tiempo de CPU. El uso de CPU y la memoria máxima se registraron aparte a nivel de proceso.
