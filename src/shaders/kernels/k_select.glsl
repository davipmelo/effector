// Highlight selection for streak filters.
// u: selectivity gamma (>=1: only the brightest survive), cell size px (0 = off), keep fraction, seed
// textures: a=1 (bright pass)
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec3 c = texelFetch(sampler2D(a, sNear), ivec2(gid), 0).rgb;
    float l = max(luma(c), 1e-5);
    float f = pow(clamp(l, 0.0, 1.0), max(u[0] - 1.0, 0.0));
    if (u[1] > 0.5) {
        vec2 cell = floor(vec2(gid) / u[1]);
        if (hash12(cell + vec2(u[3] * 17.13, u[3] * 3.7)) > u[2]) f = 0.0;
    }
    imageStore(o, ivec2(gid), vec4(c * f, 1.0));
}
