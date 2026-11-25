// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        float t = ditherThreshold(gid, int(u[0]), u[4]);
        float spread = u[3];
        vec3 outc;
        if (u[1] > 0.5) {
            float L = max(2.0, u[2]) - 1.0;
            float v = s.r * L;
            float q = floor(v + mix(0.5, t, spread));
            q = clamp(q / L, 0.0, 1.0);
            outc = mix(vec3(u[5], u[6], u[7]), vec3(u[8], u[9], u[10]), q);
        } else {
            int n = int(ex[0]);
            float r = spread * 0.5 / max(1.0, pow(float(n), 1.0 / 3.0) - 0.5);
            vec3 c = s.rgb + (t - 0.5) * r;
            outc = nearestPalette(c, n);
        }
        imageStore(o, ivec2(gid), vec4(outc, s.a));
}
