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
        float ink = 1.0 - s.r;                        // dithered 0/1 coverage (white = paper)
        // print head density varies along the paper feed + overall fade
        float dens = 1.0 - u[8] * 0.6;
        dens -= (fbm(vec2(q.y / 220.0, 0.5), seed, 3) - 0.5) * u[10] * 0.7;
        dens -= uv.y * u[11] * 0.5;
        // dead heating elements leave thin white lines down the page
        float col = floor(q.x / 1.6);
        float dead = step(1.0 - u[9] * 0.02, hash12(vec2(col, seed + 3.0)));
        dens *= 1.0 - dead * 0.85;
        ink *= clamp(dens + (hash12(floor(q) + seed) - 0.5) * 0.15, 0.0, 1.0);
        vec3 c = mix(vec3(u[5], u[6], u[7]), vec3(u[2], u[3], u[4]), ink);
        imageStore(o, ivec2(gid), vec4(c, s.a));
}
