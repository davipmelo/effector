// Reeded (fluted) glass. Each rib is a cylindrical lens: the slice behind it is minified and
// repeated, strongly toward the rib edges (refraction grows with the surface slope), the scene is
// smeared across the ribs (pre-blurred by the effect), and every rib catches light on one side
// and has a thin dark groove at its edges.
// u: 0 rib width px, 1 refraction, 2 angle°, 3 dispersion, 4 shading, 5 melt, 6 melt scale px, 7 seed,
//    8 profile (0 round, 1 concave, 2 prism, 3 wave), 9 highlight, 10 groove, 11 irregular widths,
//    12 depth-aware amount (0 = off)
// textures: a=1 (pre-blurred scene) depth=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D depth;

float profile(float t, int kind) {
    // t: -0.5..0.5 across the rib -> displacement (units of rib width)
    if (kind == 2) return t * 2.0;                                       // V prism: linear, sharp seams
    if (kind == 3) return sin(t * 6.2831853) * 0.5;                      // soft wave
    float s = clamp(t * 2.0, -0.985, 0.985);                             // round: slope of a half-cylinder
    float th = asin(s);
    float d = tan(th * 0.55) / tan(1.42 * 0.55) * 0.9;
    return kind == 1 ? -d : d;
}

void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    vec2 size = vec2(imageSize(o));
    if (gid.x >= uint(size.x) || gid.y >= uint(size.y)) return;
    vec2 p = vec2(gid) + 0.5;
    float W = max(u[0], 1.0);
    float ang = radians(u[2]);
    vec2 across = vec2(cos(ang), sin(ang));
    vec2 along = vec2(-across.y, across.x);
    vec2 c = size * 0.5;
    float x = dot(p - c, across);
    float y = dot(p - c, along);
    // irregular rib widths: warp x slowly so neighbouring ribs differ a little
    float xw = x + u[11] * W * 0.45 * (vnoise(vec2(x / (W * 3.7), 0.5), u[7] + 3.0) - 0.5) * 2.0;
    float rib = floor(xw / W);
    float t = xw / W - rib - 0.5;
    int kind = int(u[8]);
    float prof = profile(t, kind);
    float melt = u[5] * W * 0.6 * (vnoise(vec2(rib * 1.37, y / max(u[6], 1.0)), u[7]) - 0.5) * 2.0;
    float k = u[1];
    if (u[12] > 0.001) {
        float d = textureLod(sampler2D(depth, sLin), p / size, 0.0).r;      // 1 = near the glass
        k *= mix(1.0, 0.15 + 1.1 * (1.0 - d), u[12]);
    }
    // dispersion grows toward the rib edges, where the glass is steepest
    float disp = u[3] * 0.35 * (0.35 + abs(t) * 1.6);
    vec3 col;
    for (int i = 0; i < 3; i++) {
        float kk = k * (1.0 + (float(i) - 1.0) * disp);
        float xs = x - prof * W * kk * 1.6 + melt;
        vec2 q = c + across * xs + along * y;
        col[i] = textureLod(sampler2D(a, sMir), q / size, 0.0)[i];
    }
    float alpha = textureLod(sampler2D(a, sLin), p / size, 0.0).a;
    // light: the curved face is brighter on the lit side, a narrow specular line, dark grooves
    float slope = kind == 2 ? sign(t) * 0.6 : clamp(t * 2.0, -1.0, 1.0);
    float lit = 1.0 + u[4] * 0.22 * (-slope);
    float spec = exp(-pow((t + 0.28) / 0.05, 2.0)) * u[9];
    float groove = smoothstep(0.38, 0.5, abs(t)) * u[10];
    col = col * lit * (1.0 - groove * 0.6) + spec * (0.25 + 0.5 * luma(col));
    imageStore(o, ivec2(gid), vec4(clamp(col, 0.0, 1.0), alpha));
}
