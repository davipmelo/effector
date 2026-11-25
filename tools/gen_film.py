# One-off porting helper: converts the Swift film stock table into src/effects/film.rs.
import re, pathlib
s = pathlib.Path('Sources/Effector/Effects/FilmEffect.swift').read_text(encoding='utf-8')
lst = s[s.index('private static let list'):s.index('/// Stocks ordered')]
items = []
mp = {'group': 'group', 'bw': 'bw', 'saturation': 'saturation', 'contrast': 'contrast', 'fade': 'fade', 'whiteClip': 'white_clip',
      'gamma': 'gamma', 'shadowTint': 'shadow_tint', 'highlightTint': 'highlight_tint', 'bwWeights': 'bw_weights', 'grain': 'grain',
      'grainSize': 'grain_size', 'halation': 'halation', 'matrix': 'matrix'}
for m in re.finditer(r'FilmStock\((.*?)\),\n', lst, re.S):
    args = re.sub(r'\s+', ' ', m.group(1))
    parts = []; d = 0; cur = ''
    for ch in args:
        if ch == '[': d += 1
        if ch == ']': d -= 1
        if ch == ',' and d == 0:
            parts.append(cur.strip()); cur = ''
        else:
            cur += ch
    parts.append(cur.strip())
    kv = dict(p.split(': ', 1) for p in parts)
    fields = []
    for k, v in kv.items():
        if k in ('id', 'name'): continue
        if v.startswith('[') or re.fullmatch(r'-?\d+', v):
            v = re.sub(r'(?<![\d.])(-?\d+)(?![\d.])', r'\1.0', v)
        fields.append('%s: %s' % (mp[k], v))
    items.append('        FilmStock { %s..FilmStock::base(%s, %s) },' % (''.join(f + ', ' for f in fields), kv['id'], kv['name']))
print(len(items), 'stocks')
tpl = pathlib.Path('tools/film_template.rs').read_text(encoding='utf-8')
pathlib.Path('src/effects/film.rs').write_text(tpl.replace('__STOCKS__', '\n'.join(items)), encoding='utf-8')
