#version 330

precision mediump float;

layout(origin_upper_left) in vec4 gl_FragCoord;

in vec4 color;
out vec4 outColor;
 
void main() {
	outColor = color;
}