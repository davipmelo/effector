# One-off porting helper: fills the `todo!()` apply bodies of a generated draft (tools/draft/*.rs)
# with hand-ported Rust from a bodies file. Bodies file format:
#   //== fn_name [-> new_name]
#   <rust body lines>
#   //== header            (optional: code inserted after `use super::*;`)
import re, sys, pathlib

draft, bodies_path, out = sys.argv[1:4]
d = pathlib.Path(draft).read_text(encoding='utf-8')
d = re.sub(r'\n        // SWIFT:\n(        //.*\n)*        todo!\(\)\n', '\n        APPLY\n', d)
d = re.sub(r'\n// SWIFT extra.*?\n\n', '\n', d, flags=re.S)
src = pathlib.Path(bodies_path).read_text(encoding='utf-8')
parts = re.split(r'^//== (.*)$', src, flags=re.M)
for i in range(1, len(parts), 2):
    head, body = parts[i].strip(), parts[i + 1].strip('\n')
    if head == 'header':
        d = d.replace('use super::*;\n', 'use super::*;\n' + body + '\n', 1)
        continue
    name, _, new = head.partition('->')
    name, new = name.strip(), new.strip()
    k = d.index('pub fn %s()' % name)
    j = d.index('APPLY', k)
    d = d[:j] + body.strip() + d[j + 5:]
    if new:
        d = d.replace('pub fn %s()' % name, 'pub fn %s()' % new, 1)
if 'APPLY' in d:
    print('WARNING: unported bodies remain')
pathlib.Path(out).write_text(d, encoding='utf-8')
print('wrote', out)
