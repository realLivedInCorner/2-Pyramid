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

void main() {
    vec4 color = vertexColor * ColorModulator;
    
    if(color.a!=1){
        float t0 = GameTime * 1200 + pos.x + pos.y + pos.z;
        float t1 = t0 + 2.094395102366;
        float t2 = t0 + 4.188790204733;
        color = vec4(sin(t0) / 2 + 0.75, sin(t1) / 2 + 0.75, sin(t2) / 2 + 0.75, 1);
    };

    fragColor = linear_fog(color, vertexDistance, FogStart, FogEnd, FogColor);
};


