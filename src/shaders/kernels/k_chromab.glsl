// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o)));
        vec2 off;
        if (u[0] < 0.5) {
            vec2 d = (uv - vec2(u[4], u[5])) * vec2(size.x / max(size.x, size.y), size.y / max(size.x, size.y));
            float r = length(d) * 1.414;
            off = normalize(d + 1e-6) * pow(r, max(0.2, u[3])) * u[1];
        } else {
            float an = u[2] * 0.0174533;
            off = vec2(cos(an), -sin(an)) * u[1];
        }
        off /= size;
        vec4 c = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        float r = textureLod(sampler2D(a, sLin), uv + off, 0.0).r;
        float b = textureLod(sampler2D(a, sLin), uv - off, 0.0).b;
        imageStore(o, ivec2(gid), vec4(r, c.g, b, c.a));
}
