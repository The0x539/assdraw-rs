#version 330

struct Dimensions {
	vec2 screen_dims;
	vec2 scene_pos;
	float scale;
};

uniform Dimensions u_Dims;

layout (location=0) in vec2 a_Position;

out vec2 v_Position;

void main() {
	vec2 pos = a_Position;

	pos += u_Dims.scene_pos;
	pos /= u_Dims.screen_dims;
	pos *= u_Dims.scale;

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