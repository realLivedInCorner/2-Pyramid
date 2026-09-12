#version 150

#moj_import <fog.glsl>

uniform vec4 ColorModulator;
uniform float FogStart;
uniform float FogEnd;
uniform vec4 FogColor;
uniform float GameTime;

in float vertexDistance;
in vec4 vertexColor;
in vec3 pos;

out vec4 fragColor;

// 双色循环 — 打包时替换占位符（默认浅紫 ↔ 黑）
#define GRADIENT_COLOR_A vec4(@, @, @, @)
#define GRADIENT_COLOR_B vec4($, $, $, $)

void main() {
    vec4 color = vertexColor * ColorModulator;

    if (color.a != 1) {
        // 与 core_rainbow_outline 同源的空间相位 + GameTime 脉动
        float t0 = GameTime * 5000.0 + pos.x + pos.y + pos.z;
        float intensity = sin(t0) * 0.5 + 0.5;
        vec3 finalColor = mix(GRADIENT_COLOR_B.rgb, GRADIENT_COLOR_A.rgb, intensity);
        float alpha = mix(GRADIENT_COLOR_B.a, GRADIENT_COLOR_A.a, intensity);
        color = vec4(finalColor, alpha);
    }

    fragColor = linear_fog(color, vertexDistance, FogStart, FogEnd, FogColor);
}
