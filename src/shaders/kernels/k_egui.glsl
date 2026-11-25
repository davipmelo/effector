// Display copy for the UI: egui samples textures as premultiplied linear light, so convert the
// sRGB-encoded straight-alpha image (exact colours on screen after egui re-encodes them).
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 uv = (vec2(gid) + 0.5) / vec2(imageSize(o));
    vec4 c = textureLod(sampler2D(a, sLin), uv, 0.0);
    float al = clamp(c.a, 0.0, 1.0);
    imageStore(o, ivec2(gid), vec4(toLinear(clamp(c.rgb, 0.0, 1.0)) * al, al));
}
