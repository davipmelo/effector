// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 p = vec2(gid) + 0.5;
        vec2 r0 = vec2(u[0], u[1]), rs = vec2(u[2], u[3]);
        float rad = u[4];
        vec2 c = r0 + rs * 0.5;
        vec2 q = abs(p - c) - (rs * 0.5 - rad);
        float sd = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - rad;   // signed distance to the window
        float inside = 1.0 - smoothstep(-0.75, 0.75, sd);
        float unit = max(u[11], 1e-3);
        vec3 frame = vec3(u[5], u[6], u[7]);
        vec2 tq = p / unit;
        frame *= 1.0 + ((fbm(tq / 1.3, u[10], 3) - 0.5) * 0.08 + (fbm(tq / 90.0, u[10] + 4.0, 3) - 0.5) * 0.05) * u[8];
        vec2 luv = (p - r0) / rs;
        vec2 suv = vec2(u[12], u[13]) + luv * vec2(u[14], u[15]);
        vec3 img = textureLod(sampler2D(a, sLin), suv, 0.0).rgb;
        // the print sits slightly below the frame: soft shadow inside the window edge
        float sh = exp(-max(-sd, 0.0) / (unit * 4.0)) * u[9] * 0.45;
        img *= 1.0 - sh;
        imageStore(o, ivec2(gid), vec4(mix(frame, img, inside), texelFetch(sampler2D(a, sNear), ivec2(gid), 0).a));
}
