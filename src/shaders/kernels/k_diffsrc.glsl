// Diffusion filter, step 1: the light a diffusion filter scatters, in linear light.
// Each pixel gives away a share `s` of its light; `s` leans toward the highlights by u[0]
// (0 = every pixel scatters the same, 1 = mostly the highlights, like a Black Pro-Mist).
// u: highlight bias
// textures: a=1
layout(set = 0, binding = 7) uniform texture2D a;
float scatterShare(vec3 c, float bias) {
    return mix(1.0, smoothstep(0.30, 0.95, luma(c)), bias);
}
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    vec3 c = clamp(texelFetch(sampler2D(a, sNear), ivec2(gid), 0).rgb, 0.0, 1.0);
    float s = scatterShare(c, u[0]);
    imageStore(o, ivec2(gid), vec4(toLinear(c) * s, 1.0));
}
