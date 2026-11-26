// Mask overlay for the UI (Masks tool): translucent red where the mask selects, premultiplied
// for egui. u: opacity
// textures: mask=1
layout(set = 0, binding = 7) uniform texture2D a;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    float m = clamp(texelFetch(sampler2D(a, sNear), ivec2(gid), 0).r, 0.0, 1.0);
    float al = m * u[0];
    imageStore(o, ivec2(gid), vec4(vec3(1.0, 0.16, 0.16) * al, al));
}
