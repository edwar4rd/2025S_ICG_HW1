#version 300 es

// TODO HERE: 
// modify fragment shader or write another one
// to implement flat, gouraud and phong shading

precision mediump float;

in vec3 shading_mode;

// for flat shading
flat in vec4 flatcolor;

// for gouraud shading
in vec4 fragcolor;

// for phong shading
// in vec3 vertexColor;
// in vec3 fragPosition;
// in vec3 fragNormal;
// in vec3 lightLocations[3];
// in vec3 lightColors[3];
// in vec3 lightKdKsCDs[3];
// in float Ka_val;
// in vec3 ambient_lightColor;

out vec4 outputColor;

void main(void) {
    if(shading_mode[0] == 0.) {
        // flat shading
        outputColor = flatcolor;
    }
    if(shading_mode[0] == 1.) {
        // gouraud shading
        outputColor = fragcolor;
    }
    if(shading_mode[0] == 2.) {
        // TODO: phong shading
        outputColor = vec4(1.0, 0.0, 1.0, 1.0);
    }
}
