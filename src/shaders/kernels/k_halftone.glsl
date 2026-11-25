// textures: src=1
layout(set = 0, binding = 7) uniform texture2D src;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 p = vec2(gid) + 0.5;
        float cs = max(2.0, u[0]);
        int n = cs < 6.0 ? 4 : (cs < 16.0 ? 3 : 2);
        vec3 acc = vec3(0.0);
        for (int y = 0; y < n; y++) {
            for (int x = 0; x < n; x++) {
                vec2 off = (vec2(x, y) + 0.5) / float(n) - 0.5;
                acc += halftoneAt(p + off, src, size);
            }
        }
        imageStore(o, ivec2(gid), vec4(acc / float(n * n), texelFetch(sampler2D(src, sNear), ivec2(gid), 0).a));
}
