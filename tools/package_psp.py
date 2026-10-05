"""Package the native PRX as a PSP UMD ISO and a Memory Stick homebrew directory."""
import argparse
import pathlib
import shutil
import subprocess
import tempfile
import pycdlib

ROOT = pathlib.Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--audit", action="store_true", help="Make a separate emulator audit image")
args = parser.parse_args()
binary = ROOT / "psp/target/mipsel-sony-psp/release"
dist = ROOT / "psp/dist"
prx = binary / "starryflowers-psp.prx"
pbp = max(binary.glob("*EBOOT.PBP"), key=lambda p: p.stat().st_mtime)
if not prx.exists() or not pbp.exists(): raise SystemExit("Build with cargo +nightly psp --release first")
shutil.copy2(pbp, dist / "EBOOT.PBP")
with tempfile.TemporaryDirectory() as tmp:
    tmp = pathlib.Path(tmp)
    sfo = tmp / "PARAM.SFO"
    subprocess.run(["mksfo", "--bare", "-s", "TITLE=Starry Flowers", "-s", "CATEGORY=UG", "-s", "DISC_ID=SFWR00001", "-s", "DISC_VERSION=1.00", "-s", "PSP_SYSTEM_VER=1.00", "-d", "BOOTABLE=1", "-d", "PARENTAL_LEVEL=1", "Starry Flowers", str(sfo)], check=True)
    iso = pycdlib.PyCdlib()
    iso.new(interchange_level=3, sys_ident="PSP GAME", vol_ident="STARRY_FLOWERS", joliet=3)
    for directory in ["/PSP_GAME", "/PSP_GAME/SYSDIR", "/PSP_GAME/USRDIR", "/PSP_GAME/USRDIR/DATA"]:
        iso.add_directory(iso_path=directory, joliet_path=directory)
    def add(source, dest): iso.add_file(str(source), iso_path=dest + ";1", joliet_path=dest)
    add(prx, "/PSP_GAME/SYSDIR/EBOOT.BIN")
    add(prx, "/PSP_GAME/SYSDIR/BOOT.BIN")
    add(sfo, "/PSP_GAME/PARAM.SFO")
    for source in sorted((dist / "DATA").iterdir()):
        if source.is_file() and source.name != "AUDIT": add(source, "/PSP_GAME/USRDIR/DATA/" + source.name)
    umd = tmp / "UMD_DATA.BIN"
    umd.write_bytes(b"SFWR00001|0001|G")
    add(umd, "/UMD_DATA.BIN")
    if args.audit:
        marker = tmp / "AUDIT"
        marker.write_bytes(b"1")
        add(marker, "/PSP_GAME/USRDIR/DATA/AUDIT")
    result = dist / ("StarryFlowers-audit.iso" if args.audit else "StarryFlowers.iso")
    iso.write(str(result))
    iso.close()
    check = pycdlib.PyCdlib()
    check.open(str(result))
    with check.open_file_from_iso(iso_path="/PSP_GAME/SYSDIR/EBOOT.BIN;1") as f:
        if f.read(4) != b"\x7fELF": raise RuntimeError("ISO does not contain a native PSP executable")
    check.close()
    print(f"PSP ISO: {result} ({result.stat().st_size / 1024**2:.1f} MiB)")
