// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;

        if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
        vec2 size = vec2(imageSize(o));
        vec2 uv = ((vec2(gid) + 0.5) / vec2(imageSize(o))) - 0.5;
        float asp = size.x / size.y;
        vec2 q = uv * mix(vec2(2.0), vec2(asp, 1.0) / max(asp, 1.0) * 2.0, u[2]);
        float d = length(q);
        float mid = mix(0.3, 1.3, u[1]);
        float f = smoothstep(mid - u[3], mid + u[3] * 0.6, d);
        vec4 s = texelFetch(sampler2D(a, sNear), ivec2(gid), 0);
        vec3 c = s.rgb;
        float amt = u[0];
        if (amt >= 0.0) {
            float protect = 1.0 - u[7] * smoothstep(0.7, 1.0, luma(c));
            c = mix(c, c * vec3(u[4], u[5], u[6]), f * amt * protect);
        } else {
            c = mix(c, 1.0 - (1.0 - c) * 0.25, f * -amt);
        }
        imageStore(o, ivec2(gid), vec4(clamp(c, 0.0, 1.0), s.a));
}
