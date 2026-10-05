"""Preserve original art in lossless GPU tiles, with mipmaps and 4x font atlases."""
import hashlib, json, math, pathlib, subprocess, zlib, shutil
from PIL import Image, ImageDraw, ImageFont
import imageio_ffmpeg
ROOT=pathlib.Path(__file__).resolve().parents[1]
OUT=ROOT/'psp/dist/DATA'; OUT.mkdir(parents=True,exist_ok=True)
story=json.loads((ROOT/'story.json').read_text(encoding='utf-8'))
story['translations']={k:v for k,v in story['translations'].items() if k=='es'}
(ROOT/'psp/story.json').write_text(json.dumps(story,ensure_ascii=False,separators=(',',':')),encoding='utf-8')
images=dict(story['images']); images.update({'ui '+n:'gui/'+n+'.png' for n in ['main_menu','game_menu','textbox','nvl']})
manifest={'images':{},'fonts':{},'audio':{},'version':3,'inline_symbols':story.get('inline_symbols',{})}
files={}; active={'MANIFEST.JSON'}
for source,target in [('Twemoji-LICENSE.txt','TWEMOJI.TXT'),('DejaVu-LICENSE.txt','DEJAVU.TXT')]:
 shutil.copyfile(ROOT/'assets/gui/emoji'/source,OUT/target);active.add(target)
def power2(n): return 1<<(n-1).bit_length()
for name,path in images.items():
 if path not in files:
  original=Image.open(ROOT/'assets'/path).convert('RGBA'); ow,oh=original.size
  levels=[original]+[original.resize((max(1,(ow+(1<<l)-1)//(1<<l)),max(1,(oh+(1<<l)-1)//(1<<l))),Image.Resampling.LANCZOS) for l in [1,2]]
  tiles=[]
  for y in range(0,oh,448):
   for x in range(0,ow,448):
    w,h=min(448,ow-x),min(448,oh-y); tw,th=power2(w+64),power2(h+64)
    filename='T'+hashlib.sha256(path.encode()).hexdigest()[:8].upper()+f'{x//448:02X}{y//448:02X}.ZTX'
    data=bytearray()
    for level,img in enumerate(levels):
     scale=1<<level
     tile=img.crop(((x-32)//scale,(y-32)//scale,(x-32+tw)//scale,(y-32+th)//scale))
     data.extend(tile.tobytes())
    (OUT/filename).write_bytes(zlib.compress(data,6)); active.add(filename)
    tiles.append({'file':filename,'x':x,'y':y,'width':w,'height':h,'texture_width':tw,'texture_height':th})
  files[path]={'width':max(1,round(ow*.375)),'height':max(1,round(oh*.375)),'original_width':ow,'original_height':oh,'tiles':tiles}
 manifest['images'][name]=files[path]
chars=set(''.join(op.get('text','') for op in story['ops']))
chars.add('∞'); chars.update(''.join(story['translations'].get('es',{}).values())); chars.update(chr(i) for i in range(32,256))
chars=sorted(c for c in chars if c.isprintable())
for size in [8,10,12,14,18,24]:
 scale=4; font=ImageFont.truetype(str(ROOT/'assets/tl/None/Nunito-Bold.ttf'),size*scale)
 pages=[Image.new('L',(512,512))]; page=0; x=y=2; row=0; glyphs={}
 for c in chars:
  box=font.getbbox(c); w=max(1,box[2]-box[0]); h=max(1,box[3]-box[1]); glyph=Image.new('L',(w,h)); ImageDraw.Draw(glyph).text((-box[0],-box[1]),c,font=font,fill=255)
  left=box[0]/scale; top=box[1]/scale; advance=font.getlength(c)/scale
  if x+w+2>512: x=2;y+=row+4;row=0
  if y+h+2>512: pages.append(Image.new('L',(512,512)));page+=1;x=y=2;row=0
  pages[page].paste(glyph,(x,y)); filename=f'F{size:02}P{page:02}.RAW'
  glyphs[c]={'file':filename,'x':x,'y':y,'width':w,'height':h,'left':left,'top':top,'advance':advance}
  x+=w+4;row=max(row,h)
 for index,image in enumerate(pages):
  filename=f'F{size:02}P{index:02}.RAW'; (OUT/filename).write_bytes(image.tobytes());active.add(filename)
 manifest['fonts'][str(size)]={'scale':scale,'ascent':font.getmetrics()[0]/scale,'glyphs':glyphs}
ffmpeg=imageio_ffmpeg.get_ffmpeg_exe()
for path in sorted((ROOT/'assets/audio').iterdir()):
 if path.suffix.lower() not in {'.ogg','.wav'}:continue
 filename='S'+hashlib.sha256(path.stem.encode()).hexdigest()[:12].upper()+'.PCM'; dest=OUT/filename
 if not dest.exists():subprocess.run([ffmpeg,'-v','error','-y','-i',str(path),'-f','s16le','-ac','2','-ar','44100',str(dest)],check=True)
 manifest['audio'][path.stem]=filename;active.add(filename)
manifest['files']=sorted(active)
(OUT/'MANIFEST.JSON').write_text(json.dumps(manifest,ensure_ascii=False,separators=(',',':')),encoding='utf-8')
print(f"Original-resolution art: {len(files)} images; {sum(len(i['tiles']) for i in files.values())} GPU tiles; {sum((OUT/p).stat().st_size for p in active)/1024**2:.1f} MiB")
