"""Convert the user's original assets to bounded PSP textures and PCM streams."""
import hashlib
import json
import math
import pathlib
import subprocess
import struct
from PIL import Image, ImageDraw, ImageFont
import imageio_ffmpeg

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / "psp" / "dist" / "DATA"
OUT.mkdir(parents=True, exist_ok=True)
story = json.loads((ROOT / "story.json").read_text(encoding="utf-8"))
story["translations"] = {k: v for k, v in story["translations"].items() if k == "es"}
(ROOT / "psp" / "story.json").write_text(json.dumps(story, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
images = dict(story["images"])
images.update({"ui " + n: "gui/" + n + ".png" for n in ["main_menu", "game_menu", "textbox", "nvl"]})
manifest = {"images": {}, "fonts": {}, "audio": {}}
files = {}
for name, path in images.items():
    if path not in files:
        image = Image.open(ROOT / "assets" / path).convert("RGBA")
        ow, oh = image.size
        image = image.resize((max(1, round(ow * .375)), max(1, round(oh * .375))), Image.Resampling.LANCZOS)
        filename = "I" + hashlib.sha256(path.encode()).hexdigest()[:12].upper() + ".RAW"
        (OUT / filename).write_bytes(image.tobytes())
        files[path] = {"file": filename, "width": image.width, "height": image.height, "original_width": ow, "original_height": oh}
    manifest["images"][name] = files[path]
chars = set("".join(op.get("text", "") for op in story["ops"]))
chars.update("".join(story["translations"].get("es", {}).values()))
chars.update("Starry Flowers Start Continue Extras Gallery Settings Save Load Return English Español Back Skip Auto Text speed Music Sound Unseen text After choices Transitions Rollback Slot Empty Accessories Favorite Done Periwinkle Pastille Astragalus Cassia Jam Kardaemon Amaretti Gumdrop Nasty Witch Himbo Witch 0123456789:<>+-./")
chars = sorted(c for c in chars if c.isprintable() and c != "💙")
for size in [10, 12, 14, 18, 24]:
    font = ImageFont.truetype(str(ROOT / "assets/tl/None/Nunito-Bold.ttf"), size)
    cell = size * 2
    columns = 512 // cell
    atlas = Image.new("L", (512, math.ceil(len(chars) / columns) * cell))
    draw = ImageDraw.Draw(atlas)
    glyphs = {}
    for i, c in enumerate(chars):
        x, y = (i % columns) * cell, (i // columns) * cell
        draw.text((x + 1, y), c, font=font, fill=255)
        glyphs[c] = {"x": x, "y": y, "width": cell, "height": cell, "advance": font.getlength(c)}
    filename = f"FONT{size}.RAW"
    (OUT / filename).write_bytes(atlas.tobytes())
    manifest["fonts"][str(size)] = {"file": filename, "width": atlas.width, "height": atlas.height, "glyphs": glyphs}
ffmpeg = imageio_ffmpeg.get_ffmpeg_exe()
for path in sorted((ROOT / "assets/audio").iterdir()):
    if path.suffix.lower() not in {".ogg", ".wav"}: continue
    filename = "A" + hashlib.sha256(path.stem.encode()).hexdigest()[:12].upper() + ".PCM"
    dest = OUT / filename
    if not dest.exists():
        subprocess.run([ffmpeg, "-v", "error", "-y", "-i", str(path), "-f", "s16le", "-ac", "1", "-ar", "44100", str(dest)], check=True)
    manifest["audio"][path.stem] = filename
(OUT / "MANIFEST.JSON").write_text(json.dumps(manifest, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
print(f"PSP assets: {len(files)} textures, {len(manifest['audio'])} audio streams, {sum(p.stat().st_size for p in OUT.iterdir()) / 1024**2:.1f} MiB")
