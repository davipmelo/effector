// Reeded (fluted) glass: each rib is a cylindrical lens that magnifies / flips the strip behind it.
// u: rib width px, refraction, angle°, dispersion, shading, melt, melt scale px, seed, curvature
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    vec2 size = vec2(imageSize(o));
    if (gid.x >= uint(size.x) || gid.y >= uint(size.y)) return;
    vec2 p = vec2(gid) + 0.5;
    float W = max(u[0], 1.0);
    float ang = radians(u[2]);
    vec2 across = vec2(cos(ang), sin(ang));      // perpendicular to the ribs
    vec2 along = vec2(-across.y, across.x);
    vec2 c = size * 0.5;
    float x = dot(p - c, across);
    float y = dot(p - c, along);
    float rib = floor(x / W);
    float t = x / W - rib - 0.5;                  // -0.5..0.5 inside the rib
    // lens profile: linear magnification bent by curvature (stronger near the rib edges)
    float prof = mix(t, sin(t * 3.14159265) * 0.5, clamp(u[8], 0.0, 1.0));
    // melt: each rib wobbles along its length
    float melt = u[5] * W * 0.6 * (vnoise(vec2(rib * 1.37, y / max(u[6], 1.0)), u[7]) - 0.5) * 2.0;
    float k = u[1];
    float disp = u[3] * 0.35;
    vec3 col;
    for (int i = 0; i < 3; i++) {
        float kk = k * (1.0 + (float(i) - 1.0) * disp);
        float xs = x - prof * W * kk + melt;
        vec2 q = c + across * xs + along * y;
        vec4 v = textureLod(sampler2D(a, sMir), q / size, 0.0);
        col[i] = v[i];
    }
    float alpha = textureLod(sampler2D(a, sLin), p / size, 0.0).a;
    // rib shading: darker seams, a soft highlight on one side
    float seam = pow(abs(t) * 2.0, 6.0);
    float hl = exp(-pow((t + 0.22) / 0.12, 2.0));
    col = col * (1.0 - u[4] * 0.55 * seam) + u[4] * 0.18 * hl;
    imageStore(o, ivec2(gid), vec4(clamp(col, 0.0, 1.0), alpha));
}
