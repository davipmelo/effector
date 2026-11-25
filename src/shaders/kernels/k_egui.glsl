// Display copy for the UI: egui (0.29+) samples native textures as premultiplied sRGB-gamma
// values, so only premultiply the sRGB-encoded straight-alpha image (no linearisation, which
// would apply the gamma curve twice and darken the canvas).
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 uv = (vec2(gid) + 0.5) / vec2(imageSize(o));
    vec4 c = textureLod(sampler2D(a, sLin), uv, 0.0);
    float al = clamp(c.a, 0.0, 1.0);
    imageStore(o, ivec2(gid), vec4(clamp(c.rgb, 0.0, 1.0) * al, al));
}
