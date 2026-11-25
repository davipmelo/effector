# One-off porting helper: small textual fixes on the generated drafts.
import re, pathlib
for f in ['tools/draft/vhs.rs', 'tools/draft/digital.rs']:
    p = pathlib.Path(f)
    s = p.read_text(encoding='utf-8')
    s = re.sub(r'/\*TODO\*/ ParamValue\.text\(("(?:[^"\\]|\\.)*")\)', r'ParamValue::Text(\1.into())', s)
    p.write_text(s, encoding='utf-8')
for f in ['tools/draft/frame.rs', 'src/effects/frame.rs']:
    p = pathlib.Path(f)
    s = p.read_text(encoding='utf-8')
    s = s.replace('FrameEffect.ratios.map { ChoiceOption(id: $0.0, label: $0.1) }/*TODO*/', 'FRAME_RATIOS.iter().map(|r| opt(r.0, r.1)).collect()')
    p.write_text(s, encoding='utf-8')
