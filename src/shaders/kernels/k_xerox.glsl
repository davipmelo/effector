// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 p = vec2(gid) + 0.5;
        float unit = max(u[1], 1e-3);
        vec2 q = p / unit;
        vec2 uv = p / size;
        float seed = u[0];
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        float d = 1.0 - neutralGray(clamp(s.rgb, 0.0, 1.0));     // darkness
        // uneven toner density across the page
        d += (fbm(q / 180.0, seed, 3) - 0.5) * u[7] * 0.5;
        // drum streaks (vertical)
        float col = floor(q.x / 2.0);
        float st = hash12(vec2(col, seed + 7.0));
        float streak = (step(0.985, st) * 0.6 - step(0.992, hash12(vec2(col, seed + 9.0))) * 0.5) * u[5];
        streak *= 0.6 + 0.4 * vnoise(vec2(col * 0.1, q.y / 300.0), seed + 3.0);
        d += streak;
        // shadow along the page edges (lid not closed)
        vec2 e = min(uv, 1.0 - uv) * size / unit;
        float edge = 1.0 - smoothstep(0.0, 70.0, min(e.x, e.y) + (fbm(q / 25.0, seed + 5.0, 3) - 0.5) * 40.0);
        d += edge * u[6];
        // toner noise
        d += (hash12(floor(q * 1.2) + seed) - 0.5) * u[4] * 0.6;
        float th = u[2];
        float sft = max(0.003, u[3]);
        float cov;
        if (u[15] > 0.5) {
            float ign = fract(52.9829189 * fract(0.06711056 * floor(q.x) + 0.00583715 * floor(q.y)));
            cov = step(ign, smoothstep(th - sft * 3.0, th + sft * 3.0, d));
        } else {
            cov = smoothstep(th - sft, th + sft, d);
        }
        // background specks
        float bg = step(1.0 - u[14] * 0.012, hash12(floor(q / 1.5) + seed * 3.1));
        cov = max(cov, bg * 0.85);
        vec3 toner = vec3(u[8], u[9], u[10]), paper = vec3(u[11], u[12], u[13]);
        imageStore(o, ivec2(gid), vec4(mix(paper, toner, clamp(cov, 0.0, 1.0)), s.a));
}
