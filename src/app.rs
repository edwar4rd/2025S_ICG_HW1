use std::sync::Arc;

use eframe::egui_glow;
use egui::{mutex::Mutex, Checkbox, RichText, Slider};
use egui_glow::glow;
use strum::{EnumIter, IntoEnumIterator, IntoStaticStr};

#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    serde::Deserialize,
    serde::Serialize,
    Default,
    IntoStaticStr,
    EnumIter,
)]
enum RenderingMode {
    Flat = 0,
    Gouraud,
    #[default]
    Phong,
    FakeFlat,
    Cartoon,
}

#[derive(Clone, serde::Deserialize, serde::Serialize)]
struct CGObject {
    name: String,
    translation: [f32; 3],
    rotation: [f32; 3],
    rotating: [f32; 3],
    scale: [f32; 3],
    shear: [f32; 3],
    rendering_mode: RenderingMode,
    json_url: String,
}

impl Default for CGObject {
    fn default() -> Self {
        Self {
            name: "Object".into(),
            translation: Default::default(),
            rotation: Default::default(),
            rotating: Default::default(),
            scale: [1., 1., 1.],
            shear: [90., 90., 90.],
            rendering_mode: Default::default(),
            json_url: Default::default(),
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
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize, Default)]
pub struct DemoApp {
    ambient: [f32; 3],
    rotation_enabled: bool,
    selected_object: Option<usize>,
    angle: f32,
    objects: Vec<CGObject>,
    dummy_object: CGObject,
    #[serde(skip)]
    gl_stuff: Arc<Mutex<Option<GLStuff>>>,
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
        let dt = ctx.input(|i| i.unstable_dt);
        if self.rotation_enabled {
            self.angle += 1.0 * dt;
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
            ui.add(Slider::new(&mut self.ambient[0], 0.0..=1.0).text("Red"));
            ui.add(Slider::new(&mut self.ambient[1], 0.0..=1.0).text("Green"));
            ui.add(Slider::new(&mut self.ambient[2], 0.0..=1.0).text("Blue"));
            ui.separator();

            ui.heading("Objects");
            ui.horizontal(|ui| {
                if ui.button("New Object").clicked() {
                    let mut new_obj = self.dummy_object.clone();
                    new_obj.name = format!("Object{:02}", self.objects.len());
                    self.objects.push(new_obj);
                    if self.selected_object == None {
                        self.selected_object = Some(self.objects.len() - 1);
                    }
                }

                if ui
                    .button(RichText::new("Clear Objects").color(egui::Color32::RED))
                    .clicked()
                {
                    self.objects.clear();
                    self.selected_object = None;

                    // GL Stuff cleanup?
                }
            });
            ui.add_space(10.);

            egui::ComboBox::new("obj", "Selected Object")
                .selected_text(
                    self.selected_object
                        .and_then(|obj_id| self.objects.get(obj_id))
                        .and_then(|obj| Some(obj.name.as_str()))
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
                    .and_then(|obj| Some(obj))
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

                ui.collapsing("Scale", |ui| {
                    ui.add(Slider::new(&mut selected_obj.scale[0], 0.0..=10.0).text("Scale.x"));
                    ui.add(Slider::new(&mut selected_obj.scale[1], 0.0..=10.0).text("Scale.y"));
                    ui.add(Slider::new(&mut selected_obj.scale[2], 0.0..=10.0).text("Scale.z"));
                    if ui.button("Reset Scale").clicked() {
                        selected_obj.scale = [1., 1., 1.];
                    }
                });

                ui.collapsing("Translation", |ui| {
                    ui.add(
                        Slider::new(&mut selected_obj.translation[0], -10.0..=10.0)
                            .text("Translation.x"),
                    );
                    ui.add(
                        Slider::new(&mut selected_obj.translation[1], -10.0..=10.0)
                            .text("Translation.y"),
                    );
                    ui.add(
                        Slider::new(&mut selected_obj.translation[2], -10.0..=10.0)
                            .text("Translation.z"),
                    );
                    if ui.button("Reset Translation").clicked() {
                        selected_obj.translation = [0., 0., 0.];
                    }
                });
                ui.collapsing("Rotation", |ui| {
                    ui.add(
                        Slider::new(&mut selected_obj.rotation[0], -180.0..=180.0)
                            .text("Rotation.x"),
                    );
                    ui.add(
                        Slider::new(&mut selected_obj.rotation[1], -180.0..=180.0)
                            .text("Rotation.y"),
                    );
                    ui.add(
                        Slider::new(&mut selected_obj.rotation[2], -180.0..=180.0)
                            .text("Rotation.z"),
                    );
                    if ui.button("Reset Rotation").clicked() {
                        selected_obj.rotation = [0., 0., 0.];
                    }
                });
                ui.collapsing("Shear", |ui| {
                    ui.add(Slider::new(&mut selected_obj.shear[0], 0.0..=180.0).text("Shear.x"));
                    ui.add(Slider::new(&mut selected_obj.shear[1], 0.0..=180.0).text("Shear.y"));
                    ui.add(Slider::new(&mut selected_obj.shear[2], 0.0..=180.0).text("Shear.z"));

                    if ui.button("Reset Shear").clicked() {
                        selected_obj.shear = [90., 90., 90.];
                    }
                });
                ui.collapsing("Animation", |ui| {
                    ui.add(
                        Slider::new(&mut selected_obj.rotating[0], -360.0..=360.0)
                            .text("Rotating.x"),
                    );
                    ui.add(
                        Slider::new(&mut selected_obj.rotating[1], -360.0..=360.0)
                            .text("Rotating.y"),
                    );
                    ui.add(
                        Slider::new(&mut selected_obj.rotating[2], -360.0..=360.0)
                            .text("Rotating.z"),
                    );

                    if ui.button("Reset Animation").clicked() {
                        selected_obj.rotating = [0., 0., 0.];
                    }
                });
            }

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
            self.gl_stuff.lock().as_ref().map(|stuff| stuff.destroy(gl));
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
    fn custom_painting(&mut self, ui: &mut egui::Ui) {
        let (rect, response) =
            // ui.allocate_exact_size(egui::Vec2::splat(300.0), egui::Sense::drag());
        ui.allocate_exact_size(ui.available_size(), egui::Sense::drag());

        self.angle += response.drag_motion().x * 0.01;

        // Clone locals so we can move them into the paint callback:
        let angle = self.angle;
        let gl_stuff = self.gl_stuff.clone();

        let cb = egui_glow::CallbackFn::new(move |info, painter| {
            let width = info.clip_rect_in_pixels().width_px;
            let height = info.clip_rect_in_pixels().height_px;

            gl_stuff
                .lock()
                .as_ref()
                .map(|stuff| stuff.paint(painter.gl(), angle, width, height));
        });

        let callback = egui::PaintCallback {
            rect,
            callback: Arc::new(cb),
        };
        ui.painter().add(callback);
    }
}

struct GLStuff {
    program: glow::Program,
    vertex_array: glow::VertexArray,
}

#[allow(unsafe_code)] // we need unsafe code to use glow
impl GLStuff {
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

            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");

            Some(Self {
                program,
                vertex_array,
            })
        }
    }

    fn destroy(&self, gl: &glow::Context) {
        use glow::HasContext as _;
        unsafe {
            gl.delete_program(self.program);
            gl.delete_vertex_array(self.vertex_array);
        }
    }

    fn paint(&self, gl: &glow::Context, angle: f32, width: i32, height: i32) {
        use glow::HasContext as _;
        unsafe {
            gl.viewport(0, 0, width, height);
            gl.use_program(Some(self.program));
            gl.uniform_1_f32(
                gl.get_uniform_location(self.program, "u_angle").as_ref(),
                angle,
            );
            gl.bind_vertex_array(Some(self.vertex_array));
            gl.draw_arrays(glow::TRIANGLES, 0, 3);
        }
    }
}
