in vec3 aVertexPosition;
in vec3 aFrontColor;
in vec3 aVertexNormal;

uniform vec3 lightLoc[3];
uniform vec3 lightColor[3];
uniform vec3 lightKdKsCD[3];
uniform mat4 uMVMatrix;
uniform mat4 uPMatrix;
uniform float Ka;
uniform vec3 ambient_color;

uniform int mode;

out vec3 shading_mode;

// for flat shading
flat out vec4 flatcolor;

// for gouraud shading
out vec4 fragcolor;

// for phong shading
out vec3 vertexColor;
out vec3 fragPosition;
out vec3 fragNormal;
out vec3 lightLocations[3];
out vec3 lightColors[3];
out vec3 lightKdKsCDs[3];
out float Ka_val;
out vec3 ambient_lightColor;

vec3 shading(vec3 vertex) {
    vec3 phong = vec3(0.f, 0.f, 0.f);
    vec3 mvVertex = (uMVMatrix * vec4(vertex, 1.0f)).xyz;
    vec3 mvNormal = mat3(uMVMatrix) * aVertexNormal;

    float ka = Ka;
    vec3 V = -normalize(mvVertex);
    vec3 N = normalize(mvNormal);
    vec3 ambient = ka * ambient_color;

    for(int i = 0; i < 3; ++i) {
        float kd = lightKdKsCD[i][0], ks = lightKdKsCD[i][1], CosineDegree = lightKdKsCD[i][2];

        vec3 L = normalize(lightLoc[i] - mvVertex);
        vec3 H = normalize(L + V);

        vec3 Id = lightColor[i] * max(dot(L, N), 0.f);
        vec3 diffuse = kd * Id;

        vec3 Is = lightColor[i] * pow(max(dot(H, N), 0.f), CosineDegree);
        vec3 specular = ks * Is;

        if(dot(L, N) < 0.f) {
            specular = vec3(0.f, 0.f, 0.f);
        }
        phong += aFrontColor * (ambient + diffuse) + specular;
    }
    return phong;
}

void main(void) {
    shading_mode = vec3(mode);

    vec3 vertex_copy = aVertexPosition;

    if(mode == 0) {
        // flat shading
        flatcolor = vec4(shading(vertex_copy), 1.0f);
    }

    if(mode == 1) {
        // gouraud shading
        fragcolor = vec4(shading(vertex_copy), 1.0f);
    }

    if(mode == 2 || mode == 3 || mode == 4)  {
        // phong shading
        vertexColor = aFrontColor;
        fragPosition = (uMVMatrix * vec4(vertex_copy, 1.0f)).xyz;
        fragNormal = mat3(uMVMatrix) * aVertexNormal;
        Ka_val = Ka;
        ambient_lightColor = ambient_color;
        for(int i = 0; i < 3; ++i) {
            lightLocations[i] = lightLoc[i];
            lightColors[i] = lightColor[i];
            lightKdKsCDs[i] = lightKdKsCD[i];
        }
    }

    gl_Position = uPMatrix * uMVMatrix * vec4(vertex_copy, 1.0f);
}
