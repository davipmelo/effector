# One-off porting helper: turns the declarative parts of the Swift effect structs (params, looks,
# metadata) into Rust builder code and leaves the Swift `apply` body as a comment to port by hand.
import re, sys, pathlib

def num(x):
    x = x.strip()
    if re.fullmatch(r'-?\d+', x): return x + '.0'
    return x

def split_args(s):
    args, depth, cur, q = [], 0, '', False
    for ch in s:
        if ch == '"': q = not q
        if not q:
            if ch in '([{': depth += 1
            if ch in ')]}': depth -= 1
            if ch == ',' and depth == 0:
                args.append(cur.strip()); cur = ''; continue
        cur += ch
    if cur.strip(): args.append(cur.strip())
    return args

def match(s, i, o='(', c=')'):
    d = 0; q = False
    for j in range(i, len(s)):
        if s[j] == '"': q = not q
        if q: continue
        if s[j] == o: d += 1
        elif s[j] == c:
            d -= 1
            if d == 0: return j
    raise ValueError(s[i:i+80])

def color(x):
    x = x.strip()
    m = re.fullmatch(r'RGBA\(hex: "(#[0-9A-Fa-f]+)"\)', x)
    if m: return 'hex("%s")' % m.group(1)
    if x in ('.black', 'RGBA.black'): return 'Rgba::BLACK'
    if x in ('.white', 'RGBA.white'): return 'Rgba::WHITE'
    m = re.fullmatch(r'RGBA\(r: ([^,]+), g: ([^,]+), b: ([^,)]+)\)', x)
    if m: return 'Rgba::new(%s, %s, %s)' % tuple(num(v) for v in m.groups())
    return '/*TODO color*/ ' + x

def tuples(x):
    x = x.strip()
    if x.startswith('['):
        items = re.findall(r'\("([^"]*)",\s*"([^"]*)"\)', x)
        return '&[' + ', '.join('("%s", "%s")' % it for it in items) + ']'
    return '/*TODO opts*/ ' + x

def vis(x):
    x = x.strip()
    m = re.fullmatch(r'when\("(\w+)", "([^"]*)"\)', x)
    if m: return 'when("%s", "%s")' % m.groups()
    m = re.fullmatch(r'whenNot\("(\w+)", "([^"]*)"\)', x)
    if m: return 'when_not("%s", "%s")' % m.groups()
    m = re.fullmatch(r'whenOn\("(\w+)"\)', x)
    if m: return 'when_on("%s")' % m.groups()
    m = re.fullmatch(r'whenAny\("(\w+)", \[(.*)\]\)', x)
    if m: return 'when_any("%s", &[%s])' % (m.group(1), m.group(2))
    return 'Arc::new(|p: &Params| /*TODO*/ { let _ = p; true }) /* %s */' % x.replace('*/', '')

def mods(kw):
    out = ''
    for k, v in kw:
        if k == 'step': out += '.step(%s)' % num(v)
        elif k == 'unit': out += '.unit(%s)' % v
        elif k == 'curve': out += '.curve(%s)' % num(v)
        elif k == 'section': out += '.sec(%s)' % v
        elif k == 'track': out += '.track(vec![%s])' % ', '.join(color(c) for c in split_args(v.strip()[1:-1]))
        elif k == 'visible': out += '.vis(%s)' % vis(v)
        else: out += '/*TODO %s: %s*/' % (k, v)
    return out

def kwsplit(args):
    pos, kw = [], []
    for a in args:
        m = re.match(r'^(\w+):\s*(.*)$', a, re.S)
        if m and not a.startswith('"'): kw.append((m.group(1), m.group(2)))
        else: pos.append(a)
    return pos, kw

def param(e):
    e = e.strip()
    m = re.match(r'^\.(\w+)\(', e)
    if m:
        kind = m.group(1)
        lp = m.end() - 1
        args = split_args(e[lp + 1:match(e, lp)])
        pos, kw = kwsplit(args)
        if kind == 'slider':
            return 's(%s, %s, %s, %s, %s)' % (pos[0], pos[1], num(pos[2]), num(pos[3]), num(pos[4])) + mods(kw)
        if kind == 'toggle':
            return 'ParamDef::toggle(%s, %s, %s)' % (pos[0], pos[1], pos[2]) + mods(kw)
        if kind in ('choice', 'segmented'):
            okw = dict(kw)
            if 'options' in okw:
                rest = [x for x in kw if x[0] != 'options']
                return 'ParamDef::choice_opts(%s, %s, %s, %s)' % (pos[0], pos[1], okw['options'] + '/*TODO*/', pos[2]) + mods(rest)
            fn = 'choice' if kind == 'choice' else 'segmented'
            return 'ParamDef::%s(%s, %s, %s, %s)' % (fn, pos[0], pos[1], tuples(pos[2]), pos[3]) + mods(kw)
        if kind == 'color':
            return 'ParamDef::color(%s, %s, %s)' % (pos[0], pos[1], color(pos[2])) + mods(kw)
        if kind == 'sectionSwitch':
            sec = dict(kw)['section']
            d = pos[1] if len(pos) > 1 else 'true'
            return 'ParamDef::section_switch(%s, %s, %s)' % (pos[0], sec, d)
        if kind == 'seed':
            k = pos[0] if pos else '"seed"'
            return 'ParamDef::seed(%s)' % k + mods(kw)
    m = re.match(r'^ParamDef\(key: ("[^"]*"), label: ("[^"]*"), kind: (.*)\)$', e, re.S)
    if m:
        rest = m.group(3)
        km = re.match(r'^\.(\w+)(\((.*?)\))?((?:, \w+: .*)?)$', rest, re.S)
        if km:
            kind, inner, tail = km.group(1), km.group(3), km.group(4)
            _, kw = kwsplit(split_args(tail.lstrip(', '))) if tail else ([], [])
            if kind == 'curves': base = 'ParamDef::curves(%s, %s)' % (m.group(1), m.group(2))
            elif kind == 'lut': base = 'ParamDef::lut(%s, %s)' % (m.group(1), m.group(2))
            elif kind == 'gradient': base = 'ParamDef::gradient(%s, %s, %s)' % (m.group(1), m.group(2), inner.replace('def: ', '') + '/*TODO*/')
            elif kind == 'palette': base = 'ParamDef::palette(%s, %s, %s)' % (m.group(1), m.group(2), inner.replace('def: ', '') + '/*TODO*/')
            elif kind == 'text': base = 'ParamDef::text(%s, %s, %s)' % (m.group(1), m.group(2), inner.replace('def: ', ''))
            elif kind == 'seed': base = 'ParamDef::seed(%s)' % m.group(1)
            else: base = '/*TODO*/ ' + e
            return base + mods(kw)
    return '/*TODO param*/ ' + e

def value(v):
    v = v.strip()
    if v.startswith('"'): return v
    if v in ('true', 'false'): return v
    if re.fullmatch(r'-?\d+(\.\d+)?', v): return num(v)
    if v.startswith('RGBA') or v in ('.black', '.white'): return color(v)
    return '/*TODO*/ ' + v

def looks(body):
    out = []
    for m in re.finditer(r'EffectLook\(', body):
        lp = m.end() - 1
        args = split_args(body[lp + 1:match(body, lp)])
        _, kw = kwsplit(args)
        d = dict(kw)
        vals = d.get('values', '[:]').strip()[1:-1]
        items = []
        if vals.strip() != ':':
            for a in split_args(vals):
                k, _, v = a.partition(':')
                items.append('%s => %s' % (k.strip(), value(v)))
        if 'group' in d:
            out.append('look_g(%s, %s, %s, vals![%s])' % (d['id'], d['name'], d['group'], ', '.join(items)))
        else:
            out.append('look(%s, %s, vals![%s])' % (d['id'], d['name'], ', '.join(items)))
    return out

def snake(n):
    n = re.sub(r'Effect$', '', n)
    return re.sub(r'(?<!^)(?=[A-Z])', '_', n).lower()

src = pathlib.Path(sys.argv[1]).read_text(encoding='utf-8')
out = ['use super::*;', '']
for m in re.finditer(r'^struct (\w+): Effect \{', src, re.M):
    start = m.end() - 1
    end = match(src, start, '{', '}')
    body = src[start + 1:end]
    get = lambda pat: (re.search(pat, body).group(1) if re.search(pat, body) else None)
    eid = get(r'let id = ("[^"]*")')
    name = get(r'let name = ("[^"]*")')
    cat = get(r'let category = EffectCategory\.(\w+)')
    sym = get(r'var symbol: String \{ ("[^"]*") \}')
    summ = get(r'var summary: String \{ ("(?:[^"\\]|\\.)*") \}')
    pm = re.search(r'(?:let|var) params: \[ParamDef\](?: =| \{)\s*\[', body)
    params = []
    if pm:
        lb = pm.end() - 1
        rb = match(body, lb, '[', ']')
        for a in split_args(body[lb + 1:rb]):
            params.append(param(a))
    lk = []
    lm = re.search(r'var looks: \[EffectLook\]', body)
    if lm:
        lk = looks(body[lm.end():])
    am = re.search(r'func apply\(_ input: MTLTexture, _ p: Params, _ ctx: RenderContext\) -> MTLTexture \{', body)
    apply_src = ''
    if am:
        ab = am.end() - 1
        apply_src = body[ab + 1:match(body, ab, '{', '}')]
    catname = cat[0].upper() + cat[1:]
    out.append('pub fn %s() -> Effect {' % snake(m.group(1)))
    out.append('    Effect::new(%s, %s, Category::%s, |input, p, ctx| {' % (eid, name, catname))
    out.append('        // SWIFT:')
    for l in apply_src.strip('\n').split('\n'):
        out.append('        //' + l)
    out.append('        todo!()')
    out.append('    })')
    if sym: out.append('    .symbol(%s)' % sym)
    if summ: out.append('    .summary(%s)' % summ)
    if params:
        out.append('    .params(vec![')
        for p_ in params: out.append('        %s,' % p_)
        out.append('    ])')
    if lk:
        out.append('    .looks(vec![')
        for l in lk: out.append('        %s,' % l)
        out.append('    ])')
    out.append('}')
    out.append('')
    # leftovers worth a look (helper methods etc.)
    rest = re.sub(r'func apply\(.*', '', body, flags=re.S)
    extra = [l for l in rest.split('\n') if re.search(r'\bfunc \b|static ', l)]
    if extra:
        out.append('// SWIFT extra members of %s:' % m.group(1))
        for l in extra: out.append('// ' + l.strip())
        out.append('')
pathlib.Path(sys.argv[2]).write_text('\n'.join(out), encoding='utf-8')
print(sys.argv[2], src.count('struct '), 'structs')
