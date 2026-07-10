// Made by SuperAnt_

#version 150

in vec3 Position;
in vec4 Color;

uniform mat4 ModelViewMat;
uniform mat4 ProjMat;

out vec4 vertexColor;

void main() {
    gl_Position = ProjMat * ModelViewMat * vec4(Position, 1.0);

    vertexColor = Color;
	if(vertexColor.r == 0.0627451 && vertexColor.a >= 0.7 && vertexColor.a < 0.9) {
		vertexColor.a = 0; // this removes the background. You can set the transparency to whatever you want (0 is fully transparent, 1 is the opposite)
	}
}
