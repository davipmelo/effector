// Mask overlay for the UI (Masks tool): translucent red where the mask selects, premultiplied
// for egui; areas cut out by Subtract parts get light diagonal stripes.
// u: opacity, stripes on (0/1)
// textures: mask=1 sub=2
layout(set = 0, binding = 7) uniform texture2D a;
layout(set = 0, binding = 8) uniform texture2D sub;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    float m = clamp(texelFetch(sampler2D(a, sNear), ivec2(gid), 0).r, 0.0, 1.0);
    vec3 c = vec3(1.0, 0.16, 0.16) * m * u[0];
    float al = m * u[0];
    if (u[1] > 0.5) {
        float s = clamp(texelFetch(sampler2D(sub, sNear), ivec2(gid), 0).r, 0.0, 1.0);
        float stripe = step(0.5, fract((float(gid.x) + float(gid.y)) / 12.0));
        float h = s * stripe * 0.55;
        c = c * (1.0 - h) + vec3(1.0) * h;
        al = al * (1.0 - h) + h;
    }
    imageStore(o, ivec2(gid), vec4(c, al));
}
