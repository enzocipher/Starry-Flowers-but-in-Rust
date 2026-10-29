# Revisión y optimización de PC

El repositorio conserva las mejoras locales de motor, texto y preferencias existentes al separar PSP el 29 de octubre de 2026. Incluye líneas de layout compartidas mediante `Arc`, identidad de sprites sin cadenas temporales, un índice reconstruible para texto leído, validación de posiciones de guardado y normalización de preferencias. Las mediciones y sus límites se conservan en [DESKTOP_OPTIMIZATION_BENCHMARK.md](DESKTOP_OPTIMIZATION_BENCHMARK.md).

La biblioteca permite ejecutar `cargo test --lib --locked --no-default-features` sin recursos originales. Las pruebas completas locales usan `cargo test --locked` con la carpeta `assets/`.

El port PSP, su SDK, sus herramientas y su revisión se trasladaron a [Starry-Flowers-PSP](https://github.com/enzocipher/Starry-Flowers-PSP). El código de PC no depende del repositorio PSP.
