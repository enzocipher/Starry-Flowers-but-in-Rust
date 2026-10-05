"""Compile this game's Ren'Py narrative into explicit Rust VM instructions."""
import ast, collections, json, pathlib, re
root = pathlib.Path(__file__).resolve().parents[1]
assets = root / 'assets'
ops, labels, ignored = [], {}, collections.Counter()
quoted = re.compile(r'"(?:[^"\\]|\\.)*"')
def strings(s):
    return [ast.literal_eval(m.group()) for m in quoted.finditer(s)]
def emit(op, **kw):
    ops.append(dict(op=op, **kw)); return len(ops)-1
def say(s):
    m = quoted.search(s)
    if not m: return None
    prefix = s[:m.start()].strip()
    if prefix.startswith('_('): prefix = ''
    if prefix and not re.fullmatch(r'(w|p|a|c|j|k|r|u|h|g|n|centered)(\s+[\w-]+)*', prefix): return None
    parts = prefix.split()
    effect=re.search(r'\bwith\s+(.+?)(?:\s+id\s+\w+)?$',s[m.end():])
    return dict(who=parts[0] if parts else '', attrs=parts[1:], text=ast.literal_eval(m.group()),effect=effect.group(1) if effect else '')
def block(lines, i, indent):
    while i < len(lines):
        level, s = lines[i]
        if level < indent: break
        if level > indent: raise ValueError(('unexpected indent', s))
        i += 1
        if s.startswith(('if ', 'elif ')):
            branch = emit('if', condition=s.split(' ',1)[1].rstrip(':'), target=0)
            i = block(lines, i, lines[i][0])
            if i < len(lines) and lines[i][0] == indent and lines[i][1] == 'else:':
                jump = emit('goto', target=0); ops[branch]['target'] = len(ops)
                i += 1; i = block(lines,i,lines[i][0]); ops[jump]['target']=len(ops)
            else: ops[branch]['target'] = len(ops)
        elif s.startswith('menu') and s.endswith(':'):
            name=s[4:].strip().rstrip(':')
            if name: labels[name]=len(ops)
            menu = emit('menu', choices=[])
            exits=[]
            while i < len(lines) and lines[i][0] > indent:
                itemlevel, item=lines[i]; i+=1
                if not item.endswith(':'):
                    prompt=say(item)
                    if prompt: ops[menu]['text']=prompt['text']
                    continue
                ops[menu]['choices'].append(dict(text=strings(item)[0],target=len(ops)))
                i=block(lines,i,lines[i][0]); exits.append(emit('goto',target=0))
            for exit in exits: ops[exit]['target']=len(ops)
        elif s.startswith('label '): labels[s[6:].rstrip(':')]=len(ops)
        elif s.startswith('$ '): emit('set',text=s[2:])
        elif s.startswith(('scene ', 'show ', 'hide ')):
            action,arg=s.split(' ',1)
            if arg.startswith('screen '): ignored[action+' screen']+=1
            else: emit(action,text=arg)
        elif s.startswith(('play ', 'stop ')):
            parts=s.split(); emit(parts[0],channel=parts[1],text=parts[2] if len(parts)>2 else '')
        elif s.startswith('call screen accessorize'): emit('dress',text=strings(s)[0])
        elif s.startswith('call screen '): ignored[s]+=1
        elif s.startswith(('jump ', 'call ')): emit(s.split()[0],text=s.split()[1])
        elif s == 'return': emit('return')
        elif s.startswith('pause'): emit('pause',text=s[5:].strip())
        elif s.startswith(('window ', 'nvl ', 'with ')): emit(s.split()[0],text=s.split(' ',1)[1])
        elif s.startswith('extend '): emit('say',who='extend',attrs=[],text=strings(s)[0])
        elif (dialogue:=say(s)) is not None: emit('say',**dialogue)
        else: ignored[s.split()[0]]+=1
    return i
for filename in ['script.rpy','extra.rpy','credits.rpy']:
    lines=[]; active=False
    for line in (assets/filename).read_text(encoding='utf-8-sig').splitlines():
        s=line.strip()
        if s.startswith('label '): active=True
        if not active or not s or s.startswith('#'): continue
        if not line.startswith((' ','\t')) and not s.startswith('label '): active=False; continue
        # Comments outside strings only.
        s=re.split(r'\s+#',s)[0].rstrip()
        lines.append((len(line)-len(line.lstrip()),s))
    block(lines,0,0) if not lines else None
    # Top-level labels followed by their indented bodies.
    i=0
    while i<len(lines):
        level,s=lines[i]
        if level != 0: raise ValueError(s)
        labels[s[6:].rstrip(':')]=len(ops); i+=1
        if i<len(lines) and lines[i][0]>0: i=block(lines,i,lines[i][0])

images={p.stem: p.relative_to(assets).as_posix() for p in (assets/'images').rglob('*.png')}
images.update({'ui '+p.relative_to(assets/'gui').with_suffix('').as_posix():p.relative_to(assets).as_posix() for p in (assets/'gui').rglob('*.png')})
text_images={}
for filename in ['script.rpy','credits.rpy']:
    for line in (assets/filename).read_text(encoding='utf-8-sig').splitlines():
        if line.startswith('image ') and '= Text(' in line:
            name=line[6:].split(' = ')[0]
            text_images[name]=strings(line)[0]
translations={}
for folder in (assets/'tl').iterdir():
    if not folder.is_dir(): continue
    mapping={}
    for file in folder.glob('*.rpy'):
        original=None
        for line in file.read_text(encoding='utf-8-sig').splitlines():
            s=line.strip()
            if s.startswith('# '):
                d=say(s[2:]); original=d['text'] if d else None
            elif s.startswith('old '): original=strings(s)[0]
            elif s.startswith('new ') and original is not None: mapping[original]=strings(s)[0]; original=None
            elif original is not None and (d:=say(s)):
                mapping[original]=d['text']; original=None
    if mapping: translations[folder.name]=mapping
accessories=[]
source=(assets/'acc.rpy').read_text(encoding='utf-8-sig')
for n in range(1,4):
    expr=re.search(r'define periacc'+str(n)+r' = (\[.*?\])',source,re.S).group(1)
    accessories.append(ast.literal_eval(expr))
for op in ops:
    if op['op'] in ('jump','call') and op['text'] not in labels and op['text'] != 'gallery': raise ValueError(op)
data=dict(ops=ops,labels=labels,images=images,translations=translations,accessories=accessories,text_images=text_images)
(root/'story.json').write_text(json.dumps(data,ensure_ascii=False),encoding='utf8')
print(f'{len(ops)} instructions, {len(labels)} labels, {len(images)} images; translations: {list(translations)}')
print('Uncompiled commands:',dict(ignored))
