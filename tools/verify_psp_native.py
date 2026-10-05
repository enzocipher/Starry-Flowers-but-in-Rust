"""Check prepared pixels, tile geometry and the normal PSP package against source art."""
import io
import json
import pathlib
import struct
from PIL import Image
import pycdlib

root = pathlib.Path(__file__).resolve().parents[1]
dist = root / "psp/dist"
manifest = json.loads((dist / "DATA/MANIFEST.JSON").read_text(encoding="utf-8"))
story = json.loads((root / "story.json").read_text(encoding="utf-8"))
sources = dict(story["images"])
sources.update({"ui " + n: "gui/" + n + ".png" for n in ["main_menu", "game_menu", "textbox", "nvl"]})
checked = set()
for name, info in manifest["images"].items():
    original = Image.open(root / "assets" / sources[name]).convert("RGBA")
    original = original.resize((info["raster_width"], info["raster_height"]), Image.Resampling.LANCZOS)
    for tile in info["tiles"]:
        if tile["file"] in checked:
            continue
        tw, th = tile["texture_width"], tile["texture_height"]
        data = (dist / "DATA" / tile["file"]).read_bytes()
        assert len(data) == tw * th * 4 * 21 // 16
        pixels = Image.frombytes("RGBA", (tw, th), data[:tw * th * 4])
        x, y, w, h = (tile[k] for k in ["x", "y", "width", "height"])
        assert pixels.crop((4, 4, 4 + w, 4 + h)).tobytes() == original.crop((x, y, x + w, y + h)).tobytes()
        assert x + w <= original.width and y + h <= original.height
        checked.add(tile["file"])

iso = pycdlib.PyCdlib()
iso.open(str(dist / "StarryFlowers.iso"))
def read_iso(path):
    with iso.open_file_from_iso(iso_path=path + ";1") as stream:
        return stream.read()
prx = (root / "psp/target/mipsel-sony-psp/release/starryflowers-psp.prx").read_bytes()
assert read_iso("/PSP_GAME/SYSDIR/EBOOT.BIN") == prx
try:
    read_iso("/PSP_GAME/USRDIR/DATA/AUDIT")
except pycdlib.pycdlibexception.PyCdlibInvalidInput:
    pass
else:
    raise AssertionError("Normal ISO contains the audit marker")
for name, size in [("ICON0.PNG", (144, 80)), ("PIC1.PNG", (480, 272))]:
    assert Image.open(io.BytesIO(read_iso("/PSP_GAME/" + name))).size == size
iso.close()
packed = (dist / "EBOOT.PBP").read_bytes()
offsets = list(struct.unpack_from("<8I", packed, 8)) + [len(packed)]
assert packed[offsets[6]:offsets[7]] == prx
for index, size in [(1, (144, 80)), (4, (480, 272))]:
    assert Image.open(io.BytesIO(packed[offsets[index]:offsets[index + 1]])).size == size
print(f"Verified {len(checked)} native RGBA tiles, tile offsets, ISO/PBP executables and launcher art, and normal ISO.")
