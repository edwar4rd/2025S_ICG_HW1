in vec3 aVertexPosition;
in vec3 aFrontColor;

uniform mat4 uMVMatrix;
uniform mat4 uPMatrix;
uniform float Ka;
uniform vec3 ambient_color;

// for gouraud shading
out vec4 fragcolor;

void main(void) {
    fragcolor = vec4(Ka*ambient_color*aFrontColor, 1.0);
    gl_Position = uPMatrix * uMVMatrix * vec4(aVertexPosition, 1.0);
}
