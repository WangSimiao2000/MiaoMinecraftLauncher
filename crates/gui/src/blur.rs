use std::sync::Arc;

use eframe::egui;
use egui_glow::glow;
use glow::HasContext;

const BLUR_VERTEX_SHADER: &str = r#"#version 330 core
layout(location = 0) in vec2 a_pos;
layout(location = 1) in vec2 a_uv;
out vec2 v_uv;
void main() {
    gl_Position = vec4(a_pos, 0.0, 1.0);
    v_uv = a_uv;
}
"#;

const BLUR_FRAGMENT_SHADER: &str = r#"#version 330 core
in vec2 v_uv;
out vec4 frag_color;
uniform sampler2D u_texture;
uniform vec2 u_direction;
uniform vec2 u_resolution;

void main() {
    // 9-tap Gaussian kernel (sigma ~= 4.0)
    float weights[5] = float[](0.2270270, 0.1945946, 0.1216216, 0.0540540, 0.0162162);

    vec2 texel = u_direction / u_resolution;
    vec4 color = texture(u_texture, v_uv) * weights[0];

    for (int i = 1; i < 5; i++) {
        color += texture(u_texture, v_uv + texel * float(i)) * weights[i];
        color += texture(u_texture, v_uv - texel * float(i)) * weights[i];
    }

    frag_color = color;
}
"#;

const COMPOSITE_FRAGMENT_SHADER: &str = r#"#version 330 core
in vec2 v_uv;
out vec4 frag_color;
uniform sampler2D u_texture;
uniform vec4 u_tint;

void main() {
    vec4 blurred = texture(u_texture, v_uv);
    frag_color = mix(blurred, u_tint, u_tint.a);
}
"#;

pub struct BlurRenderer {
    blur_program: glow::Program,
    composite_program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    fbo_a: glow::Framebuffer,
    fbo_b: glow::Framebuffer,
    tex_a: glow::Texture,
    tex_b: glow::Texture,
    allocated_size: (u32, u32),
    gl: Arc<glow::Context>,
}

impl BlurRenderer {
    pub fn new(gl: &Arc<glow::Context>) -> Self {
        unsafe {
            let blur_program = Self::create_program(gl, BLUR_VERTEX_SHADER, BLUR_FRAGMENT_SHADER);
            let composite_program =
                Self::create_program(gl, BLUR_VERTEX_SHADER, COMPOSITE_FRAGMENT_SHADER);

            let vao = gl.create_vertex_array().unwrap();
            let vbo = gl.create_buffer().unwrap();

            #[rustfmt::skip]
            let vertices: [f32; 24] = [
                // pos      // uv
                -1.0, -1.0, 0.0, 0.0,
                 1.0, -1.0, 1.0, 0.0,
                 1.0,  1.0, 1.0, 1.0,
                -1.0, -1.0, 0.0, 0.0,
                 1.0,  1.0, 1.0, 1.0,
                -1.0,  1.0, 0.0, 1.0,
            ];

            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            gl.buffer_data_u8_slice(
                glow::ARRAY_BUFFER,
                bytemuck::cast_slice(&vertices),
                glow::STATIC_DRAW,
            );
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(0, 2, glow::FLOAT, false, 16, 0);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(1, 2, glow::FLOAT, false, 16, 8);
            gl.bind_vertex_array(None);

            let (tex_a, fbo_a) = Self::create_fbo_texture(gl, 16, 16);
            let (tex_b, fbo_b) = Self::create_fbo_texture(gl, 16, 16);

            Self {
                blur_program,
                composite_program,
                vao,
                vbo,
                fbo_a,
                fbo_b,
                tex_a,
                tex_b,
                allocated_size: (16, 16),
                gl: Arc::clone(gl),
            }
        }
    }

    fn ensure_size(&mut self, width: u32, height: u32) {
        if self.allocated_size == (width, height) {
            return;
        }
        unsafe {
            let gl = &self.gl;
            gl.delete_texture(self.tex_a);
            gl.delete_texture(self.tex_b);
            gl.delete_framebuffer(self.fbo_a);
            gl.delete_framebuffer(self.fbo_b);

            let (tex_a, fbo_a) = Self::create_fbo_texture(gl, width, height);
            let (tex_b, fbo_b) = Self::create_fbo_texture(gl, width, height);
            self.tex_a = tex_a;
            self.tex_b = tex_b;
            self.fbo_a = fbo_a;
            self.fbo_b = fbo_b;
            self.allocated_size = (width, height);
        }
    }

    pub fn render_blur(
        &mut self,
        screen_pixels: [u32; 2],
        rect_pixels: egui::Rect,
        radius: f32,
        tint: [f32; 4],
    ) {
        let x = rect_pixels.left() as u32;
        let y = rect_pixels.top() as u32;
        let w = rect_pixels.width() as u32;
        let h = rect_pixels.height() as u32;

        if w == 0 || h == 0 {
            return;
        }

        self.ensure_size(w, h);

        let gl = &self.gl;
        let passes = (radius / 4.0).ceil().max(1.0) as u32;

        unsafe {
            gl.bind_texture(glow::TEXTURE_2D, Some(self.tex_a));
            gl.copy_tex_sub_image_2d(
                glow::TEXTURE_2D,
                0,
                0,
                0,
                x as i32,
                (screen_pixels[1] - y - h) as i32,
                w as i32,
                h as i32,
            );

            gl.bind_vertex_array(Some(self.vao));
            gl.use_program(Some(self.blur_program));

            let res_loc = gl.get_uniform_location(self.blur_program, "u_resolution");
            let dir_loc = gl.get_uniform_location(self.blur_program, "u_direction");

            gl.uniform_2_f32(res_loc.as_ref(), w as f32, h as f32);

            for _ in 0..passes {
                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo_b));
                gl.viewport(0, 0, w as i32, h as i32);
                gl.bind_texture(glow::TEXTURE_2D, Some(self.tex_a));
                gl.uniform_2_f32(dir_loc.as_ref(), 1.0, 0.0);
                gl.draw_arrays(glow::TRIANGLES, 0, 6);

                gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.fbo_a));
                gl.viewport(0, 0, w as i32, h as i32);
                gl.bind_texture(glow::TEXTURE_2D, Some(self.tex_b));
                gl.uniform_2_f32(dir_loc.as_ref(), 0.0, 1.0);
                gl.draw_arrays(glow::TRIANGLES, 0, 6);
            }

            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, screen_pixels[0] as i32, screen_pixels[1] as i32);

            gl.enable(glow::SCISSOR_TEST);
            gl.scissor(
                x as i32,
                (screen_pixels[1] - y - h) as i32,
                w as i32,
                h as i32,
            );

            gl.use_program(Some(self.composite_program));
            let tint_loc = gl.get_uniform_location(self.composite_program, "u_tint");
            gl.uniform_4_f32(tint_loc.as_ref(), tint[0], tint[1], tint[2], tint[3]);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.tex_a));

            gl.draw_arrays(glow::TRIANGLES, 0, 6);

            gl.disable(glow::SCISSOR_TEST);
            gl.bind_vertex_array(None);
            gl.use_program(None);
        }
    }

    unsafe fn create_program(gl: &glow::Context, vs_src: &str, fs_src: &str) -> glow::Program {
        unsafe {
            let program = gl.create_program().unwrap();
            let vs = gl.create_shader(glow::VERTEX_SHADER).unwrap();
            gl.shader_source(vs, vs_src);
            gl.compile_shader(vs);
            assert!(
                gl.get_shader_compile_status(vs),
                "VS: {}",
                gl.get_shader_info_log(vs)
            );

            let fs = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
            gl.shader_source(fs, fs_src);
            gl.compile_shader(fs);
            assert!(
                gl.get_shader_compile_status(fs),
                "FS: {}",
                gl.get_shader_info_log(fs)
            );

            gl.attach_shader(program, vs);
            gl.attach_shader(program, fs);
            gl.link_program(program);
            assert!(
                gl.get_program_link_status(program),
                "Link: {}",
                gl.get_program_info_log(program)
            );

            gl.delete_shader(vs);
            gl.delete_shader(fs);
            program
        }
    }

    unsafe fn create_fbo_texture(
        gl: &glow::Context,
        width: u32,
        height: u32,
    ) -> (glow::Texture, glow::Framebuffer) {
        unsafe {
            let tex = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(tex));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::RGBA8 as i32,
                width as i32,
                height as i32,
                0,
                glow::RGBA,
                glow::UNSIGNED_BYTE,
                glow::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_S,
                glow::CLAMP_TO_EDGE as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_WRAP_T,
                glow::CLAMP_TO_EDGE as i32,
            );

            let fbo = gl.create_framebuffer().unwrap();
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(fbo));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::COLOR_ATTACHMENT0,
                glow::TEXTURE_2D,
                Some(tex),
                0,
            );
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            (tex, fbo)
        }
    }
}

impl Drop for BlurRenderer {
    fn drop(&mut self) {
        unsafe {
            let gl = &self.gl;
            gl.delete_program(self.blur_program);
            gl.delete_program(self.composite_program);
            gl.delete_vertex_array(self.vao);
            gl.delete_buffer(self.vbo);
            gl.delete_texture(self.tex_a);
            gl.delete_texture(self.tex_b);
            gl.delete_framebuffer(self.fbo_a);
            gl.delete_framebuffer(self.fbo_b);
        }
    }
}

pub fn blur_behind(
    ui: &mut egui::Ui,
    blur_renderer: &Arc<std::sync::Mutex<BlurRenderer>>,
    rect: egui::Rect,
    radius: f32,
    tint: egui::Color32,
) {
    let renderer = Arc::clone(blur_renderer);
    let callback = egui::PaintCallback {
        rect,
        callback: Arc::new(egui_glow::CallbackFn::new(move |info, painter| {
            let mut renderer = renderer.lock().unwrap();
            let screen = info.screen_size_px;
            let clip = info.clip_rect_in_pixels();
            let rect_pixels = egui::Rect::from_min_max(
                egui::pos2(clip.left_px as f32, clip.top_px as f32),
                egui::pos2(
                    (clip.left_px + clip.width_px) as f32,
                    (clip.top_px + clip.height_px) as f32,
                ),
            );

            let tint_f = [
                tint.r() as f32 / 255.0,
                tint.g() as f32 / 255.0,
                tint.b() as f32 / 255.0,
                tint.a() as f32 / 255.0,
            ];

            renderer.render_blur(screen, rect_pixels, radius, tint_f);

            unsafe {
                let gl = painter.gl();
                gl.bind_framebuffer(glow::FRAMEBUFFER, painter.intermediate_fbo());
            }
        })),
    };
    ui.painter().add(callback);
}
