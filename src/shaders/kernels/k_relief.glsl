// Depth tricks. mode 0: parallax (foreground shifts against the background), 1: relief (the depth
// map lit like an embossed surface), 2: depth colour (near / far tint).
// u: 0 mode, 1 amount, 2 angle deg, 3 pivot depth, 4 light angle deg, 5 relief strength, 6-8 near colour,
//    9-11 far colour, 12 blend
// textures: a=1 depth=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D depth;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec2 size = vec2(imageSize(o));
    vec2 uv = (vec2(gid) + 0.5) / size;
    vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
    int mode = int(u[0]);
    vec3 c = s.rgb;
    if (mode == 0) {
        float an = u[2] * 0.0174533;
        vec2 dir = vec2(cos(an), -sin(an)) * u[1] / size;
        vec2 suv = uv;
        for (int k = 0; k < 6; k++) {
            float ds = textureLod(sampler2D(depth, sLin), suv, 0.0).r;
            suv = uv - dir * (ds - u[3]);
        }
        c = textureLod(sampler2D(a, sMir), suv, 0.0).rgb;
    } else if (mode == 1) {
        vec2 px = 1.5 / size;
        float dl = textureLod(sampler2D(depth, sLin), uv - vec2(px.x, 0.0), 0.0).r;
        float dr = textureLod(sampler2D(depth, sLin), uv + vec2(px.x, 0.0), 0.0).r;
        float du = textureLod(sampler2D(depth, sLin), uv - vec2(0.0, px.y), 0.0).r;
        float dd = textureLod(sampler2D(depth, sLin), uv + vec2(0.0, px.y), 0.0).r;
        vec3 n = normalize(vec3((dl - dr) * u[5] * 60.0, (du - dd) * u[5] * 60.0, 1.0));
        float la = u[4] * 0.0174533;
        vec3 L = normalize(vec3(cos(la), -sin(la), 0.9));
        float sh = clamp(dot(n, L), 0.0, 1.0);
        vec3 lit = c * (0.35 + sh * 0.9) + pow(sh, 24.0) * 0.25;
        c = mix(c, lit, clamp(u[1] / 100.0, 0.0, 1.0));
    } else {
        float d = texelFetch(sampler2D(depth, sNear), ivec2(gid), 0).r;
        vec3 t = mix(vec3(u[9], u[10], u[11]), vec3(u[6], u[7], u[8]), d);
        vec3 r = int(u[12]) == 1 ? blendSoftLight(clamp(c, 0.0, 1.0), t) : (int(u[12]) == 2 ? c * t * 1.6 : t);
        c = mix(c, r, clamp(u[1] / 100.0, 0.0, 1.0));
    }
    imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), s.a));
}
