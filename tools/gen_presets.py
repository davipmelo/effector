# One-off porting helper: converts the Swift factory preset table (FactoryPresets.all) into Rust
# (src/presets_factory.rs) with a tiny recursive-descent parser over the Swift DSL.
import re, pathlib

s = pathlib.Path('Sources/Effector/Model/Presets.swift').read_text(encoding='utf-8')
body = s[s.index('static let all: [Preset] = ['):]
body = body[body.index('= [') + 3:]
# strip comments
body = re.sub(r'//[^\n]*', '', body)

pos = 0
def peek():
    global pos
    while pos < len(body) and body[pos].isspace(): pos += 1
    return body[pos] if pos < len(body) else ''

def eat(ch):
    global pos
    assert peek() == ch, (ch, body[pos:pos + 80])
    pos += 1

def string():
    global pos
    peek()
    assert body[pos] == '"'
    j = pos + 1
    while body[j] != '"':
        if body[j] == '\\': j += 1
        j += 1
    v = body[pos:j + 1]
    pos = j + 1
    return v

def ident():
    global pos
    peek()
    m = re.match(r'[A-Za-z_][A-Za-z0-9_.]*', body[pos:])
    pos += m.end()
    return m.group(0)

def number():
    global pos
    peek()
    m = re.match(r'-?\d+(\.\d+)?', body[pos:])
    pos += m.end()
    v = m.group(0)
    return v if '.' in v else v + '.0'

def args():
    """parse '(' ... ')' into list of (label|None, value)"""
    eat('(')
    out = []
    while peek() != ')':
        m = re.match(r'([A-Za-z_]\w*)\s*:(?!:)', body[pos:])
        label = None
        if m and not body[pos:].startswith('"'):
            label = m.group(1)
            globals()['pos'] += m.end()
        out.append((label, expr()))
        if peek() == ',': eat(',')
    eat(')')
    return out

def expr():
    c = peek()
    if c == '"': return ('str', string())
    if c == '-' or c.isdigit(): return ('num', number())
    if c == '.':
        eat('.')
        return ('member', ident())
    if c == '[':
        eat('[')
        if peek() == ':':
            eat(':'); eat(']'); return ('dict', [])
        items = []
        is_dict = False
        while peek() != ']':
            k = expr()
            if peek() == ':':
                eat(':'); is_dict = True
                v = expr()
                items.append((k, v))
            else:
                items.append(k)
            if peek() == ',': eat(',')
        eat(']')
        return ('dict', items) if is_dict else ('arr', items)
    if c == '(':
        a = args()
        return ('tuple', [v for _, v in a])
    name = ident()
    if peek() == '(':
        return ('call', name, args())
    return ('id', name)

blend_map = {'softLight': 'SoftLight', 'multiply': 'Multiply', 'color': 'Color', 'overlay': 'Overlay', 'screen': 'Screen',
             'normal': 'Normal', 'lighten': 'Lighten', 'darken': 'Darken', 'luminosity': 'Luminosity', 'difference': 'Difference',
             'hardLight': 'HardLight', 'add': 'Add', 'colorDodge': 'ColorDodge', 'colorBurn': 'ColorBurn', 'exclusion': 'Exclusion',
             'hue': 'Hue', 'saturation': 'Saturation', 'linearBurn': 'LinearBurn'}

def pts(e):
    if e is None: return '&[]'
    assert e[0] == 'arr', e
    return '&[' + ', '.join('(%s, %s)' % (emit(t[1][0]), emit(t[1][1])) for t in e[1]) + ']'

def emit(e):
    k = e[0]
    if k == 'str': return e[1]
    if k == 'num': return e[1]
    if k == 'id':
        n = e[1]
        if n in ('true', 'false'): return n
        if n == 'sCurve': return 's_curve()'
        if n == 'softS': return 'soft_s()'
        raise ValueError(n)
    if k == 'dict':
        return 'vals![' + ', '.join('%s => %s' % (emit(a), emit(b)) for a, b in e[1]) + ']'
    if k == 'arr':
        return 'vec![\n' + ',\n'.join(emit(x) for x in e[1]) + ',\n]'
    if k == 'call':
        name, a = e[1], e[2]
        lab = {l: v for l, v in a if l}
        pos_ = [v for l, v in a if not l]
        op = emit(lab['opacity']) if 'opacity' in lab else '1.0'
        bl = 'BlendMode::' + blend_map[lab['blend'][1]] if 'blend' in lab else 'BlendMode::Normal'
        nm = 'Some(%s)' % emit(lab['name']) if 'name' in lab else 'None'
        if name == 'RGBA':
            return 'hex(%s)' % emit(lab['hex'])
        if name == 'ParamValue.text':
            return 'ParamValue::Text(%s.into())' % emit(pos_[0])
        if name == 'gradient': return 'swatches::gradient_named(%s)' % emit(pos_[0])
        if name == 'palette': return 'swatches::palette_named(%s)' % emit(pos_[0])
        if name == 'curves':
            return 'curves(%s, %s, %s, %s)' % (pts(pos_[0] if pos_ else None), pts(lab.get('r')), pts(lab.get('g')), pts(lab.get('b')))
        if name == 'make':
            return 'make(%s, %s, %s)' % (emit(pos_[0]), emit(pos_[1]), emit(pos_[2]))
        if name == 'fx':
            vals = emit(pos_[1]) if len(pos_) > 1 else 'vals![]'
            return 'fx(%s, %s, %s, %s, %s)' % (emit(pos_[0]), nm, op, bl, vals)
        if name == 'look':
            vals = emit(pos_[2]) if len(pos_) > 2 else 'vals![]'
            return 'look(%s, %s, %s, %s, %s, %s)' % (emit(pos_[0]), emit(pos_[1]), nm, op, bl, vals)
        if name == 'grp':
            return 'grp(%s, %s, %s, %s)' % (emit(pos_[0]), op, bl, emit(pos_[1]))
        if name == 'film':
            vals = emit(pos_[1]) if len(pos_) > 1 else 'vals![]'
            g = emit(lab['grain']) if 'grain' in lab else '0.0'
            h = emit(lab['halation']) if 'halation' in lab else '30.0'
            return 'film(%s, %s, %s, %s)' % (emit(pos_[0]), g, h, vals)
        raise ValueError(name)
    if k == 'member':
        raise ValueError('member ' + e[1])
    if k == 'tuple':
        return '(' + ', '.join(emit(x) for x in e[1]) + ')'
    raise ValueError(e)

presets = []
while peek() != ']':
    presets.append(expr())
    if peek() == ',': eat(',')
print(len(presets), 'presets')
code = '// Generated from the macOS app\'s FactoryPresets.all (Presets.swift).\n'
code += 'pub fn all() -> Vec<Preset> {\n    vec![\n'
for p in presets:
    code += emit(p) + ',\n'
code += '    ]\n}\n'
pathlib.Path('src/presets_factory.rs').write_text(code, encoding='utf-8')
