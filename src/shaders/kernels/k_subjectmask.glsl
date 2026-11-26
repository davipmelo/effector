// Subject / background mask from the (softened) subject matte. u: threshold, half width, invert
// textures: m=1
layout(set = 0, binding = 7) uniform texture2D m;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    float x = texelFetch(sampler2D(m, sNear), ivec2(gid), 0).r;
    float v = smoothstep(u[0] - u[1], u[0] + u[1], x);
    if (u[2] > 0.5) v = 1.0 - v;
    imageStore(o, ivec2(gid), vec4(v, v, v, 1.0));
}
