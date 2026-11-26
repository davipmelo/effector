// Display copy with Lightroom-style clipping warnings: blown highlights (any channel at 255) in
// red, crushed shadows (every channel at 0) in blue. Same output format as k_egui.
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 uv = (vec2(gid) + 0.5) / vec2(imageSize(o));
    vec4 c = textureLod(sampler2D(a, sLin), uv, 0.0);
    vec3 rgb = clamp(c.rgb, 0.0, 1.0);
    float hi = max(rgb.r, max(rgb.g, rgb.b));
    if (hi >= 254.5 / 255.0) {
        rgb = vec3(1.0, 0.0, 0.0);
    } else if (hi <= 0.5 / 255.0) {
        rgb = vec3(0.1, 0.35, 1.0);
    }
    float al = clamp(c.a, 0.0, 1.0);
    imageStore(o, ivec2(gid), vec4(rgb * al, al));
}
