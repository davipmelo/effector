# Extracts the Metal shader sources embedded in the Swift app into one .metal file (reference for the port).
import re, sys, pathlib
root = pathlib.Path(sys.argv[1])
order = ["common","coreKernels","displayShaders","AdjustShaders","DitherShaders","HalftoneShaders","AsciiShaders",
 "GradientShaders","GrainShaders","GlowShaders","FilmShaders","TrailShaders","VHSShaders","BlobShaders",
 "TextureShaders","LensShaders","LensFilterShaders","DistortShaders","BlurShaders","CanvasShaders","DigitalShaders","CopyShaders","FrameShaders"]
blocks = {}
for f in root.rglob("*.swift"):
    t = f.read_text(encoding="utf-8")
    for m in re.finditer(r'enum (\w+Shaders) \{\s*static let source = """\n(.*?)\n\s*"""', t, re.S):
        blocks[m.group(1)] = m.group(2)
    if f.name == "ShaderCommon.swift":
        for name in ["common","coreKernels","displayShaders"]:
            m = re.search(r'static let %s = """\n(.*?)\n\s*"""' % name, t, re.S)
            blocks[name] = m.group(1)
out = []
for k in order:
    if k not in blocks: print("missing", k, file=sys.stderr); continue
    out.append("// ==== %s ====\n" % k + blocks[k])
src = "\n".join(out).replace(chr(92) * 2, chr(92))
if '\(' in src: print("WARNING: swift interpolation found", file=sys.stderr)
pathlib.Path(sys.argv[2]).write_text(src, encoding="utf-8")
print(len(src.splitlines()), "lines")
