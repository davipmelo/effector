// Split toning: soft-light tint of the shadows and the highlights.
// u: shadow rgb, highlight rgb, shadow amount, highlight amount, balance (-1..1), preserve brightness
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    vec3 c = clamp(s.rgb, 0.0, 1.0);
    float l = luma(c);
    float pivot = clamp(0.5 + u[10] * 0.35, 0.1, 0.9);
    float ws = 1.0 - smoothstep(0.0, pivot * 1.6, l);
    float wh = smoothstep(pivot * 0.6, 1.0, l);
    vec3 r = c;
    r = mix(r, blendSoftLight(r, vec3(u[0], u[1], u[2])), clamp(u[6] * ws * 2.2, 0.0, 1.0));
    r = mix(r, blendSoftLight(r, vec3(u[3], u[4], u[5])), clamp(u[7] * wh * 2.2, 0.0, 1.0));
    if (u[11] > 0.5) r = clamp(r + (l - luma(r)), 0.0, 1.0);
    imageStore(o, ivec2(gid), vec4(r, s.a));
}
