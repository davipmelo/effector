# One-off porting tool: converts the app's Metal compute kernels (tools/all.metal) into GLSL compute
# shaders for wgpu/naga. Output: src/shaders/prelude.glsl + src/shaders/kernels/<name>.glsl.
# The generated files are then maintained by hand; this script is kept only as a record of the port.
import re, sys, pathlib

src = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
outdir = pathlib.Path(sys.argv[2])
(outdir / "kernels").mkdir(parents=True, exist_ok=True)

# Drop the Metal header / macros / samplers and the display (vertex/fragment) section.
src = re.sub(r'#include <metal_stdlib>\s*using namespace metal;', '', src)
src = re.sub(r'#define (KIN|GUARD|UV|OSIZE)[^\n]*\n', '', src)
src = re.sub(r'constexpr sampler s(Lin|Near|Rep)\([^)]*\);', '', src)
src = re.sub(r'// ==== displayShaders ====.*?(?=// ====)', '', src, flags=re.S)


def match_paren(s, i):
    """s[i] == '(' -> index of matching ')'"""
    depth = 0
    for j in range(i, len(s)):
        if s[j] == '(': depth += 1
        elif s[j] == ')':
            depth -= 1
            if depth == 0: return j
    raise ValueError("unbalanced")


def split_args(s):
    args, depth, cur = [], 0, ''
    for ch in s:
        if ch in '([{': depth += 1
        if ch in ')]}': depth -= 1
        if ch == ',' and depth == 0:
            args.append(cur.strip()); cur = ''
        else:
            cur += ch
    if cur.strip(): args.append(cur.strip())
    return args


def convert_methods(s):
    pat = re.compile(r'\b(\w+)\.(sample|read|write|get_width|get_height)\(')
    while True:
        m = pat.search(s)
        if not m: return s
        obj, meth = m.group(1), m.group(2)
        lp = m.end() - 1
        rp = match_paren(s, lp)
        args = split_args(s[lp + 1:rp])
        if meth == 'sample':
            smp, uv = args[0], args[1]
            if len(args) > 2:
                lvl = re.match(r'level\((.*)\)$', args[2])
                rep = 'textureLod(sampler2D(%s, %s), %s, %s)' % (obj, smp, uv, lvl.group(1) if lvl else args[2])
            else:
                rep = 'texture(sampler2D(%s, %s), %s)' % (obj, smp, uv)
        elif meth == 'read':
            rep = 'texelFetch(sampler2D(%s, sNear), ivec2(%s), 0)' % (obj, args[0])
        elif meth == 'write':
            rep = 'imageStore(%s, ivec2(%s), %s)' % (obj, args[1], args[0])
        else:
            comp = 'x' if meth == 'get_width' else 'y'
            if obj == 'o':
                rep = 'uint(imageSize(o).%s)' % comp
            else:
                rep = 'uint(textureSize(sampler2D(%s, sNear), 0).%s)' % (obj, comp)
        s = s[:m.start()] + rep + s[rp + 1:]


def common(s):
    s = re.sub(r'\binline\s+', '', s)
    s = re.sub(r'\bconstant float\* (u|ex)\b,?\s*', '', s)
    s = re.sub(r',\s*\)', ')', s)
    s = re.sub(r'\bconstant (float \w+\[\d+\]) = \{([^}]*)\};',
               lambda m: 'const %s = float[](%s);' % (m.group(1), m.group(2)), s)
    s = re.sub(r'\bconst (float \w+\[\d+\]) = \{([^}]*)\};',
               lambda m: 'const %s = float[](%s);' % (m.group(1), m.group(2)), s)
    s = re.sub(r'texture2d<float(, access::\w+)?>', 'texture2D', s)
    for n in '234':
        s = re.sub(r'\bfloat%s\b' % n, 'vec' + n, s)
        s = re.sub(r'\bint%s\b' % n, 'ivec' + n, s)
        s = re.sub(r'\buint%s\b' % n, 'uvec' + n, s)
        s = re.sub(r'\bbool%s\b' % n, 'bvec' + n, s)
    s = re.sub(r'\bfloat3x3\b', 'mat3', s)
    s = re.sub(r'\bfloat2x2\b', 'mat2', s)
    s = re.sub(r'\batan2\(', 'atan(', s)
    s = re.sub(r'\bfmod\(', 'fmodc(', s)
    s = re.sub(r'\brsqrt\(', 'inversesqrt(', s)
    s = re.sub(r'\bsaturate\(', 'sat(', s)
    s = re.sub(r'(\d\.\d*|\d)f\b', r'\1', s)
    s = re.sub(r'\bnearestPalette\(([^,]+), ex, ', r'nearestPalette(\1, ', s)
    s = convert_methods(s)
    return s


kernels = []
prelude = []
pos = 0
kpat = re.compile(r'kernel void (\w+)\(KIN(.*?)\)\s*\{', re.S)
while True:
    m = kpat.search(src, pos)
    if not m:
        prelude.append(src[pos:]); break
    prelude.append(src[pos:m.start()])
    # body
    i = m.end() - 1
    depth = 0
    for j in range(i, len(src)):
        if src[j] == '{': depth += 1
        elif src[j] == '}':
            depth -= 1
            if depth == 0: break
    body = src[i + 1:j]
    params = m.group(2)
    texs = re.findall(r'texture2d<float> (\w+) \[\[texture\((\d)\)\]\]', params)
    kernels.append((m.group(1), texs, body))
    pos = j + 1

pre = common(''.join(prelude))
pre = re.sub(r'\n\s*\n\s*\n+', '\n\n', pre)
(outdir / 'prelude.glsl').write_text(pre.strip() + '\n', encoding='utf-8')

for name, texs, body in kernels:
    b = body.replace('GUARD', 'if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;')
    b = b.replace('OSIZE', 'vec2(imageSize(o))')
    b = re.sub(r'\bUV\b', '((vec2(gid) + 0.5) / vec2(imageSize(o)))', b)
    b = common(b)
    lines = ['// textures: ' + ' '.join('%s=%s' % (t, n) for t, n in texs)]
    for t, n in texs:
        lines.append('layout(set = 0, binding = %d) uniform texture2D %s;' % (6 + int(n), t))
    lines.append('void main() {')
    lines.append('    uvec2 gid = gl_GlobalInvocationID.xy;')
    lines.append(b.rstrip())
    lines.append('}')
    (outdir / 'kernels' / (name + '.glsl')).write_text('\n'.join(lines) + '\n', encoding='utf-8')
print(len(kernels), 'kernels')
