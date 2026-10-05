"""Package the native PRX as a PSP UMD ISO and a Memory Stick homebrew directory."""
import argparse
import json
import pathlib
import shutil
import subprocess
import tempfile
import struct
from PIL import Image
import pycdlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--audit", action="store_true", help="Make a separate emulator audit image")
parser.add_argument("--performance-audit", action="store_true", help="Run the fast native renderer/navigation audit")
args = parser.parse_args()
if args.performance_audit: args.audit = True
binary = ROOT / "psp/target/mipsel-sony-psp/release"
dist = ROOT / "psp/dist"
prx = binary / "starryflowers-psp.prx"
pbp = max(binary.glob("*EBOOT.PBP"), key=lambda p: p.stat().st_mtime)
if not prx.exists() or not pbp.exists(): raise SystemExit("Build with cargo +nightly psp --release first")
# Use the original title art for the PSP launcher, without generated artwork.
def launcher_picture(size):
    bg = Image.open(ROOT / "assets/gui/main_menu.png").convert("RGBA").resize(size, Image.Resampling.LANCZOS)
    logo = Image.open(ROOT / "assets/images/titlelogo.png").convert("RGBA")
    width = round(size[0] * .92)
    logo = logo.resize((width, round(width / 2)), Image.Resampling.LANCZOS)
    bg.alpha_composite(logo, ((size[0] - logo.width) // 2, (size[1] - logo.height) // 2))
    return bg.convert("RGB")
launcher_picture((144, 80)).save(dist / "ICON0.PNG")
launcher_picture((480, 272)).save(dist / "PIC1.PNG")
with tempfile.TemporaryDirectory() as tmp:
    tmp = pathlib.Path(tmp)
    sfo = tmp / "PARAM.SFO"
    subprocess.run(["mksfo", "--bare", "-s", "TITLE=Starry Flowers", "-s", "CATEGORY=UG", "-s", "DISC_ID=SFWR00001", "-s", "DISC_VERSION=1.00", "-s", "PSP_SYSTEM_VER=1.00", "-d", "BOOTABLE=1", "-d", "PARENTAL_LEVEL=1", "Starry Flowers", str(sfo)], check=True)
    # Preserve executable/PSAR sections while replacing metadata and launcher art.
    packed = pbp.read_bytes()
    if packed[:4] != b"\x00PBP": raise RuntimeError("Invalid source EBOOT.PBP")
    offsets = list(struct.unpack_from("<8I", packed, 8)) + [len(packed)]
    sections = [packed[offsets[n]:offsets[n+1]] for n in range(8)]
    sections[0] = sfo.read_bytes()
    sections[1] = (dist / "ICON0.PNG").read_bytes()
    sections[4] = (dist / "PIC1.PNG").read_bytes()
    position = 40; new_offsets = []
    for section in sections:
        new_offsets.append(position); position += len(section)
    (dist / "EBOOT.PBP").write_bytes(packed[:8] + struct.pack("<8I", *new_offsets) + b"".join(sections))
    iso = pycdlib.PyCdlib()
    iso.new(interchange_level=3, sys_ident="PSP GAME", vol_ident="STARRY_FLOWERS", joliet=3)
    for directory in ["/PSP_GAME", "/PSP_GAME/SYSDIR", "/PSP_GAME/USRDIR", "/PSP_GAME/USRDIR/DATA"]:
        iso.add_directory(iso_path=directory, joliet_path=directory)
    def add(source, dest): iso.add_file(str(source), iso_path=dest + ";1", joliet_path=dest)
    add(prx, "/PSP_GAME/SYSDIR/EBOOT.BIN")
    add(prx, "/PSP_GAME/SYSDIR/BOOT.BIN")
    add(sfo, "/PSP_GAME/PARAM.SFO")
    add(dist / "ICON0.PNG", "/PSP_GAME/ICON0.PNG")
    add(dist / "PIC1.PNG", "/PSP_GAME/PIC1.PNG")
    active = set(json.loads((dist / "DATA/MANIFEST.JSON").read_text(encoding="utf-8"))["files"])
    for source in sorted((dist / "DATA").iterdir()):
        if source.is_file() and source.name in active: add(source, "/PSP_GAME/USRDIR/DATA/" + source.name)
    umd = tmp / "UMD_DATA.BIN"
    umd.write_bytes(b"SFWR00001|0001|G")
    add(umd, "/UMD_DATA.BIN")
    if args.audit:
        marker = tmp / "AUDIT"
        marker.write_bytes(b"1")
        add(marker, "/PSP_GAME/USRDIR/DATA/AUDIT")
        if args.performance_audit: add(marker, "/PSP_GAME/USRDIR/DATA/PERF")
    result = dist / ("StarryFlowers-audit.iso" if args.audit else "StarryFlowers.iso")
    iso.write(str(result))
    iso.close()
    check = pycdlib.PyCdlib()
    check.open(str(result))
    with check.open_file_from_iso(iso_path="/PSP_GAME/SYSDIR/EBOOT.BIN;1") as f:
        if f.read(4) != b"\x7fELF": raise RuntimeError("ISO does not contain a native PSP executable")
    check.close()
    print(f"PSP ISO: {result} ({result.stat().st_size / 1024**2:.1f} MiB)")
