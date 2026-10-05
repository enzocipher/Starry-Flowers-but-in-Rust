import pathlib, pickle, zlib

root = pathlib.Path(__file__).resolve().parents[1]
archive = root.parent / 'StarryFlowers-1.7.4-pc/game/archive.rpa'
with archive.open('rb') as f:
    header = f.readline()
    offset, key = int(header[8:24], 16), int(header[25:33], 16)
    f.seek(offset)
    index = pickle.loads(zlib.decompress(f.read()), encoding='bytes')
    for name, chunks in index.items():
        if isinstance(name, bytes): name = name.decode('utf8')
        target = (root / 'assets' / name).resolve()
        if not target.is_relative_to((root / 'assets').resolve()):
            raise ValueError(name)
        target.parent.mkdir(parents=True, exist_ok=True)
        with target.open('wb') as out:
            for chunk in chunks:
                position, length = chunk[0] ^ key, chunk[1] ^ key
                prefix = chunk[2] if len(chunk) > 2 else b''
                if isinstance(prefix, str): prefix = prefix.encode('latin1')
                f.seek(position)
                out.write(prefix + f.read(length))
print(f'Extracted {len(index)} files')
