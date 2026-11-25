// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 c = s.rgb;
        vec2 p = vec2(gid);
        float l = clamp(luma(c), 0.0, 1.0);
        float w = u[4] * (1.0 - l) * (1.0 - l) * 1.6 + u[5] * 4.0 * l * (1.0 - l) + u[6] * l * l * 1.6;
        float amt = u[0] * 0.22 * w;
        if (u[3] > 0.5) {
            float n0 = grainNoise(p, u[1], u[2], u[7]);
            vec3 n = vec3(grainNoise(p, u[1], u[2], u[7] + 31.0), grainNoise(p, u[1], u[2], u[7] + 57.0), grainNoise(p, u[1], u[2], u[7] + 83.0));
            n = mix(vec3(n0), n, u[8]);
            c += n * amt;
        } else {
            c += grainNoise(p, u[1], u[2], u[7]) * amt;
        }
        imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), s.a));
}
