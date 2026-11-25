// Camcorder lens: barrel / fisheye distortion (wide converter), lateral chromatic aberration,
// soft corners and vignette. u: 0 barrel (-1..1), 1 CA, 2 vignette, 3 corner blur mix, 4 vignette roundness
// textures: a=1 soft=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D soft;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    vec2 asp = size / length(size) * 2.0;
    vec2 d = (uv - 0.5) * asp;                       // corner radius = 1
    float r2 = dot(d, d);
    float k = u[0];
    float norm = 1.0 + max(k, 0.0);                  // keep the corners inside the frame
    vec3 c, cs;
    for (int ch = 0; ch < 3; ch++) {
        float kk = k + (float(ch) - 1.0) * u[1] * 0.06;
        vec2 q = d * (1.0 + kk * r2) / norm;
        vec2 suv = q / asp + 0.5;
        c[ch] = textureLod(sampler2D(a, sMir), suv, 0.0)[ch];
        cs[ch] = textureLod(sampler2D(soft, sMir), suv, 0.0)[ch];
    }
    // corners go soft on cheap wide converters
    c = mix(c, cs, clamp(u[3] * smoothstep(0.35, 1.0, r2), 0.0, 1.0));
    float rr = mix(max(abs(d.x), abs(d.y)), sqrt(r2), u[4]);
    c *= 1.0 - u[2] * smoothstep(0.45, 1.05, rr);
    imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), texelFetch(sampler2D(a, sNear), ivec2(gid), 0).a));
}
