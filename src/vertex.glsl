const vec2 verts[3] = vec2[3](vec2(0.0, 1.0), vec2(-1.0, -1.0), vec2(1.0, -1.0));
const vec4 colors[3] = vec4[3](vec4(1.0, 0.0, 0.0, 1.0), vec4(0.0, 1.0, 0.0, 1.0), vec4(0.0, 0.0, 1.0, 1.0));

// in vec3 verts;

uniform mat4 uPMatrix;
uniform mat4 uMVMatrix;

out vec4 v_color;
void main() {
    v_color = colors[gl_VertexID];
    gl_Position = uPMatrix * uMVMatrix * vec4(vec3(verts[gl_VertexID], 0.0), 1.0);
}