#version 330

layout (location=0) in vec2 a_Position;
out vec2 v_Position;

uniform ivec2 u_vpPos;
uniform ivec2 u_vpSize;

uniform ivec2 u_Offset;
uniform ivec2 u_Delta;
uniform float u_Scale;

void main() {
	vec2 pos = a_Position;

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
	v_Position = a_Position;

	gl_Position = vec4(pos, 0.0, 1.0);
}