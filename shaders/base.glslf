#version 150 core

//uniform sampler2D t_font;

in vec4 in_color;
in vec2 in_texcoords;

out vec4 out_color;

void main() {
    //vec4 col = texture2D(t_font, in_texcoords);

    //out_color.rgb = col.aaa * in_color.rgb;
    //out_color.a = 1.0;

    out_color = in_color;
}
