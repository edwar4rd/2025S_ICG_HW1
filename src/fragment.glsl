precision mediump float;

in vec3 shading_mode;
in vec3 clipping_plane_f;
in vec3 clipping_plane_pos_f;
in vec3 fragPosition;

// for flat shading
flat in vec4 flatcolor;

// for gouraud shading
in vec4 fragcolor;

// for phong shading
in vec3 vertexColor;
in vec3 fragNormal;
in vec3 lightLocations[3];
in vec3 lightColors[3];
in vec3 lightKdKsCDs[3];
in float Ka_val;
in vec3 ambient_lightColor;
out vec4 outputColor;

vec3 shading(vec3 mvVertex, vec3 mvNormal) {
    vec3 phong = vec3(0., 0., 0.);

    float ka = Ka_val;
    vec3 V = -normalize(mvVertex);
    vec3 N = normalize(mvNormal);
    vec3 ambient = ka * ambient_lightColor;

    for(int i = 0; i < 3; ++i) {
        float kd = lightKdKsCDs[i][0], ks = lightKdKsCDs[i][1], CosineDegree = lightKdKsCDs[i][2];

        vec3 L = normalize(lightLocations[i] - mvVertex);
        vec3 H = normalize(L + V);

        vec3 Id = lightColors[i] * max(dot(L, N), 0.);
        vec3 diffuse = kd * Id;

        vec3 Is = lightColors[i] * pow(max(dot(H, N), 0.), CosineDegree);
        vec3 specular = ks * Is;

        if(dot(L, N) < 0.) {
            specular = vec3(0., 0., 0.);
        }
        phong += vertexColor * (ambient + diffuse) + specular;
    }
    return phong;
}

void main(void) {
    if(dot((fragPosition - clipping_plane_pos_f) , clipping_plane_f) < 0.) {
        discard;
    }

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
    if(shading_mode[0] == 4.) {
        outputColor = vec4(vec3(0.6, 0.4, 0.9) * (ceil((shading(fragPosition, fragNormal).x) * 5.0) / 5.0), 1.0);
    }
}
