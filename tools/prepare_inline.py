"""Rasterize the exact inline symbols shipped with the original Ren'Py runtime."""
import argparse,json,pathlib,shutil
from PIL import Image,ImageDraw,ImageFont
ROOT=pathlib.Path(__file__).resolve().parents[1]
parser=argparse.ArgumentParser();parser.add_argument('--original',type=pathlib.Path,default=ROOT.parent/'StarryFlowers-1.7.4-pc');args=parser.parse_args()
common=args.original/'renpy/common';out=ROOT/'assets/gui/emoji';out.mkdir(parents=True,exist_ok=True)
font=ImageFont.truetype(str(common/'TwemojiCOLRv0.ttf'),128)
fallback=ImageFont.truetype(str(common/'DejaVuSans.ttf'),128)
symbols={}
for code in [0x1f499,0x1f90e,0x2764,0x2665,0x2714,0x2718]:
 c=chr(code);f=fallback if code==0x2718 else font
 box=f.getbbox(c);image=Image.new('RGBA',(box[2]-box[0],box[3]-box[1]));ImageDraw.Draw(image).text((-box[0],-box[1]),c,font=f,embedded_color=True,fill='white');name=f'{code:x}';image.save(out/(name+'.png'))
 symbols[c]={'image':'ui emoji/'+name,'advance':f.getlength(c)/128,'width':image.width/128,'height':image.height/128,'baseline_top':(box[1]-f.getmetrics()[0])/128,'tint':code==0x2718}
(out/'symbols.json').write_text(json.dumps(symbols,ensure_ascii=False,indent=2),encoding='utf-8')
shutil.copyfile(common/'TwemojiCOLRv0.txt',out/'Twemoji-LICENSE.txt');shutil.copyfile(common/'DejaVuSans.txt',out/'DejaVu-LICENSE.txt')
print('Extracted',len(symbols),'original inline graphics')
