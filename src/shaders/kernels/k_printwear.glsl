// Print wear over any image: ink density loss (fade, uneven, down the page, banding, dead lines)
// pulls towards the paper colour; streaks, edge shadow and specks add toner.
// u: seed, unit, fade, uneven, roll, streaks, banding, dead, noise, specks, edge, paper rgb, toner rgb
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    vec2 size = vec2(imageSize(o));
    if (gid.x >= uint(size.x) || gid.y >= uint(size.y)) return;
    vec2 p = vec2(gid) + 0.5;
    float unit = max(u[1], 1e-3);
    vec2 q = p / unit;
    vec2 uv = p / size;
    float seed = u[0];
    vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    vec3 paper = vec3(u[11], u[12], u[13]);
    vec3 toner = vec3(u[14], u[15], u[16]);
    // ink density
    float dens = 1.0 - u[2] * 0.6;
    dens -= (fbm(q / 180.0, seed, 3) - 0.5) * u[3] * 0.8;
    dens -= uv.y * u[4] * 0.55;
    dens -= (fbm(vec2(q.y / 160.0, 0.5), seed + 4.0, 3) - 0.45) * u[6] * 0.9;
    float col = floor(q.x / 1.6);
    float dead = step(1.0 - u[7] * 0.02, hash12(vec2(col, seed + 3.0)));
    dens *= 1.0 - dead * 0.9;
    dens += (hash12(floor(q * 1.2) + seed) - 0.5) * u[8] * 0.5;
    vec3 c = mix(paper, clamp(s.rgb, 0.0, 1.0), clamp(dens, 0.0, 1.0));
    // added toner
    float scol = floor(q.x / 2.0);
    float streak = step(0.985, hash12(vec2(scol, seed + 7.0))) * 0.6;
    streak *= 0.6 + 0.4 * vnoise(vec2(scol * 0.1, q.y / 300.0), seed + 3.0);
    float lighten = step(0.992, hash12(vec2(scol, seed + 9.0))) * 0.5 * u[5];
    vec2 e = min(uv, 1.0 - uv) * size / unit;
    float edge = 1.0 - smoothstep(0.0, 70.0, min(e.x, e.y) + (fbm(q / 25.0, seed + 5.0, 3) - 0.5) * 40.0);
    float speck = step(1.0 - u[9] * 0.012, hash12(floor(q / 1.5) + seed * 3.1)) * 0.85;
    float add = clamp(streak * u[5] + edge * u[10] * 0.8 + speck, 0.0, 1.0);
    c = mix(c, toner, add);
    c = mix(c, paper, lighten);
    imageStore(o, ivec2(gid), vec4(c, s.a));
}
