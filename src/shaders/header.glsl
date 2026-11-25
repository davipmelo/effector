#version 450
// Shared bindings for every compute kernel (mirrors the Metal KIN macro of the macOS app):
//   0 = output (rgba16f storage), 1 = uniform floats `u`, 2 = extra floats `ex`,
//   3..6 = samplers, 7.. = input textures (texture(1) -> binding 7, texture(2) -> 8, ...).
layout(local_size_x = 16, local_size_y = 16, local_size_z = 1) in;
layout(set = 0, binding = 0, rgba16f) uniform writeonly image2D o;
layout(set = 0, binding = 1) readonly buffer UBuf { float u[]; };
layout(set = 0, binding = 2) readonly buffer ExBuf { float ex[]; };
layout(set = 0, binding = 3) uniform sampler sLin;
layout(set = 0, binding = 4) uniform sampler sNear;
layout(set = 0, binding = 5) uniform sampler sRep;
layout(set = 0, binding = 6) uniform sampler sMir;

// Metal/C fmod: truncated remainder (sign follows x), unlike GLSL mod.
float fmodc(float x, float y) { return x - y * trunc(x / y); }
vec2 fmodc(vec2 x, vec2 y) { return x - y * trunc(x / y); }
vec2 fmodc(vec2 x, float y) { return x - y * trunc(x / y); }
vec3 fmodc(vec3 x, float y) { return x - y * trunc(x / y); }
float sat(float x) { return clamp(x, 0.0, 1.0); }
vec3 sat(vec3 x) { return clamp(x, 0.0, 1.0); }
