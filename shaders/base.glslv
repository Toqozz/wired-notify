#version 150 core

in vec2 v_pos;
in vec4 v_color;
uniform mat4 u_proj;

out vec4 in_color;

void main() {
    gl_Position = u_proj * vec4(v_pos, 0.0, 1.0);
    gl_Position.y *= -1.0;

    in_color = v_color;
}
