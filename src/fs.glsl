#version 330

precision mediump float;

layout(origin_upper_left) in vec4 gl_FragCoord;

uniform sampler2DRect u_Texture;

in vec4 color;
in vec2 position;
out vec4 outColor;
 
void main() {
	vec4 a = color;
	vec4 b = texture(u_Texture, position);
	vec4 c = mix(a, b, 0.5);
	outColor = c;
}