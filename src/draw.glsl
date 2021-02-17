#version 330

precision mediump float;

uniform sampler2DRect u_Texture;
uniform uvec3 u_Color;
uniform uint u_Alpha;

in vec2 v_Position;
out vec4 outColor;
 
void main() {
	float a = texture(u_Texture, v_Position).r;
	outColor = vec4(u_Color / 255.0, a * (u_Alpha / 100.0));
}
