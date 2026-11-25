// Interlaced video: the two fields come from different instants, so odd lines are shifted (combing).
// u: 0 shift px, 1 field darkening, 2 jitter, 3 seed, 4 line height px
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    float line = floor(float(gid.y) / max(u[4], 1.0));
    bool odd = mod(line, 2.0) > 0.5;
    vec4 c = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    if (odd) {
        float j = (vnoise(vec2(line * 0.05, u[3]), u[3]) - 0.5) * 2.0 * u[2];
        c = textureLod(sampler2D(a, sLin), uv + vec2((u[0] + j) / size.x, 0.0), 0.0);
        c.rgb *= 1.0 - u[1];
    }
    imageStore(o, ivec2(gid), c);
}
