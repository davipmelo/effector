# One-off porting helper: turns the Swift Portuguese dictionaries (PT*.swift) into src/l10n_pt.rs.
# Keys are the English UI strings; lookup order follows the Swift `tr` (dict, v2, v3, v4, v5).
import re, pathlib, json

root = pathlib.Path('Sources/Effector/Support')
lit = r'"((?:[^"\\]|\\.)*)"'
pairs = []
seen = set()
for name in ['PT.swift', 'PT2.swift', 'PT3.swift', 'PT4.swift', 'PT5.swift']:
    s = (root / name).read_text(encoding='utf-8')
    # strip line comments outside strings (good enough: comments start the line)
    s = re.sub(r'^\s*//.*$', '', s, flags=re.M)
    for m in re.finditer(lit + r'\s*:\s*' + lit, s):
        k, v = m.group(1), m.group(2)
        if k in seen:
            continue
        seen.add(k)
        pairs.append((k, v))

def unswift(x):
    # Swift escapes are compatible with Rust string escapes for \n \t \" \\ ; \u{...} too.
    return x

out = ['// Generated from the macOS app\'s PT*.swift (Portuguese translations, key = English text).',
       'pub const PT: &[(&str, &str)] = &[']
for k, v in pairs:
    out.append('    ("%s", "%s"),' % (unswift(k), unswift(v)))
out.append('];')
pathlib.Path('src/l10n_pt.rs').write_text('\n'.join(out) + '\n', encoding='utf-8')
print(len(pairs), 'strings')
