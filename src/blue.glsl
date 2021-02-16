#version 330

precision mediump float;

uniform uvec3 u_Color;

out vec4 outColor;
 
void main() {
	outColor = vec4(u_Color / 255.0, 1.0);
}