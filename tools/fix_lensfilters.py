# One-off porting helper: fixes the prism params that had inline comments in Swift.
import pathlib
p = pathlib.Path('tools/draft/lensfilters.rs')
s = p.read_text(encoding='utf-8')
rep = {
    '''        /*TODO param*/ // prism ghosts
        .choice("mode", "Placement", [("side", "One Side"), ("ring", "Around the Edges"), ("full", "Whole Frame")], "side",
                section: "Prism", visible: when("type", "ghosts")),''':
    '''        // prism ghosts
        ParamDef::choice("mode", "Placement", &[("side", "One Side"), ("ring", "Around the Edges"), ("full", "Whole Frame")], "side")
            .sec("Prism")
            .vis(when("type", "ghosts")),''',
    '''        /*TODO param*/ // kaleidoscope
        .slider("segments", "Segments", 2, 32, 6, step: 1, section: "Kaleidoscope", visible: when("type", "kaleido")),''':
    '''        // kaleidoscope
        s("segments", "Segments", 2.0, 32.0, 6.0).step(1.0).sec("Kaleidoscope").vis(when("type", "kaleido")),''',
    '''        /*TODO param*/ // facets
        .slider("cell", "Facet Size", 10, 400, 90, step: 1, curve: 1.6, section: "Crystal", visible: when("type", "facets")),''':
    '''        // facets
        s("cell", "Facet Size", 10.0, 400.0, 90.0).step(1.0).curve(1.6).sec("Crystal").vis(when("type", "facets")),''',
    '''        /*TODO param*/ // shared
        .slider("dispersion", "Rainbow Dispersion", 0, 100, 35, step: 1, unit: "%", section: "Color & Mix"),''':
    '''        // shared
        s("dispersion", "Rainbow Dispersion", 0.0, 100.0, 35.0).step(1.0).unit("%").sec("Color & Mix"),''',
}
for a, b in rep.items():
    assert a in s, a[:60]
    s = s.replace(a, b)
ghost_vis = 'Arc::new(|p: &Params| p.choice_or("type", "ghosts") == "ghosts" && p.choice_or("mode", "side") != "full")'
import re
s = re.sub(r'Arc::new\(\|p: &Params\| /\*TODO\*/ \{ let _ = p; true \}\) /\* \{ \$0\.choice\("type", "ghosts"\) == "ghosts" && \$0\.choice\("mode", "side"\) != "full" \} \*/', ghost_vis, s)
assert 'TODO' not in s, [l for l in s.split('\n') if 'TODO' in l]
p.write_text(s, encoding='utf-8')
