// textures: grid=1 atlas=2
layout(set = 0, binding = 7) uniform texture2D grid;
layout(set = 0, binding = 8) uniform texture2D atlas;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 p = vec2(gid) + 0.5;
        vec2 cs = vec2(u[0], u[1]);
        vec2 cellF = p / cs;
        uvec2 cell = uvec2(floor(cellF));
        cell = min(cell, uvec2(uint(textureSize(sampler2D(grid, sNear), 0).x) - 1, uint(textureSize(sampler2D(grid, sNear), 0).y) - 1));
        vec2 local = fract(cellF);
        vec4 c = texelFetch(sampler2D(grid, sNear), ivec2(cell), 0);
        float l = neutralGray(clamp(c.rgb, 0.0, 1.0));
        l = clamp((l - 0.5) * (1.0 + u[11] / 100.0) + 0.5 + u[12] / 100.0, 0.0, 1.0);
        if (u[10] > 0.5) l = 1.0 - l;
        float n = max(1.0, u[2]);
        float idx = min(n - 1.0, floor(l * n));
        float ax = (idx + local.x) / n;
        float g = textureLod(sampler2D(atlas, sLin), vec2(ax, local.y), 0.0).r;
        vec3 fg = vec3(u[4], u[5], u[6]);
        vec3 bg = vec3(u[7], u[8], u[9]);
        int mode = int(u[3]);
        if (mode == 1) {
            int pc = int(ex[0]);
            int pi = clamp(int(floor(l * float(pc))), 0, pc - 1);
            fg = vec3(ex[1 + pi * 3], ex[2 + pi * 3], ex[3 + pi * 3]);
        } else if (mode == 2) {
            fg = clamp(c.rgb * 1.15, 0.0, 1.0);
        }
        imageStore(o, ivec2(gid), vec4(mix(bg, fg, g), c.a));
}
