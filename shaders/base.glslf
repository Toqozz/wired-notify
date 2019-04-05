#version 150 core

uniform sampler2D t_font;

in vec4 in_color;
in vec2 in_texcoords;

out vec4 out_color;

void main() {
    vec4 col = texture2D(t_font, in_texcoords);
    col.xyz = in_color.xyz;

    out_color = col;
}
