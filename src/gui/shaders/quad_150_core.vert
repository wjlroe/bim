#version 150 core

in vec2 a_Pos;
out vec4 v_Color;

layout(std140) uniform Locals {
  mat4 u_Transform;
  vec4 u_Color;
  float u_Z;
};

void main() {
  v_Color = u_Color;
  vec4 position = vec4(a_Pos, u_Z, 1.0);
  gl_Position = u_Transform * position;
}
