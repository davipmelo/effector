// textures: img=1 bg=2
layout(set = 0, binding = 7) uniform texture2D img;
layout(set = 0, binding = 8) uniform texture2D bg;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 p = vec2(gid) + 0.5;
        vec2 os = vec2(imageSize(o));
        vec2 r0 = vec2(u[0], u[1]), rs = vec2(u[2], u[3]);
        float rad = u[4];
        vec2 c = r0 + rs * 0.5;
        vec2 q = abs(p - c) - (rs * 0.5 - rad);
        float sd = length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - rad;
        vec3 back;
        int fill = int(u[5]);
        if (fill == 1) back = textureLod(sampler2D(bg, sLin), p / os, 0.0).rgb * (1.0 - u[14]);
        else if (fill == 2) back = mix(vec3(u[6], u[7], u[8]), vec3(u[9], u[10], u[11]), p.y / os.y);
        else back = vec3(u[6], u[7], u[8]);
        // soft drop shadow under the photo
        float sh = u[12] * 0.6 * exp(-max(sd - u[13] * 0.15, 0.0) / max(1.0, u[13]));
        back *= 1.0 - sh;
        float inside = 1.0 - smoothstep(-0.75, 0.75, sd);
        vec2 iuv = (p - r0) / rs;
        vec4 s = textureLod(sampler2D(img, sLin), iuv, 0.0);
        vec3 col = mix(back, s.rgb, inside * s.a);
        imageStore(o, ivec2(gid), vec4(col, 1.0));
}
