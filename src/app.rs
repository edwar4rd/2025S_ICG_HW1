use std::{collections::BTreeMap, sync::Arc};

use eframe::{
    egui_glow::{self, glow},
    glow::{Buffer, HasContext, VertexArray},
};
use egui::{mutex::Mutex, Checkbox, RichText, Slider};
use glam::{vec3, Mat4, Quat, Vec3};
use poll_promise::Promise;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};

#[derive(Copy, Clone, PartialEq, Eq, Deserialize, Serialize, Default, IntoStaticStr, EnumIter)]
enum RenderingMode {
    Flat = 0,
    Gouraud,
    #[default]
    Phong,
    FakeFlat,
    Cartoon,
}

#[derive(Clone, Deserialize, Serialize)]
struct CGObject {
    name: String,
    translation: Vec3,
    rotation: Vec3,
    rotating: Vec3,
    scale: Vec3,
    shear: Vec3,
    rendering_mode: RenderingMode,
    model_id: Option<(usize, usize)>,
}

impl Default for CGObject {
    fn default() -> Self {
        Self {
            name: "Object".into(),
            translation: Default::default(),
            rotation: Default::default(),
            rotating: Default::default(),
            scale: vec3(1., 1., 1.),
            shear: vec3(90., 90., 90.),
            rendering_mode: Default::default(),
            model_id: Default::default(),
        }
    }
}

impl CGObject {
    fn tick_animation(&mut self, dt: f32) {
        for i in 0..3 {
            self.rotation[i] += self.rotating[i] * dt;
            if self.rotation[i] > 180. {
                self.rotation[i] -= 360.;
            };
            if self.rotation[i] < -180. {
                self.rotation[i] += 360.;
            };
        }
    }

    fn mv_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            self.scale,
            Quat::from_rotation_x(self.rotation.x.to_radians())
                * Quat::from_rotation_y(self.rotation.y.to_radians())
                * Quat::from_rotation_z(self.rotation.z.to_radians()),
            self.translation,
        ) * Mat4::from_cols_array(&[
            1.,
            0.,
            self.shear.z.to_radians().tan().recip(),
            0.,
            self.shear.x.to_radians().tan().recip(),
            1.,
            0.,
            0.,
            0.,
            self.shear.y.to_radians().tan().recip(),
            1.,
            0.,
            0.,
            0.,
            0.,
            1.,
        ])
    }

    fn to_rendered(&self) -> RenderedObject {
        RenderedObject {
            mv_mat: self.mv_matrix(),
            mode: self.rendering_mode as i32,
            model_id: self.model_id.map(|(_self_id, gl_id)| gl_id),
        }
    }
}

#[derive(Default)]
enum ModelState {
    #[default]
    Ready,
    Loading(Promise<Option<ICGJson>>),
    Failed,
    Loaded(usize),
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Deserialize, Serialize, Default)]
struct CGModel {
    source: String,
    #[serde(skip)]
    state: ModelState,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Deserialize, Serialize)]
pub struct DemoApp {
    ambient: [f32; 3],
    ambient_ka: f32,
    rotation_enabled: bool,
    selected_object: Option<usize>,
    objects: Vec<CGObject>,
    dummy_object: CGObject,
    #[serde(skip)]
    models: Arc<Mutex<BTreeMap<usize, CGModel>>>,
    model_source: String,
    camera_pos: Vec3,
    fovy: f32,
    #[serde(skip)]
    gl_stuff: Arc<Mutex<Option<GLStuff>>>,
}

impl Default for DemoApp {
    fn default() -> Self {
        Self {
            ambient: Default::default(),
            ambient_ka: Default::default(),
            rotation_enabled: Default::default(),
            selected_object: Default::default(),
            objects: Default::default(),
            dummy_object: Default::default(),
            models: Default::default(),
            model_source: "/model/".into(),
            camera_pos: vec3(0., 0., 25.),
            fovy: 60f32,
            gl_stuff: Default::default(),
        }
    }
}

impl DemoApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Option<Self> {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let mut value: DemoApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            DemoApp::default()
        };

        let gl = cc.gl.as_ref()?;

        value.gl_stuff = Arc::new(Mutex::new(Some(GLStuff::new(gl)?)));
        Some(value)
    }
}

impl eframe::App for DemoApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.input(|i| {
            if i.key_down(egui::Key::W) {
                self.camera_pos.z -= 0.1;
            }
            if i.key_down(egui::Key::S) {
                self.camera_pos.z += 0.1;
            }
            if i.key_down(egui::Key::A) {
                self.camera_pos.x -= 0.1;
            }
            if i.key_down(egui::Key::D) {
                self.camera_pos.x += 0.1;
            }
            if i.key_down(egui::Key::E) {
                self.camera_pos.y += 0.1;
            }
            if i.key_down(egui::Key::Q) {
                self.camera_pos.y -= 0.1;
            }
        });

        let dt = ctx.input(|i| i.unstable_dt);
        if self.rotation_enabled {
            for obj in &mut self.objects {
                obj.tick_animation(dt);
            }
            ctx.request_repaint();
        }
        // Put your widgets into a `SidePanel`, `TopBottomPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:

            egui::menu::bar(ui, |ui| {
                // NOTE: no File->Quit on web pages!
                let is_web = cfg!(target_arch = "wasm32");
                if !is_web {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });
                    ui.add_space(16.0);
                }

                ui.heading("2025S ICG Homework #1");
                ui.add_space(50.);
                egui::widgets::global_theme_preference_buttons(ui);
            });
        });

        egui::Window::new("Settings").vscroll(true).show(ctx, |ui| {
            ui.heading("Rotation");
            ui.add(Checkbox::new(&mut self.rotation_enabled, "Enabled"));
            ui.separator();

            ui.heading("Ambient Light");
            ui.add(Slider::new(&mut self.ambient_ka, 0.0..=1.0).text("Ka"));
            ui.add(Slider::new(&mut self.ambient[0], 0.0..=1.0).text("Red"));
            ui.add(Slider::new(&mut self.ambient[1], 0.0..=1.0).text("Green"));
            ui.add(Slider::new(&mut self.ambient[2], 0.0..=1.0).text("Blue"));
            ui.separator();

            ui.heading("Camera");
            ui.label(format!("x: {}", self.camera_pos.x));
            ui.label(format!("y: {}", self.camera_pos.y));
            ui.label(format!("z: {}", self.camera_pos.z));
            ui.add(Slider::new(&mut self.fovy, 0.0..=180.).text("fov"));
            ui.separator();

            ui.heading("Objects");
            self.object_settings(ui);
            ui.separator();

            ui.heading("Models");
            self.model_settings(ui);
            ui.separator();

            ui.add(egui::github_link_file!(
                "https://github.com/edwar4rd/2025S_ICG_HW1/",
                "Source code."
            ));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::Frame::canvas(ui.style()).show(ui, |ui| {
                self.custom_painting(ui);
            });
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                powered_by_egui_and_eframe(ui);
                egui::warn_if_debug_build(ui);
            });
        });
    }

    fn on_exit(&mut self, gl: Option<&glow::Context>) {
        if let Some(gl) = gl {
            if let Some(stuff) = self.gl_stuff.lock().as_ref() {
                stuff.destroy(gl)
            }
        }
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}

impl DemoApp {
    fn object_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("New Object").clicked() {
                let mut new_obj = self.dummy_object.clone();
                new_obj.name = format!("Object{:02}", self.objects.len());
                self.objects.push(new_obj);
                if self.selected_object.is_none() {
                    self.selected_object = Some(self.objects.len() - 1);
                }
            }

            if ui
                .button(RichText::new("Clear Objects").color(egui::Color32::RED))
                .clicked()
            {
                self.objects.clear();
                self.selected_object = None;
            }
        });
        ui.add_space(10.);

        egui::ComboBox::new("obj", "Selected Object")
            .selected_text(
                self.selected_object
                    .and_then(|obj_id| self.objects.get(obj_id))
                    .map(|obj| obj.name.as_str())
                    .unwrap_or("None"),
            )
            .show_ui(ui, |ui| {
                for (id, obj) in self.objects.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_object, Some(id), &obj.name);
                }
            });

        {
            let selected = self.selected_object.is_some();
            let selected_obj = self
                .selected_object
                .and_then(|obj_id| self.objects.get_mut(obj_id))
                .unwrap_or(&mut self.dummy_object);

            if selected {
                ui.text_edit_singleline(&mut selected_obj.name);
            } else {
                ui.label("None Selected");
            }

            egui::ComboBox::new("obj_mode", "Mode")
                .selected_text(Into::<&'static str>::into(selected_obj.rendering_mode))
                .show_ui(ui, |ui| {
                    for mode in RenderingMode::iter() {
                        ui.selectable_value(
                            &mut selected_obj.rendering_mode,
                            mode,
                            Into::<&'static str>::into(mode),
                        );
                    }
                });

            egui::ComboBox::new("obj_model", "Model")
                .selected_text(
                    selected_obj
                        .model_id
                        .and_then(|id| {
                            self.models
                                .lock()
                                .get(&id.0)
                                .map(|model| model.source.clone())
                        })
                        .unwrap_or("None".into()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected_obj.model_id, None, "None");
                    for (id, model) in self.models.lock().iter() {
                        if let ModelState::Loaded(gl_id) = &model.state {
                            ui.selectable_value(
                                &mut selected_obj.model_id,
                                Some((*id, *gl_id)),
                                model.source.clone(),
                            );
                        }
                    }
                });

            ui.collapsing("Scale", |ui| {
                ui.add(Slider::new(&mut selected_obj.scale[0], 0.01..=10.0).text("Scale.x"));
                ui.add(Slider::new(&mut selected_obj.scale[1], 0.01..=10.0).text("Scale.y"));
                ui.add(Slider::new(&mut selected_obj.scale[2], 0.01..=10.0).text("Scale.z"));
                ui.horizontal(|ui| {
                    if ui.button("Make Uniform").clicked() {
                        selected_obj.scale = selected_obj.scale.length() * Vec3::ONE / 3f32.sqrt();
                    }
                    if ui.button("Reset Scale").clicked() {
                        selected_obj.scale = vec3(1., 1., 1.);
                    }
                });
            });

            ui.collapsing("Translation", |ui| {
                ui.add(
                    Slider::new(&mut selected_obj.translation[0], -20.0..=20.0)
                        .text("Translation.x"),
                );
                ui.add(
                    Slider::new(&mut selected_obj.translation[1], -20.0..=20.0)
                        .text("Translation.y"),
                );
                ui.add(
                    Slider::new(&mut selected_obj.translation[2], -20.0..=20.0)
                        .text("Translation.z"),
                );
                if ui.button("Reset Translation").clicked() {
                    selected_obj.translation = vec3(0., 0., 0.);
                }
            });
            ui.collapsing("Rotation", |ui| {
                ui.add(
                    Slider::new(&mut selected_obj.rotation[0], -180.0..=180.0).text("Rotation.x"),
                );
                ui.add(
                    Slider::new(&mut selected_obj.rotation[1], -180.0..=180.0).text("Rotation.y"),
                );
                ui.add(
                    Slider::new(&mut selected_obj.rotation[2], -180.0..=180.0).text("Rotation.z"),
                );
                ui.horizontal(|ui| {
                    if ui.button("X +90°").clicked() {
                        selected_obj.rotation.x += 90.;
                        if selected_obj.rotation.x > 180. {
                            selected_obj.rotation.x -= 360.;
                        }
                    }
                    if ui.button("Y +90°").clicked() {
                        selected_obj.rotation.y += 90.;
                        if selected_obj.rotation.y > 180. {
                            selected_obj.rotation.y -= 360.;
                        }
                    }
                    if ui.button("Z +90°").clicked() {
                        selected_obj.rotation.z += 90.;
                        if selected_obj.rotation.z > 180. {
                            selected_obj.rotation.z -= 360.;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if ui.button("X +180°").clicked() {
                        selected_obj.rotation.x += 180.;
                        if selected_obj.rotation.x > 180. {
                            selected_obj.rotation.x -= 360.;
                        }
                    }
                    if ui.button("Y +180°").clicked() {
                        selected_obj.rotation.y += 180.;
                        if selected_obj.rotation.y > 180. {
                            selected_obj.rotation.y -= 360.;
                        }
                    }
                    if ui.button("Z +180°").clicked() {
                        selected_obj.rotation.z += 180.;
                        if selected_obj.rotation.z > 180. {
                            selected_obj.rotation.z -= 360.;
                        }
                    }
                });

                if ui.button("Reset Rotation").clicked() {
                    selected_obj.rotation = vec3(0., 0., 0.);
                }
            });
            ui.collapsing("Shear", |ui| {
                ui.add(Slider::new(&mut selected_obj.shear[0], 0.0..=180.0).text("Shear.x"));
                ui.add(Slider::new(&mut selected_obj.shear[1], 0.0..=180.0).text("Shear.y"));
                ui.add(Slider::new(&mut selected_obj.shear[2], 0.0..=180.0).text("Shear.z"));

                if ui.button("Reset Shear").clicked() {
                    selected_obj.shear = vec3(90., 90., 90.);
                }
            });
            ui.collapsing("Animation", |ui| {
                ui.add(
                    Slider::new(&mut selected_obj.rotating[0], -360.0..=360.0).text("Rotating.x"),
                );
                ui.add(
                    Slider::new(&mut selected_obj.rotating[1], -360.0..=360.0).text("Rotating.y"),
                );
                ui.add(
                    Slider::new(&mut selected_obj.rotating[2], -360.0..=360.0).text("Rotating.z"),
                );

                if ui.button("Reset Animation").clicked() {
                    selected_obj.rotating = vec3(0., 0., 0.);
                }
            });
        }
    }

    fn model_settings(&mut self, ui: &mut egui::Ui) {
        ui.text_edit_singleline(&mut self.model_source);
        ui.horizontal(|ui| {
            if ui.button("New Model").clicked() {
                const URL_BASE: &str = if cfg!(target_arch = "wasm32") {
                    "."
                } else {
                    "https://edwar4rd.github.io/2025S_ICG_HW1"
                };

                let source = self.model_source.clone();
                let request = ehttp::Request::get(format!("{}{}", URL_BASE, &source));
                let (tx, rx) = Promise::new();
                ehttp::fetch(request, move |response| {
                    let resource = response.ok().and_then(|res| {
                        res.text()
                            .and_then(|text| serde_json::from_str::<ICGJson>(text).ok())
                    });
                    tx.send(resource);
                });
                let new_key = self
                    .models
                    .lock()
                    .last_key_value()
                    .map(|(key, _)| *key + 1)
                    .unwrap_or(0);
                self.models.lock().insert(new_key, {
                    CGModel {
                        source,
                        state: ModelState::Loading(rx),
                    }
                });
            }
            if ui
                .button(RichText::new("Clear Models").color(egui::Color32::RED))
                .clicked()
            {
                self.models.lock().clear();
            }
        });

        for (id, model) in self.models.lock().iter() {
            ui.horizontal(|ui| {
                ui.label(format!("{id}"));
                ui.label(&model.source);
                match &model.state {
                    ModelState::Ready => {
                        ui.label("Ready");
                    }
                    ModelState::Loading(_) => {
                        ui.label("Fetching...");
                    }
                    ModelState::Failed => {
                        ui.label("Failed downloading...");
                    }
                    ModelState::Loaded(id) => {
                        ui.label(format!("Loaded, id {}", id));
                    }
                }
            });
        }
    }

    fn get_scene_data(&self) -> SceneData {
        SceneData {
            objs: self.objects.iter().map(|obj| obj.to_rendered()).collect(),
            ambient: self.ambient,
            ambient_ka: self.ambient_ka,
            camera_pos: self.camera_pos,
            fovy: self.fovy,
        }
    }

    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

        {
            let selected_obj = self
                .selected_object
                .and_then(|obj_id| self.objects.get_mut(obj_id))
                .unwrap_or(&mut self.dummy_object);
            selected_obj.translation.x += response.drag_motion().x * 0.01;
            selected_obj.translation.y += response.drag_motion().y * -0.01;
        }

        // Clone locals so we can move them into the paint callback:
        // TODO: Optimize this
        let gl_stuff = self.gl_stuff.clone();
        let models = self.models.clone();
        let scene_data = Arc::new(self.get_scene_data());

        let cb = egui_glow::CallbackFn::new(move |info, painter| {
            let width = info.clip_rect_in_pixels().width_px;
            let height = info.clip_rect_in_pixels().height_px;

            if let Some(stuff) = gl_stuff.lock().as_mut() {
                stuff.process_model(painter.gl(), &mut models.lock());
                stuff.paint(
                    painter.gl(),
                    width,
                    height,
                    scene_data.clone(),
                    painter.intermediate_fbo(),
                )
            }
        });

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }
}

struct RenderedObject {
    mv_mat: Mat4,
    mode: i32,
    model_id: Option<usize>,
}

struct SceneData {
    objs: Vec<RenderedObject>,
    ambient: [f32; 3],
    ambient_ka: f32,
    camera_pos: Vec3,
    fovy: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ICGJson {
    vertex_positions: Vec<f32>,
    vertex_normals: Vec<f32>,
    vertex_frontcolors: Vec<f32>,
    vertex_backcolors: Vec<f32>,
    #[serde(default)]
    vertex_texture_coords: Vec<f32>,
}

struct ICGLoaded {
    pos_buffer: Buffer,
    color_buffer: Buffer,
    norm_buffer: Buffer,
    item_count: i32,
}

impl ICGJson {
    fn load_model(&self, vao: VertexArray, gl: &glow::Context) -> ICGLoaded {
        unsafe {
            // let bound_vao = gl
            //     .get_parameter_vertex_array(glow::VERTEX_ARRAY_BINDING)
            //     .unwrap();

            gl.bind_vertex_array(Some(vao));

            let pos_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(pos_buffer));
            let data: &[f32] = &self.vertex_positions;
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(data),
                glow::STATIC_DRAW,
            );

            let color_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(color_buffer));
            let data: &[f32] = &self.vertex_frontcolors;
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(data),
                glow::STATIC_DRAW,
            );

            let norm_buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(norm_buffer));
            let data: &[f32] = &self.vertex_normals;
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(data),
                glow::STATIC_DRAW,
            );

            gl.bind_vertex_array(None);

            ICGLoaded {
                pos_buffer,
                color_buffer,
                norm_buffer,
                item_count: self.vertex_positions.len() as i32 / 3,
            }
        }
    }
}

impl ICGLoaded {
    fn destroy(&self, gl: &glow::Context) {
        unsafe {
            gl.delete_buffer(self.pos_buffer);
            gl.delete_buffer(self.color_buffer);
            gl.delete_buffer(self.norm_buffer);
        }
    }
}

struct GLStuff {
    program: glow::Program,
    vertex_array: VertexArray,
    default_model: ICGLoaded,
    models: BTreeMap<usize, ICGLoaded>,
}

#[allow(unsafe_code)] // we need unsafe code to use glow
impl GLStuff {
    fn process_model(&mut self, gl: &glow::Context, model_list: &mut BTreeMap<usize, CGModel>) {
        let mut used_model = std::collections::BTreeSet::new();
        for (_, model) in model_list.iter_mut() {
            match &mut model.state {
                ModelState::Ready => {}
                ModelState::Loading(promise) => {
                    if let Some(result) = promise.ready() {
                        if let Some(loaded) = result {
                            let loaded = loaded.load_model(self.vertex_array, gl);
                            let new_key = self
                                .models
                                .last_key_value()
                                .map(|(key, _)| *key + 1)
                                .unwrap_or(0);
                            self.models.insert(new_key, loaded);
                            model.state = ModelState::Loaded(new_key);
                            used_model.insert(new_key);
                        } else {
                            model.state = ModelState::Failed;
                        }
                    }
                }
                ModelState::Failed => {}
                ModelState::Loaded(id) => {
                    used_model.insert(*id);
                }
            }
        }
        let mut mark_delete = Vec::new();
        for id in &mut self.models.keys() {
            if !used_model.contains(id) {
                mark_delete.push(*id);
            }
        }
        for id in mark_delete {
            let model = self.models.get(&id).unwrap();
            unsafe {
                gl.bind_vertex_array(Some(self.vertex_array));
                model.destroy(gl);
                gl.bind_vertex_array(None);
            }
            self.models.remove(&id);
        }
    }

    fn new(gl: &glow::Context) -> Option<Self> {
        use glow::HasContext as _;

        let shader_version = egui_glow::ShaderVersion::get(gl);

        unsafe {
            let program = gl.create_program().expect("Cannot create program");

            if !shader_version.is_new_shader_interface() {
                log::warn!(
                    "Custom 3D painting hasn't been ported to {:?}",
                    shader_version
                );
                return None;
            }

            let (vertex_shader_source, fragment_shader_source) =
                (include_str!("vertex.glsl"), include_str!("fragment.glsl"));

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let shaders: Vec<_> = shader_sources
                .iter()
                .map(|(shader_type, shader_source)| {
                    let shader = gl
                        .create_shader(*shader_type)
                        .expect("Cannot create shader");
                    gl.shader_source(
                        shader,
                        &format!(
                            "{}\n{}",
                            shader_version.version_declaration(),
                            shader_source
                        ),
                    );
                    gl.compile_shader(shader);
                    assert!(
                        gl.get_shader_compile_status(shader),
                        "Failed to compile custom_3d_glow {shader_type}: {}",
                        gl.get_shader_info_log(shader)
                    );

                    gl.attach_shader(program, shader);
                    shader
                })
                .collect();

            gl.link_program(program);
            assert!(
                gl.get_program_link_status(program),
                "{}",
                gl.get_program_info_log(program)
            );

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let vertex_array = gl.create_vertex_array().unwrap();

            gl.bind_vertex_array(Some(vertex_array));
            let vertex_position_loc = gl.get_attrib_location(program, "aVertexPosition").unwrap();
            gl.enable_vertex_attrib_array(vertex_position_loc);
            let front_color_loc = gl.get_attrib_location(program, "aFrontColor").unwrap();
            gl.enable_vertex_attrib_array(front_color_loc);
            let vertex_normal_loc = gl.get_attrib_location(program, "aVertexNormal").unwrap();
            gl.enable_vertex_attrib_array(vertex_normal_loc);
            gl.bind_vertex_array(None);

            let teapot_json = include_str!("../model/Slider.json");
            let teapot_json: ICGJson = serde_json::from_str(teapot_json).unwrap();
            let teapot_model = teapot_json.load_model(vertex_array, gl);

            Some(Self {
                program,
                vertex_array,
                default_model: teapot_model,
                models: BTreeMap::new(),
            })
        }
    }

    fn destroy(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            self.default_model.destroy(gl);
            for model in self.models.values() {
                model.destroy(gl);
            }
        }
    }

    fn paint(
        &self,
        gl: &glow::Context,
        width: i32,
        height: i32,
        scene_data: Arc<SceneData>,
        intermediate_fbo: Option<glow::Framebuffer>,
    ) {
        use glow::HasContext as _;
        let perspective_mat = Mat4::perspective_rh_gl(
            scene_data.fovy.to_radians(),
            width as f32 / height as f32,
            0.1,
            100.0,
        ) * Mat4::from_translation(-scene_data.camera_pos);

        unsafe {
            gl.use_program(Some(self.program));
            gl.enable(glow::DEPTH_TEST);
            gl.clear(glow::DEPTH_BUFFER_BIT);

            // gl.bind_framebuffer(glow::FRAMEBUFFER, intermediate_fbo);

            let p_mat_loc = gl.get_uniform_location(self.program, "uPMatrix").unwrap();
            let mv_mat_loc = gl.get_uniform_location(self.program, "uMVMatrix").unwrap();

            gl.uniform_matrix_4_f32_slice(
                Some(&p_mat_loc),
                false,
                &perspective_mat.to_cols_array(),
            );
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.program, "lightLoc").as_ref(),
                &[0., 5., 5., 17., 5., -2., -17., 5., -2.],
            );
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.program, "lightColor").as_ref(),
                &[1., 1., 1., 1., 1., 1., 1., 1., 1.],
            );
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.program, "lightKdKsCD")
                    .as_ref(),
                &[0.6, 0.3, 20.0, 0.6, 0.3, 20.0, 0.6, 0.3, 20.0],
            );
            gl.uniform_3_f32_slice(
                gl.get_uniform_location(self.program, "ambient_color")
                    .as_ref(),
                &scene_data.ambient,
            );
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "Ka").as_ref(),
                scene_data.ambient_ka,
            );

            let vertex_position_loc = gl
                .get_attrib_location(self.program, "aVertexPosition")
                .unwrap();
            let front_color_loc = gl.get_attrib_location(self.program, "aFrontColor").unwrap();
            let vertex_normal_loc = gl
                .get_attrib_location(self.program, "aVertexNormal")
                .unwrap();

            for obj in scene_data.objs.iter() {
                let obj_model = if let Some(id) = obj.model_id {
                    if let Some(model) = self.models.get(&id) {
                        model
                    } else {
                        &self.default_model
                    }
                } else {
                    &self.default_model
                };

                gl.uniform_matrix_4_f32_slice(
                    Some(&mv_mat_loc),
                    false,
                    &obj.mv_mat.to_cols_array(),
                );

                gl.uniform_1_i32(
                    gl.get_uniform_location(self.program, "mode").as_ref(),
                    obj.mode,
                );

                gl.bind_vertex_array(Some(self.vertex_array));
                gl.bind_framebuffer(glow::FRAMEBUFFER, intermediate_fbo);
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(obj_model.pos_buffer));
                gl.vertex_attrib_pointer_f32(vertex_position_loc, 3, glow::FLOAT, false, 0, 0);
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(obj_model.color_buffer));
                gl.vertex_attrib_pointer_f32(front_color_loc, 3, glow::FLOAT, false, 0, 0);
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(obj_model.norm_buffer));
                gl.vertex_attrib_pointer_f32(vertex_normal_loc, 3, glow::FLOAT, false, 0, 0);

                gl.draw_arrays(glow::TRIANGLES, 0, obj_model.item_count);
                gl.bind_vertex_array(None);
            }

            gl.disable(glow::DEPTH_TEST);
        }
    }
}
