#version 150 core

in vec2 a_Pos;
out vec4 v_Color;

layout(std140) uniform Locals {
  mat4 u_Transform;
  vec3 u_Color;
};

void main() {
  v_Color = vec4(u_Color, 1.0);
  vec4 position = vec4(a_Pos, 0.8, 1.0);
  gl_Position = u_Transform * position;
}
