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

// 自定义颜色配置 - 替换以下数值
#define BORDER_COLOR vec4(#, #, #, #)           // 替换#为边框颜色RGBA值
#define COLLISION_BOX_COLOR vec4(%, %, %, %)     // 替换%为碰撞箱颜色RGBA值
#define FISHING_LINE_COLOR vec4(&, &, &, &)     // 替换&为鱼线颜色RGBA值
#define SELECTION_BOX_COLOR vec4(*, *, *, *)    // 替换*为选择框颜色RGBA值

void main() {
    const float PI = 3.14159265359;
    vec4 color = vertexColor * ColorModulator;
    vec4 newcol = color;
    
    // 根据颜色类型应用自定义颜色
    if (color[0] == 0 && color[1] == 0 && color[2] == 0 && color[3] < 1) {
        // 方块边框
        newcol = BORDER_COLOR;
    } else if (color[0] == 1 && color[1] == 1 && color[2] == 1 && color[3] == 1) {
        // 碰撞箱
        newcol = COLLISION_BOX_COLOR;
    } else if (color[0] == 0 && color[1] == 0 && color[2] == 0 && color[3] == 1) {
        // 鱼线
        newcol = FISHING_LINE_COLOR;
    } else if ((color[0] == 1 && color[1] == 0 && color[2] == 0 && color[3] == 1) || 
               (color[0] == 0 && color[1] == 0 && color[2] == 1 && color[3] == 1)) {
        // 选择框（红色或蓝色）
        newcol = SELECTION_BOX_COLOR;
    }
    
    fragColor = linear_fog(newcol, vertexDistance, FogStart, FogEnd, FogColor);
}