#version 150

in vec3 Position;
in vec4 Color;
in vec3 Normal;

uniform mat4 ModelViewMat;
uniform mat4 ProjMat;
uniform float LineWidth;
uniform vec2 ScreenSize;
uniform int FogShape;

out float vertexDistance;
out vec4 vertexColor;

flat out int CustomOutlinesLineType;
out float CustomOutlinesGradient;
out vec3 vertexPos;

const float VIEW_SHRINK = 1.0 - (1.0 / 256.0);
const mat4 VIEW_SCALE = mat4(
    VIEW_SHRINK, 0.0, 0.0, 0.0,
    0.0, VIEW_SHRINK, 0.0, 0.0,
    0.0, 0.0, VIEW_SHRINK, 0.0,
    0.0, 0.0, 0.0, 1.0
);

out vec4 pos1, pos2;
flat out vec4 pos3;

// 自定义颜色配置 - 替换以下数值
#define BORDER_COLOR vec4(#, #, #, #)           // 替换#为边框颜色RGBA值
#define COLLISION_BOX_COLOR vec4(%, %, %, %)     // 替换%为碰撞箱颜色RGBA值
#define FISHING_LINE_COLOR vec4(&, &, &, &)     // 替换&为鱼线颜色RGBA值
#define SELECTION_BOX_COLOR vec4(*, *, *, *)    // 替换*为选择框颜色RGBA值

// 自定义线条宽度 - 替换@为方块边框宽度值
#define BORDER_LINE_WIDTH @

void main() {
    vertexDistance = length((ModelViewMat * vec4(Position, 1.0)).xyz);
    vertexPos = Position;
    int id = gl_VertexID % 4;

    vec4 linePosStart = ProjMat * VIEW_SCALE * ModelViewMat * vec4(Position, 1.0);
    vec4 linePosEnd = ProjMat * VIEW_SCALE * ModelViewMat * vec4(Position + Normal, 1.0);

    vec3 ndc1 = linePosStart.xyz / linePosStart.w;
    vec3 ndc2 = linePosEnd.xyz / linePosEnd.w;

    vec2 lineScreenDirection = normalize((ndc2.xy - ndc1.xy) * ScreenSize);

    // 根据线条类型设置不同的宽度乘数
    float lineWidthMultiplier = 2.0;
    CustomOutlinesLineType = 0;
    
    // 如果是方块边框，使用自定义宽度
    if (Color == vec4(0.0, 0.0, 0.0, 0.4)) {
        lineWidthMultiplier = BORDER_LINE_WIDTH;
    }
    
    vec2 lineOffset = vec2(-lineScreenDirection.y, lineScreenDirection.x) * LineWidth * lineWidthMultiplier / ScreenSize;

    if (lineOffset.x < 0.0) {
        lineOffset *= -1.0;
    }

    if (gl_VertexID % 2 == 0) {
        vertexPos = (ndc1 + vec3(lineOffset, 0.0)) * linePosStart.w;
        gl_Position = vec4(vertexPos, linePosStart.w);
    } else {
        vertexPos = (ndc1 - vec3(lineOffset, 0.0)) * linePosStart.w;
        gl_Position = vec4(vertexPos, linePosStart.w);
    }

    // 根据原始颜色应用自定义颜色
    if (Color == vec4(0.0, 0.0, 0.0, 0.4)) {
        // 方块边框
        vertexColor = BORDER_COLOR;
    } else if (Color == vec4(1.0, 1.0, 1.0, 1.0)) {
        // 碰撞箱
        vertexColor = COLLISION_BOX_COLOR;
    } else if (Color == vec4(0.0, 0.0, 0.0, 1.0)) {
        // 鱼线
        vertexColor = FISHING_LINE_COLOR;
    } else if (Color == vec4(1.0, 0.0, 0.0, 1.0) || Color == vec4(0.0, 0.0, 1.0, 1.0)) {
        // 选择框（红色或蓝色）
        vertexColor = SELECTION_BOX_COLOR;
    } else {
        // 其他线条保持原色
        vertexColor = Color;
    }

    pos1 = pos2 = vec4(0);
    pos3 = vec4(Position, id == 1);
    if (id == 0) pos1 = vec4(Position, 1);
    if (id == 2) pos2 = vec4(Position, 1);
}