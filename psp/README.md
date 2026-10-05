# Starry Flowers — PSP / Rust

Port nativo de desarrollo para PSP, en la rama `dev-psp`. La versión Windows está en `dev` y `master`.

## Ejecutar

- ISO: `psp/dist/StarryFlowers.iso`, para PPSSPP o una PSP con firmware compatible con homebrew e imágenes ISO.
- Homebrew: copiar `psp/dist/EBOOT.PBP` y la carpeta `psp/dist/DATA` a `PSP/GAME/StarryFlowers/` en la Memory Stick.
- Los guardados propios están en `PSP/SAVEDATA/STARRYFLOWERS`. No son los guardados de Ren'Py ni de Windows.

La ISO incorpora un ejecutable MIPS PSP real y los recursos convertidos de tu copia de Starry Flowers 1.7.4. El juego no requiere Python ni Ren'Py al ejecutarse.

## Controles

| Control | Acción |
|---|---|
| Cruceta | Seleccionar opciones o accesorios |
| X | Completar texto, avanzar o confirmar |
| O / L | Retroceder un diálogo; O vuelve en los menús |
| R mantenido | Saltar texto leído |
| Select | Activar/desactivar salto |
| Cuadrado | Activar/desactivar avance automático |
| Triángulo | Ocultar/mostrar diálogo |
| Start | Menú, ajustes y seis ranuras de guardado |
| Cuadrado / Triángulo en accesorios | Guardar/cargar accesorios favoritos |

Idioma inicial inglés; español seleccionable en Settings. Ajustes de velocidad de texto, avance automático, volúmenes separados y comportamiento del salto. Se incluyen ambos recorridos iniciales, los capítulos, epílogo y ocho extras, además de una galería disponible al terminar.

## Compilar en Windows

Requiere Rust nightly con `rust-src`, `cargo-psp 0.2.9`, Python con Pillow, imageio-ffmpeg y pycdlib, y los recursos originales en `../assets`. Se ha compilado con nightly de **2026-10-05**, host Windows GNU.

```powershell
rustup toolchain install nightly-x86_64-pc-windows-gnu --component rust-src --profile minimal
cargo install cargo-psp --version 0.2.9
python -m pip install Pillow imageio-ffmpeg pycdlib
# Desde la raíz del repositorio:
./tools/build_psp.ps1 -Python python
```

`prepare_psp.py` conserva la resolución original del arte en mosaicos RGBA comprimidos sin pérdida, genera atlas de la fuente Nunito a cuatro veces su tamaño y convierte el audio a PCM estéreo de 44,1 kHz. `package_psp.py` genera la ISO y comprueba que `PSP_GAME/SYSDIR/EBOOT.BIN` contenga el ejecutable ELF nativo. Los archivos generados y los recursos del juego están excluidos de Git.

## Memoria y SDK

Se reutiliza el motor narrativo de Windows al compilar, adaptando sus colecciones a `alloc::collections::BTreeMap`. El ejecutable usa `no_std`, el SDK de PSP para archivos, mandos, pantalla y audio, un área de memoria de 16 MiB y una caché de texturas con umbral de 4 MiB (hasta un mosaico adicional durante la carga).

`third_party/psp` procede de `psp 0.3.14` (licencia MIT incluida). Su asignador original creaba un objeto del kernel por cada asignación; el guion agotaba el límite de objetos al cargar. El parche utiliza `linked_list_allocator` dentro de una sola reserva. Los trabajadores de audio no asignan memoria durante la reproducción. Los guardados se escriben primero en un archivo temporal y conservan un respaldo durante el reemplazo.

## Verificación y límites

`python tools/package_psp.py --audit` genera una ISO independiente para pruebas automáticas. Esta arranca en PPSSPP, captura el menú y el primer diálogo, recorre las dos rutas y los ocho extras, comprueba guardar/cargar y sobrescribir ajustes, y registra bloques de audio reproducidos. El resultado se escribe en `PSP/SAVEDATA/STARRYFLOWERS/AUDIT.JSON`. La ISO normal no activa esta prueba.

Se prueba con el perfil PSP-1000 de 32 MiB de PPSSPP. La verificación en emulador no sustituye las pruebas en una PSP física. El renderizado utiliza la GPU de PSP, filtrado bilineal y mipmaps, transparencias y una caché de escenas en VRAM. PPSSPP puede renderizar el arte y las fuentes a su resolución interna configurada. Se conservan los archivos de arte originales, audio estéreo, inglés/español y retroceso limitado a 32 diálogos; las animaciones de escena y la presentación de créditos son simplificadas. No tiene todavía todas las opciones visuales y de idiomas de Windows.

Historia, arte y música originales: **NomnomNami**. Tema final “Pretty in Pink”: **Marlene Bellissimo**. Se conserva la autoría y licencia de los recursos originales.
