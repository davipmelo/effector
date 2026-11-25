// textures: a=1 glow=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D glow;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        vec2 cc = uv * 2.0 - 1.0;
        float k = u[0];
        vec2 dc = cc * (1.0 + k * vec2(cc.y * cc.y, cc.x * cc.x) * 0.25);
        vec2 suv = dc * 0.5 + 0.5;
        float inside = step(0.0, suv.x) * step(suv.x, 1.0) * step(0.0, suv.y) * step(suv.y, 1.0);
        if (u[11] < 0.5) { inside = 1.0; }
        // rounded screen corners
        vec2 e = abs(dc);
        float cr = u[9];
        vec2 qq = max(e - (1.0 - cr), 0.0);
        float corner = u[11] < 0.5 ? 1.0 : 1.0 - smoothstep(cr - 0.01, cr + 0.005, length(qq));
        vec3 c = textureLod(sampler2D(a, sLin), suv, 0.0).rgb;
        c += textureLod(sampler2D(glow, sLin), suv, 0.0).rgb * u[6];
        vec2 px = vec2(gid);
        float ms = max(1.0, u[2]);
        int mt = int(u[1]);
        vec3 mask = vec3(1.0);
        if (mt == 1) {
            int i = int(floor(px.x / ms)) % 3;
            mask = i == 0 ? vec3(1, 0.25, 0.25) : (i == 1 ? vec3(0.25, 1, 0.25) : vec3(0.25, 0.25, 1));
        } else if (mt == 2) {
            float row = floor(px.y / ms);
            int i = int(floor(px.x / ms + fmodc(row, 2.0) * 1.5)) % 3;
            mask = i == 0 ? vec3(1, 0.3, 0.3) : (i == 1 ? vec3(0.3, 1, 0.3) : vec3(0.3, 0.3, 1));
            mask *= 0.75 + 0.25 * step(0.25, fract(px.y / ms));
        } else if (mt == 3) {
            float col = floor(px.x / ms);
            int i = int(col) % 3;
            mask = i == 0 ? vec3(1, 0.3, 0.3) : (i == 1 ? vec3(0.3, 1, 0.3) : vec3(0.3, 0.3, 1));
            float yy = fract((px.y + floor(col / 3.0) * ms * 1.5) / (ms * 3.0));
            mask *= 0.7 + 0.3 * step(0.12, yy);
        }
        c *= mix(vec3(1.0), mask * 1.6, u[3]);
        if (u[4] > 0.0) {
            float sp = max(2.0, u[5]);
            float ph = fract(suv.y * size.y / sp);
            float l = luma(c);
            float w = mix(0.5, 0.15, clamp(l, 0.0, 1.0));       // bright lines bloom wider
            float sl = smoothstep(0.0, w, ph) * smoothstep(1.0, 1.0 - w, ph);
            c *= mix(1.0, 0.35 + 0.9 * sl, u[4]);
        }
        c *= u[7];
        c *= 1.0 - u[8] * smoothstep(0.4, 1.4, length(cc));
        c += (hash12(px + u[10] * 13.0) - 0.5) * u[10] * 0.06;
        c = mix(vec3(0.02), c, inside * corner);
        imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), texelFetch(sampler2D(a, sNear), ivec2(gid), 0).a));
}
