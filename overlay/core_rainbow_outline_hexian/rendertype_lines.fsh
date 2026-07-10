#version 150

#moj_import <minecraft:fog.glsl>

uniform float GameTime;
uniform vec4 ColorModulator;
uniform float FogStart;
uniform float FogEnd;
uniform vec4 FogColor;

in float vertexDistance;
in vec4 vertexColor;

out vec4 fragColor;

void main() {
    const float PI = 3.14159265359;
    vec4 color = vertexColor * ColorModulator;
    vec4 newcol = color;
    if (color[0] == 0 && color[1] == 0 && color[2] == 0 && color[3] < 1)
    {
        newcol = vec4(abs(sin((gl_FragCoord.x + gl_FragCoord.y) * 0.01 + GameTime * 2000)), abs(sin((gl_FragCoord.x + gl_FragCoord.y) * 0.01 + PI / 3 + GameTime * 2000)), abs(sin((gl_FragCoord.x + gl_FragCoord.y) * 0.01 + 2 * PI / 3 + GameTime * 2000)), 1);
    }
    fragColor = linear_fog(newcol, vertexDistance, FogStart, FogEnd, FogColor);
}
