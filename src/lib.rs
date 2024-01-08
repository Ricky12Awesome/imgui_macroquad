// mod render;

use imgui::{DrawCmd, Ui};
use macroquad::input::mouse_position;
use macroquad::prelude::{screen_height, screen_width};
use miniquad::{
  Bindings, BufferSource, BufferType, BufferUsage, PassAction, RenderingBackend, TextureId,
  UniformsSource,
};
use miniquad::window::screen_size;

pub mod shader {
  use miniquad::{
    BlendFactor, BlendState, BlendValue, BufferLayout, Equation, Pipeline, PipelineParams,
    RenderingBackend, ShaderMeta, ShaderSource, UniformBlockLayout, UniformDesc, UniformType,
    VertexAttribute, VertexFormat,
  };

  pub fn pipeline(ctx: &mut dyn RenderingBackend) -> Pipeline {
    let shader = ctx
      .new_shader(
        ShaderSource::Glsl {
          fragment: FRAGMENT,
          vertex: VERTEX,
        },
        meta(),
      )
      .unwrap();

    ctx.new_pipeline_with_params(
      &[BufferLayout::default()],
      &[
        VertexAttribute::new("position", VertexFormat::Float2),
        VertexAttribute::new("texcoord", VertexFormat::Float2),
        VertexAttribute::new("color0", VertexFormat::Byte4),
      ],
      shader,
      PipelineParams {
        color_blend: Some(BlendState::new(
          Equation::Add,
          BlendFactor::Value(BlendValue::SourceAlpha),
          BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
        )),
        ..Default::default()
      },
    )
  }

  pub const VERTEX: &str = r#"#version 100
    attribute vec2 position;
    attribute vec2 texcoord;
    attribute vec4 color0;

    varying lowp vec2 uv;
    varying lowp vec4 color;

    uniform mat4 Projection;

    void main() {
        gl_Position = Projection * vec4(position, 0, 1);
        gl_Position.z = 0.;
        color = color0 / 255.0;
        uv = texcoord;
    }"#;

  pub const FRAGMENT: &str = r#"#version 100
    varying lowp vec4 color;
    varying lowp vec2 uv;

    uniform sampler2D Texture;

    void main() {
        gl_FragColor = color * texture2D(Texture, uv);
    }"#;

  pub fn meta() -> ShaderMeta {
    ShaderMeta {
      images: vec!["Texture".to_string()],
      uniforms: UniformBlockLayout {
        uniforms: vec![UniformDesc::new("Projection", UniformType::Mat4)],
      },
    }
  }

  #[repr(C)]
  #[derive(Debug)]
  pub struct Uniforms {
    pub projection: glam::Mat4,
  }
}

pub struct ImGuiContext<'a> {
  gl: &'a mut dyn RenderingBackend,
  font_atlas: TextureId,
  context: imgui::Context,
}

impl<'a> ImGuiContext<'a> {
  // noinspection RsBorrowChecker
  pub fn new(gl: &'a mut dyn RenderingBackend) -> Self {
    let mut context = imgui::Context::create();
    let font_atlas = context.fonts().build_rgba32_texture();
    let font_atlas = gl.new_texture_from_rgba8(
      font_atlas.width as u16,
      font_atlas.height as u16,
      font_atlas.data,
    );

    Self {
      // Intellij-Rust/RustRover is stupid
      // false-positive: The value was moved out while it was still borrowed
      gl,
      font_atlas,
      context,
    }
  }

  pub fn update(&mut self) {
    let io = self.context.io_mut();

    io.display_size = [screen_width(), screen_height()];
    io.mouse_pos = mouse_position().into();
    io.font_global_scale = 1.5;
  }

  pub fn raw_imgui(&mut self) -> &mut imgui::Context {
    &mut self.context
  } 

  pub fn ui(&mut self, frame: impl FnOnce(&mut Ui)) {
    self.update();
    let ui = self.context.new_frame();
    frame(ui);
  }

  pub fn draw(&mut self) {
    #[cfg(feature = "macroquad")]
    unsafe {
      macroquad::window::get_internal_gl().flush();
    }

    let draw_data = self.context.render();
    let pipeline = shader::pipeline(self.gl);
    let (width, height) = screen_size();

    let projection = ::glam::Mat4::orthographic_rh_gl(0., width, height, 0., -1., 1.);
    let uniform = shader::Uniforms { projection };

    self.gl.apply_pipeline(&pipeline);
    self.gl.begin_default_pass(PassAction::Nothing);

    let clip_off = draw_data.display_pos;
    let clip_scale = draw_data.framebuffer_scale;

    for draw_list in draw_data.draw_lists() {
      let vtx_buffer = self.gl.new_buffer(
        BufferType::VertexBuffer,
        BufferUsage::Stream,
        BufferSource::slice(draw_list.vtx_buffer()),
      );

      let idx_buffer = self.gl.new_buffer(
        BufferType::IndexBuffer,
        BufferUsage::Stream,
        BufferSource::slice(draw_list.idx_buffer()),
      );

      let bindings = Bindings {
        vertex_buffers: vec![vtx_buffer],
        index_buffer: idx_buffer,
        images: vec![self.font_atlas],
      };

      let mut slice_start = 0;

      for command in draw_list.commands() {
        if let DrawCmd::Elements { count, cmd_params } = command {
          let imgui::DrawCmdParams { clip_rect, .. } = cmd_params;

          let clip_rect = [
            (clip_rect[0] - clip_off[0]) * clip_scale[0],
            (clip_rect[1] - clip_off[1]) * clip_scale[1],
            (clip_rect[2] - clip_off[0]) * clip_scale[0],
            (clip_rect[3] - clip_off[1]) * clip_scale[1],
          ];
          let h = clip_rect[3] - clip_rect[1];

          self.gl.apply_scissor_rect(
            clip_rect[0] as i32,
            height as i32 - (clip_rect[1] + h) as i32,
            (clip_rect[2] - clip_rect[0]) as i32,
            h as i32,
          );

          self.gl.apply_bindings(&bindings);
          self.gl.apply_uniforms(UniformsSource::table(&uniform));
          self.gl.draw(slice_start, count as i32, 1);
          slice_start += count as i32;
        }
      }
    }

    self.gl.end_render_pass();
  }
}

#[cfg(feature = "macroquad")]
pub fn create_imgui_context<'a>() -> ImGuiContext<'a> {
  let gl = unsafe { macroquad::window::get_internal_gl() };
  ImGuiContext::new(gl.quad_context)
}
