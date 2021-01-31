#version 330

precision mediump float;

uniform sampler2DRect u_Texture;

in vec2 v_Position;
out vec4 outColor;
 
void main() {
	vec4 c = texture(u_Texture, v_Position);
	outColor = c;
}