// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        int dir = int(u[0]);
        float pos = u[1];
        vec2 s = uv;
        bool past;
        float lane;
        if (dir == 0) { past = uv.x > pos; lane = floor(uv.y * size.y / max(1.0, u[4] * 2.0)); }
        else if (dir == 1) { past = uv.x < pos; lane = floor(uv.y * size.y / max(1.0, u[4] * 2.0)); }
        else if (dir == 2) { past = uv.y > pos; lane = floor(uv.x * size.x / max(1.0, u[4] * 2.0)); }
        else { past = uv.y < pos; lane = floor(uv.x * size.x / max(1.0, u[4] * 2.0)); }
        if (past) {
            float j = (hash12(vec2(lane, u[3])) - 0.5) * u[2] * 0.2;
            if (dir < 2) s.x = clamp(pos + j, 0.0, 1.0);
            else s.y = clamp(pos + j, 0.0, 1.0);
        }
        imageStore(o, ivec2(gid), vec4(textureLod(sampler2D(a, sLin), s, 0.0).rgb, texelFetch(sampler2D(a, sNear), ivec2(gid), 0).a));
}
