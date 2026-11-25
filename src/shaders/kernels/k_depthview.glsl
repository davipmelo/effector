// Depth map preview: near = light, far = dark, with a faint tint so it reads as a map.
layout(set = 0, binding = 7) uniform texture2D depth;
void main() {
    uvec2 gid = gl_GlobalInvocationID.xy;
    if (gid.x >= uint(imageSize(o).x) || gid.y >= uint(imageSize(o).y)) return;
    float d = texelFetch(sampler2D(depth, sNear), ivec2(gid), 0).r;
    imageStore(o, ivec2(gid), vec4(vec3(d), 1.0));
}
