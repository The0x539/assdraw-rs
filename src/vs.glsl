#version 330

layout (location=0) in vec2 a_position;
layout (location=1) in vec4 a_color;

out vec4 color;
out vec2 position;

uniform ivec2 u_vpPos;
uniform ivec2 u_vpSize;

uniform ivec2 u_Offset;
uniform ivec2 u_Delta;
uniform float u_Scale;

void main() {
	color = a_color;
	vec2 pos = a_position;

	// apply offset
	pos += u_Offset + u_Delta;

	// scale to canvas size
	pos /= u_vpSize;

	// centered origin -> corner origin (it feels like this operation is backwards...)
	pos -= 0.5;
	pos *= 2;

	// bottom left -> top left
	pos.y *= -1;

	pos *= u_Scale;
	position = a_position;

	gl_Position = vec4(pos, 0.0, 1.0);
}