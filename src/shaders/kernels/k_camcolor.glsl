// Camcorder colour science: NightShot infrared, colour casts, skin/yellow push, cool shadows,
// black level and gamma. u: 0 nightshot, 1-3 IR tint, 4 black lift, 5 yellow push, 6 cool shadows,
// 7 saturation, 8 gamma, 9 tint r, 10 tint g, 11 tint b (multipliers)
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    vec3 c = clamp(s.rgb, 0.0, 1.0);
    c *= vec3(u[9], u[10], u[11]);
    float l = luma(c);
    // warm subjects (skin, yellow light) pushed: the classic CCD "yellow faces"
    float warm = smoothstep(0.02, 0.25, c.r - c.b) * smoothstep(0.1, 0.5, l);
    c += vec3(0.07, 0.045, -0.05) * warm * u[5];
    float sh = 1.0 - smoothstep(0.0, 0.45, l);
    c += vec3(-0.02, 0.005, 0.05) * sh * u[6];
    c = mix(vec3(luma(c)), c, u[7]);
    // NightShot: the IR filter flips out, everything glows green-cyan, skin and fabric go pale
    if (u[0] > 0.001) {
        float ir = pow(clamp(dot(c, vec3(0.45, 0.4, 0.15)) * 1.25, 0.0, 1.0), 0.75);
        vec3 tinted = mix(vec3(0.0), vec3(u[1], u[2], u[3]), ir) + vec3(ir * ir * 0.35);
        c = mix(c, tinted, u[0]);
    }
    c = pow(clamp(c, 0.0, 1.0), vec3(1.0 / max(u[8], 0.2)));
    c = c * (1.0 - u[4]) + u[4];
    imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), s.a));
}
