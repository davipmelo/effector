// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        float asp = size.x / size.y;
        float seed = u[0];
        int edge = int(u[2]);
        float d;
        if (edge == 0) d = uv.x * asp;
        else if (edge == 1) d = (1.0 - uv.x) * asp;
        else if (edge == 2) d = uv.y;
        else if (edge == 3) d = 1.0 - uv.y;
        else if (edge == 4) d = length(uv * vec2(asp, 1.0)) * 0.75;
        else if (edge == 5) d = length((1.0 - uv) * vec2(asp, 1.0)) * 0.75;
        else {
            vec2 c = vec2(hash12(vec2(seed, 4.0)), hash12(vec2(seed, 5.0)));
            d = length((uv - c) * vec2(asp, 1.0)) * 0.9;
        }
        vec2 nq = uv * vec2(asp, 1.0) * 2.2;
        d += (fbm(nq, seed, 4) - 0.5) * u[11];
        float w = exp(-max(d, 0.0) / max(0.02, u[3]) * 2.5);
        float t = smoothstep(0.25, 0.75, fbm(nq * 0.8 + 7.0, seed + 3.0, 3));
        vec3 col = mix(vec3(u[5], u[6], u[7]), vec3(u[8], u[9], u[10]), t) * w;
        col += vec3(1.0, 0.96, 0.88) * pow(w, 3.0) * 0.45;
        col *= u[4];
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 b = clamp(s.rgb, 0.0, 1.0);
        vec3 r;
        int m = int(u[12]);
        if (m == 1) r = b + col;
        else if (m == 2) r = max(b, col);
        else r = 1.0 - (1.0 - b) * (1.0 - clamp(col, 0.0, 1.0));
        imageStore(o, ivec2(gid), vec4(clamp(r, 0.0, 1.0), s.a));
}
