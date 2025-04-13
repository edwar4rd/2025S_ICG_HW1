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
in vec3 vertexColor;
in vec3 fragPosition;
in vec3 fragNormal;
in vec3 lightLocations[3];
in vec3 lightColors[3];
in vec3 lightKdKsCDs[3];
in float Ka_val;
in vec3 ambient_lightColor;

out vec4 outputColor;

vec3 shading(vec3 mvVertex, vec3 mvNormal) {
    vec3 phong = vec3(0.f, 0.f, 0.f);

    float ka = Ka_val;
    vec3 V = -normalize(mvVertex);
    vec3 N = normalize(mvNormal);
    vec3 ambient = ka * ambient_lightColor;

    for(int i = 0; i < 3; ++i) {
        float kd = lightKdKsCDs[i][0], ks = lightKdKsCDs[i][1], CosineDegree = lightKdKsCDs[i][2];

        vec3 L = normalize(lightLocations[i] - mvVertex);
        vec3 H = normalize(L + V);

        vec3 Id = lightColors[i] * max(dot(L, N), 0.f);
        vec3 diffuse = kd * Id;

        vec3 Is = lightColors[i] * pow(max(dot(H, N), 0.f), CosineDegree);
        vec3 specular = ks * Is;

        if(dot(L, N) < 0.f) {
            specular = vec3(0.f, 0.f, 0.f);
        }
        phong += vertexColor * (ambient + diffuse) + specular;
    }
    return phong;
}

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
        outputColor = vec4(shading(fragPosition, fragNormal), 1.0);
    }
    if(shading_mode[0] == 3.) {
        vec3 normal = cross(dFdx(fragPosition), dFdy(fragPosition));
        outputColor = vec4(shading(fragPosition, normal), 1.0);
    }
}
