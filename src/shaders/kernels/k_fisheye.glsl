// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        float asp = size.x / size.y;
        vec2 c = vec2(u[4], u[5]);
        vec2 d = (((vec2(gid) + 0.5) / vec2(imageSize(o))) - c) * vec2(asp, 1.0);
        float r = length(d);
        float lim = 0.5 * min(asp, 1.0);
        vec2 dir = r > 1e-6 ? d / r : vec2(0.0);
        vec3 col;
        bool outside = false;
        for (int ch = 0; ch < 3; ch++) {
            float spread = 1.0 + (float(ch) - 1.0) * u[3];
            float sr;
            if (u[6] > 0.5) {
                // true fisheye: the radius maps to a view angle up to fov/2 (stereographic, so it
                // grows smoothly); past ~180° the photo wraps around like a mirror ball
                float th = (r / lim) * u[7] * 0.5 * spread / max(0.2, u[1]);
                float srcHalf = 0.9;              // assumed half field of view of the photo (~52°)
                sr = tan(clamp(th * 0.5, -1.45, 1.45)) / tan(srcHalf * 0.5) * lim * 0.5;
            } else {
                float k = u[0] * spread;
                float r2 = r * r;
                sr = r * (1.0 + k * r2 + k * 0.5 * r2 * r2) / max(0.05, u[1]);
            }
            vec2 suv = c + dir * sr / vec2(asp, 1.0);
            if (any(lessThan(suv, vec2(0.0))) || any(greaterThan(suv, vec2(1.0)))) outside = true;
            col[ch] = textureLod(sampler2D(a, sMir), suv, 0.0)[ch];
        }
        if (outside && u[8] < 0.5 && u[2] < 0.5) col = vec3(0.0);
        if (u[2] > 0.5) col *= 1.0 - smoothstep(lim * 0.99, lim, r);
        imageStore(o, ivec2(gid), vec4(col, texelFetch(sampler2D(a, sNear), ivec2(gid), 0).a));
}
