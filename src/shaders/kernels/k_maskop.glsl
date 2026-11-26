// Combines two masks (r channel). u: mode (0 add = max, 1 subtract, 2 intersect, 3 invert a)
// textures: a=1 b=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D b;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    float x = clamp(texelFetch(sampler2D(a, sNear), ivec2(gid), 0).r, 0.0, 1.0);
    float y = clamp(texelFetch(sampler2D(b, sNear), ivec2(gid), 0).r, 0.0, 1.0);
    int m = int(u[0] + 0.5);
    float r = m == 0 ? max(x, y) : m == 1 ? x * (1.0 - y) : m == 2 ? x * y : 1.0 - x;
    imageStore(o, ivec2(gid), vec4(r, r, r, 1.0));
}
