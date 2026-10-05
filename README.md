# Starry Flowers — port local a Rust para Windows

Adaptación del juego Starry Flowers 1.7.4 de NomnomNami. Ejecuta la historia con un motor nativo en Rust y Macroquad: no utiliza Python ni Ren'Py en tiempo de ejecución. Los textos, ilustraciones, música y traducciones proceden de la copia original de esta carpeta.

"Es el mismo juego, pero una versión inferior"

## Jugar

Abre `dist/StarryFlowersRust/StarryFlowersRust.exe`. Mantén la carpeta `assets` junto al ejecutable. La ventana se llama **Starry Flowers** y usa el icono original. El idioma inicial es inglés; las traducciones originales están disponibles en Options → Language… con sus fuentes correspondientes.

- Clic, Espacio o Enter: completar la aparición del texto o avanzar.
- Ctrl: saltar texto leído; Tab o Skip: activar/desactivar salto. Options permite incluir texto no leído, continuar después de elecciones y omitir transiciones durante el salto.
- Flecha izquierda, Retroceso, Back o rueda hacia arriba: volver al diálogo anterior (hasta 250 entradas). Rollback Side permite hacerlo pulsando el lateral izquierdo o derecho.
- Escape o clic derecho: opciones; F11: pantalla completa; H o Hide Textbox: ocultar texto.
- Velocidad de texto (60 caracteres/s de inicio), tiempo de avance automático (15 de inicio), volumen de música y sonidos por separado y Mute All.
- Seis ranuras de guardado, historial, accesorios y favoritos, disponibles desde Menu.
- Extras, capítulos adicionales, galería completa y arte conceptual al terminar la historia.
- Guardados propios en `%LOCALAPPDATA%/StarryFlowersRust`. Los guardados de Ren'Py no son compatibles.

## Compilar

```powershell
cargo test
cargo build --release
./tools/package.ps1
```

Para reconstruir el contenido desde el juego que está junto a este proyecto:

```powershell
python tools/extract.py
Copy-Item -Recurse -Force ../StarryFlowers-1.7.4-pc/game/tl assets/
python tools/prepare_inline.py
python tools/compile_story.py
```

`story.json` se integra en el ejecutable al compilar. Python se usa únicamente para preparar recursos. `Cargo.lock` fija las dependencias. La implementación gráfica usa [Macroquad](https://docs.rs/macroquad/0.4.16/macroquad/).

## Alcance y diferencias

Incluye los siete capítulos, epílogo y ocho escenas extra. Conserva el diálogo y las decisiones originales; elegir accesorios no cambia la historia. Los personajes por capas, fondos, CG, audio, favoritos y desbloqueos se gestionan en Rust.

En Windows, el menú principal utiliza siempre el cielo estrellado. Las opciones y el diálogo conservan los recursos originales; los controles usan las barras de flores, muestran sus valores y ajustan los rótulos largos para evitar superposiciones. Los corazones y otros símbolos de texto se extraen de las fuentes Twemoji y DejaVu incluidas en el juego, con sus gráficos, colores y medidas originales. Se conservan las transiciones suaves entre expresiones, y los `vpunch`/`hpunch` reproducen los desplazamientos y la duración de Ren'Py (0,275 s); el temblor se activa en los diálogos originales, sin deducirlo por el rubor del personaje.

Todavía hay diferencias: algunas transiciones de escena, partículas y movimientos son aproximaciones; el formato de texto se muestra sin cursivas ni efectos tipográficos avanzados. El historial guarda las últimas 250 entradas. La galería permite consultar las ilustraciones y escuchar la banda sonora. No se presenta como una reproducción píxel por píxel.

Los ajustes de la primera versión se actualizan al abrir esta versión: el idioma inicial pasa a inglés, conservando partidas, favoritos y desbloqueos. Las elecciones de idioma posteriores se guardan normalmente.

Los nombres usan la fuente original Nunito Bold (o la correspondiente al idioma), relleno blanco y los contornos de cada personaje: rosa para Pastille, celeste para Periwinkle y las paletas originales para los demás. El diálogo se ajusta por las medidas de la fuente completa, con saltos de línea estables durante la aparición gradual. La caja crece cuando hace falta; la narración y el historial ajustan el tamaño para respetar sus áreas y dejar libres los controles.

## Verificación

La auditoría `--audit-layout` comprobó 5.038 diálogos y páginas de narración en inglés y español con las fuentes originales, sin desbordamiento horizontal ni vertical. Las capturas `--smoke=long-text`, `--smoke=narration` y `--smoke=history` cubren casos de texto largo.

Las 14 pruebas recorren ambas elecciones iniciales hasta el final, las ocho escenas extra, la traducción española, la serialización, los recursos, la conservación de los shakes y su curva temporal, los valores de ajustes, la migración y el texto gradual Unicode. `--smoke=dialogue`, `--smoke=portrait`, `--smoke=dress`, `--smoke=title`, `--smoke=gallery`, `--smoke=credits`, `--smoke=settings`, `--smoke=language` y `--smoke=shake` exportan capturas y cierran la ventana. Esto no sustituye una revisión manual completa de todas las escenas.

Autoría original: historia, arte y música por **NomnomNami**; tema final **“Pretty in Pink”**, por **Marlene Bellissimo**. Se conserva la copia original y sus archivos de atribución. Este paquete local no cambia la autoría ni la licencia de los recursos originales.

El texto leído se registra al terminar de aparecer y se guarda inmediatamente. Saltar se detiene al llegar a texto no leído (salvo la opción de saltarlo también); Auto avanza tras mostrar el diálogo y esperar el tiempo configurado. El modo activo aparece subrayado. Avanzar manualmente desactiva ambos modos.

Gráficos Twemoji: © Twitter y colaboradores, licencia CC-BY 4.0; las atribuciones originales se incluyen en `assets/gui/emoji/`.

About incluye los créditos originales de historia, arte, música, las 13 traducciones, pruebas, agradecimientos y mecenas, repartidos en páginas. Los nombres de las listas se transcribieron de `names1.png`, `names2.png` y `names3.png`; la licencia original de Ren'Py acompaña los recursos.
