// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 c0 = clamp(s.rgb, 0.0, 1.0);
        vec3 c = toSRGB(toLinear(c0) * exp2(u[18]));
        float warm = u[19] / 100.0;
        c = clamp(c * vec3(1.0 + warm * 0.06, 1.0, 1.0 - warm * 0.08), 0.0, 1.0);
        if (u[0] > 0.5) {
            vec3 w = vec3(u[14], u[15], u[16]);
            float g = dot(toLinear(c), w / max(1e-4, w.x + w.y + w.z));
            g = toSRGB(vec3(g)).x;
            g = sCurve(g, u[2]);
            g = u[3] + g * (u[4] - u[3]);
            float base = neutralGray(c0);
            g = mix(base, g, u[17]);
            imageStore(o, ivec2(gid), vec4(g, g, g, s.a));   // exactly neutral
            return;
        }
        mat3 mx = mat3(vec3(u[20], u[23], u[26]), vec3(u[21], u[24], u[27]), vec3(u[22], u[25], u[28]));
        c = clamp(mx * c, 0.0, 1.0);
        float l = luma(c);
        c = mix(vec3(l), c, u[1]);
        c = pow(clamp(c, 0.0, 1.0), vec3(u[5], u[6], u[7]));
        c = vec3(sCurve(c.r, u[2]), sCurve(c.g, u[2]), sCurve(c.b, u[2]));
        float l2 = luma(c);
        c += vec3(u[8], u[9], u[10]) * (1.0 - l2) * (1.0 - l2) * 3.0;
        c += vec3(u[11], u[12], u[13]) * l2 * l2 * 2.0;
        c = u[3] + clamp(c, 0.0, 1.0) * (u[4] - u[3]);
        imageStore(o, ivec2(gid), vec4(mix(c0, clamp(c, 0.0, 1.0), u[17]), s.a));
}
