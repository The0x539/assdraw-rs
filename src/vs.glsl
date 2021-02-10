#version 330
#extension GL_ARB_explicit_uniform_location : require

layout (location=0) uniform vec2 screen_dims;
layout (location=2) uniform vec2 scene_pos;
layout (location=4) uniform float scale;
layout (location=5) uniform vec2 drawing_pos;

layout (location=0) in vec2 a_Position;

out vec2 v_Position;

void main() {
	vec2 pos = a_Position;

	pos += drawing_pos;

	pos -= scene_pos;
	pos /= screen_dims;
	pos *= scale;

	// top-left origin
	pos -= 0.5;
	pos *= 2;
	pos.y *= -1;

	/*
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
	*/

	v_Position = a_Position;

	gl_Position = vec4(pos, 0.0, 1.0);
}