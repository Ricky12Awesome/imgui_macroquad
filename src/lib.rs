// mod render;

use ::imgui::{FontConfig, FontId, FontSource};
use imgui::{DrawCmd, Io, Key, Ui};
use miniquad::window::screen_size;
use miniquad::{
  Bindings, BlendFactor, BlendState, BlendValue, BufferLayout, BufferSource, BufferType,
  BufferUsage, Equation, EventHandler, KeyCode, KeyMods, MouseButton, PassAction, Pipeline,
  PipelineParams, RawId, RenderingBackend, ShaderMeta, ShaderSource, TextureId, UniformBlockLayout,
  UniformDesc, UniformType, UniformsSource, VertexAttribute, VertexFormat,
};
use std::cell::RefCell;
use std::rc::Rc;

#[cfg(feature = "macroquad")]
pub use feature_macroquad::*;

/// reexport of imgui
pub mod imgui {
  pub use imgui::*;
}

mod shader {
  use super::*;

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
  font_texture: TextureId,
  default_font: Option<FontIdHandle>,
  fonts: Vec<(FontIdHandle, FontSource<'a>)>,
  textures: Vec<(imgui::TextureId, TextureId)>,
  context: imgui::Context,
  last_frame: f64,
  #[cfg(feature = "macroquad")]
  macroquad_event_id: usize,
}

impl<'a> ImGuiContext<'a> {
  pub fn new(gl: &'a mut dyn RenderingBackend) -> Self {
    let mut context = imgui::Context::create();
    let fonts = context.fonts();
    let font_atlas = fonts.build_rgba32_texture();
    let font_texture = gl.new_texture_from_rgba8(
      font_atlas.width as u16,
      font_atlas.height as u16,
      font_atlas.data,
    );

    setup(&mut context);

    Self {
      gl,
      context,
      font_texture,
      default_font: None,
      fonts: vec![],
      textures: vec![],
      last_frame: miniquad::date::now(),
      #[cfg(feature = "macroquad")]
      macroquad_event_id: macroquad::input::utils::register_input_subscriber(),
    }
  }

  /// TTF Font
  pub fn add_font_from_bytes(&mut self, name: &str, bytes: &'a [u8]) -> FontIdHandle {
    let fonts = self.context.fonts();

    let source = FontSource::TtfData {
      data: bytes,
      size_pixels: 16.,
      config: Some(FontConfig {
        name: Some(name.into()),
        ..Default::default()
      }),
    };

    let id = fonts.add_font(&[source.clone()]);
    let handle = FontIdHandle::new(id);

    self.fonts.push((handle.clone(), source));

    let font_atlas = fonts.build_rgba32_texture();

    self.gl.texture_resize(
      self.font_texture,
      font_atlas.width,
      font_atlas.height,
      Some(font_atlas.data),
    );

    handle
  }

  pub fn set_font_size(&mut self, size: f32) {
    let fonts = self.context.fonts();
    fonts.clear();

    for (handle, source) in &self.fonts {
      let source = match source {
        FontSource::DefaultFontData { config } => FontSource::DefaultFontData {
          config: config.as_ref().map(|config| FontConfig {
            size_pixels: size,
            ..config.clone()
          }),
        },
        FontSource::TtfData {
          data,
          size_pixels: _,
          config,
        } => FontSource::TtfData {
          data,
          size_pixels: size,
          config: config.as_ref().map(|config| FontConfig {
            size_pixels: size,
            ..config.clone()
          }),
        },
      };

      let id = fonts.add_font(&[source]);
      handle.update(id);
    }

    let font_atlas = fonts.build_rgba32_texture();

    self.gl.texture_resize(
      self.font_texture,
      font_atlas.width,
      font_atlas.height,
      Some(font_atlas.data),
    );

    self.context.style_mut().scale_all_sizes(1.0);
  }

  pub fn set_default_font(&mut self, id: FontIdHandle) {
    self.default_font = Some(id);
  }

  pub fn bind_texture_id(&mut self, id: TextureId) -> imgui::TextureId {
    let id = to_imgui_id(id);

    self.textures.push(id);

    id.0
  }

  pub fn raw_imgui(&mut self) -> &mut imgui::Context {
    &mut self.context
  }

  pub fn setup(&mut self, setup: impl FnOnce(&mut imgui::Context)) {
    setup(&mut self.context);
  }

  pub fn style(&mut self, style: impl FnOnce(&mut imgui::Style)) {
    style(self.context.style_mut());
  }

  pub fn ui(&mut self, frame: impl FnOnce(&mut Ui)) {
    self.update();

    let ui = self.context.new_frame();

    // false positive: cannot borrow `*ui` as mutable because it is also borrowed as immutable
    // its fine in this case
    let ui2 = unsafe { ignore_lifetime_mut(ui) };
    let _stack = self.default_font.as_ref().map(|id| ui2.push_font(id.get()));

    frame(ui);
  }

  fn update(&mut self) {
    let io = self.context.io_mut();
    let now = miniquad::date::now();

    io.display_size = screen_size().into();
    io.delta_time = (now - self.last_frame) as _;
    self.last_frame = now;
  }

  pub fn draw(&mut self) {
    #[cfg(feature = "macroquad")]
    unsafe {
      macroquad::window::get_internal_gl().flush();
    }

    let draw_data = self.context.render();
    let pipeline = shader::pipeline(self.gl);
    let (width, height) = screen_size();

    let projection = glam::Mat4::orthographic_rh_gl(0., width, height, 0., -1., 1.);
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

      let mut slice_start = 0;

      for command in draw_list.commands() {
        if let DrawCmd::Elements { count, cmd_params } = command {
          let imgui::DrawCmdParams {
            clip_rect,
            texture_id,
            ..
          } = cmd_params;

          let id = if texture_id.id() == 0 {
            self.font_texture
          } else {
            let (_, id) = self
              .textures
              .iter()
              .find(|(id, _)| *id == texture_id)
              .copied()
              .unwrap();

            id
          };

          let bindings = Bindings {
            vertex_buffers: vec![vtx_buffer],
            index_buffer: idx_buffer,
            images: vec![id],
          };

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

impl<'a> EventHandler for ImGuiContext<'a> {
  fn update(&mut self) {}

  fn draw(&mut self) {}

  fn mouse_motion_event(&mut self, x: f32, y: f32) {
    let io = self.context.io_mut();
    io.mouse_pos = [x, y];
  }

  fn mouse_wheel_event(&mut self, x: f32, y: f32) {
    let io = self.context.io_mut();
    io.mouse_wheel = y / 100.;
    io.mouse_wheel_h = x / 100.;
  }

  fn mouse_button_down_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
    let io = self.context.io_mut();
    let mouse_left = button == MouseButton::Left;
    let mouse_right = button == MouseButton::Right;
    let mouse_middle = button == MouseButton::Middle;

    io.mouse_down = [mouse_left, mouse_right, mouse_middle, false, false];
  }

  fn mouse_button_up_event(&mut self, _button: MouseButton, _x: f32, _y: f32) {
    let io = self.context.io_mut();
    io.mouse_down = [false, false, false, false, false];
  }

  fn char_event(&mut self, character: char, mods: KeyMods, _: bool) {
    let io = self.context.io_mut();

    io.key_ctrl = mods.ctrl;
    io.key_alt = mods.alt;
    io.key_shift = mods.shift;

    io.add_input_character(character);
  }

  fn key_down_event(&mut self, keycode: KeyCode, mods: KeyMods, _: bool) {
    let io = self.context.io_mut();

    // when the keycode is the modifier itself - mods.MODIFIER is false yet, however the modifier button is just pressed and is actually true
    io.key_ctrl = mods.ctrl;
    io.key_alt = mods.alt;
    io.key_shift = mods.shift;

    io.keys_down[keycode as usize] = true;
  }

  fn key_up_event(&mut self, keycode: KeyCode, mods: KeyMods) {
    let io = self.context.io_mut();

    // when the keycode is the modifier itself - mods.MODIFIER is true, however the modifier is actually released
    io.key_ctrl = keycode != KeyCode::LeftControl && keycode != KeyCode::RightControl && mods.ctrl;
    io.key_alt = keycode != KeyCode::LeftAlt && keycode != KeyCode::RightAlt && mods.alt;
    io.key_shift = keycode != KeyCode::LeftShift && keycode != KeyCode::RightShift && mods.shift;

    io.keys_down[keycode as usize] = false;
  }
}

/// Handle for FontId since resizing fonts will give new ids
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontIdHandle(Rc<RefCell<FontId>>);

unsafe impl Send for FontIdHandle {}
unsafe impl Sync for FontIdHandle {}

impl FontIdHandle {
  fn new(id: FontId) -> Self {
    Self(Rc::new(RefCell::new(id)))
  }

  pub fn get(&self) -> FontId {
    *self.0.borrow()
  }

  fn update(&self, id: FontId) {
    *self.0.borrow_mut() = id;
  }
}

impl From<FontId> for FontIdHandle {
  fn from(value: FontId) -> Self {
    Self::new(value)
  }
}

#[allow(clippy::from_over_into)]
impl Into<FontId> for FontIdHandle {
  fn into(self) -> FontId {
    self.get()
  }
}

/// Copied from `miniquad::graphics::TextureIdInner` because it's private I need the info
#[allow(unused)]
#[derive(Clone, Debug, Copy, PartialEq, Eq, Hash)]
enum TextureIdInnerCast {
  Managed(usize),
  Raw(RawId),
}

fn to_imgui_id(texture_id: TextureId) -> (imgui::TextureId, TextureId) {
  let cast = unsafe { std::mem::transmute::<TextureId, TextureIdInnerCast>(texture_id) };

  match cast {
    TextureIdInnerCast::Managed(id) => (id.into(), texture_id),
    TextureIdInnerCast::Raw(RawId::OpenGl(id)) => ((id as usize).into(), texture_id),
    #[cfg(target_vendor = "apple")]
    TextureIdInnerCast::Raw(RawId::Metal(_)) => {
      panic!("Metal not support")
    }
  }
}

/// Here because borrow checker gets in the way of imgui in certain cases
unsafe fn ignore_lifetime_mut<'a, T>(t: &mut T) -> &'a mut T {
  &mut *(t as *mut T)
}

struct Clipboard;

impl imgui::ClipboardBackend for Clipboard {
  fn get(&mut self) -> Option<String> {
    miniquad::window::clipboard_get()
  }

  fn set(&mut self, value: &str) {
    miniquad::window::clipboard_set(value)
  }
}

fn setup(ctx: &mut imgui::Context) {
  ctx.set_clipboard_backend(Clipboard);
  setup_keymap(ctx.io_mut());
}

fn setup_keymap(io: &mut Io) {
  io[Key::Space] = KeyCode::Space as _;
  io[Key::Apostrophe] = KeyCode::Apostrophe as _;
  io[Key::Comma] = KeyCode::Comma as _;
  io[Key::Minus] = KeyCode::Minus as _;
  io[Key::Period] = KeyCode::Period as _;
  io[Key::Slash] = KeyCode::Slash as _;
  io[Key::Alpha0] = KeyCode::Key0 as _;
  io[Key::Alpha1] = KeyCode::Key1 as _;
  io[Key::Alpha2] = KeyCode::Key2 as _;
  io[Key::Alpha3] = KeyCode::Key3 as _;
  io[Key::Alpha4] = KeyCode::Key4 as _;
  io[Key::Alpha5] = KeyCode::Key5 as _;
  io[Key::Alpha6] = KeyCode::Key6 as _;
  io[Key::Alpha7] = KeyCode::Key7 as _;
  io[Key::Alpha8] = KeyCode::Key8 as _;
  io[Key::Alpha9] = KeyCode::Key9 as _;
  io[Key::Semicolon] = KeyCode::Semicolon as _;
  io[Key::Equal] = KeyCode::Equal as _;
  io[Key::A] = KeyCode::A as _;
  io[Key::B] = KeyCode::B as _;
  io[Key::C] = KeyCode::C as _;
  io[Key::D] = KeyCode::D as _;
  io[Key::E] = KeyCode::E as _;
  io[Key::F] = KeyCode::F as _;
  io[Key::G] = KeyCode::G as _;
  io[Key::H] = KeyCode::H as _;
  io[Key::I] = KeyCode::I as _;
  io[Key::J] = KeyCode::J as _;
  io[Key::K] = KeyCode::K as _;
  io[Key::L] = KeyCode::L as _;
  io[Key::M] = KeyCode::M as _;
  io[Key::N] = KeyCode::N as _;
  io[Key::O] = KeyCode::O as _;
  io[Key::P] = KeyCode::P as _;
  io[Key::Q] = KeyCode::Q as _;
  io[Key::R] = KeyCode::R as _;
  io[Key::S] = KeyCode::S as _;
  io[Key::T] = KeyCode::T as _;
  io[Key::U] = KeyCode::U as _;
  io[Key::V] = KeyCode::V as _;
  io[Key::W] = KeyCode::W as _;
  io[Key::X] = KeyCode::X as _;
  io[Key::Y] = KeyCode::Y as _;
  io[Key::Z] = KeyCode::Z as _;
  io[Key::LeftBracket] = KeyCode::LeftBracket as _;
  io[Key::Backslash] = KeyCode::Backslash as _;
  io[Key::RightBracket] = KeyCode::RightBracket as _;
  io[Key::GraveAccent] = KeyCode::GraveAccent as _;
  io[Key::Escape] = KeyCode::Escape as _;
  io[Key::Enter] = KeyCode::Enter as _;
  io[Key::Tab] = KeyCode::Tab as _;
  io[Key::Backspace] = KeyCode::Backspace as _;
  io[Key::Insert] = KeyCode::Insert as _;
  io[Key::Delete] = KeyCode::Delete as _;
  io[Key::RightArrow] = KeyCode::Right as _;
  io[Key::LeftArrow] = KeyCode::Left as _;
  io[Key::DownArrow] = KeyCode::Down as _;
  io[Key::UpArrow] = KeyCode::Up as _;
  io[Key::PageUp] = KeyCode::PageUp as _;
  io[Key::PageDown] = KeyCode::PageDown as _;
  io[Key::Home] = KeyCode::Home as _;
  io[Key::End] = KeyCode::End as _;
  io[Key::CapsLock] = KeyCode::CapsLock as _;
  io[Key::ScrollLock] = KeyCode::ScrollLock as _;
  io[Key::NumLock] = KeyCode::NumLock as _;
  io[Key::PrintScreen] = KeyCode::PrintScreen as _;
  io[Key::Pause] = KeyCode::Pause as _;
  io[Key::F1] = KeyCode::F1 as _;
  io[Key::F2] = KeyCode::F2 as _;
  io[Key::F3] = KeyCode::F3 as _;
  io[Key::F4] = KeyCode::F4 as _;
  io[Key::F5] = KeyCode::F5 as _;
  io[Key::F6] = KeyCode::F6 as _;
  io[Key::F7] = KeyCode::F7 as _;
  io[Key::F8] = KeyCode::F8 as _;
  io[Key::F9] = KeyCode::F9 as _;
  io[Key::F10] = KeyCode::F10 as _;
  io[Key::F11] = KeyCode::F11 as _;
  io[Key::F12] = KeyCode::F12 as _;
  io[Key::Keypad0] = KeyCode::Kp0 as _;
  io[Key::Keypad1] = KeyCode::Kp1 as _;
  io[Key::Keypad2] = KeyCode::Kp2 as _;
  io[Key::Keypad3] = KeyCode::Kp3 as _;
  io[Key::Keypad4] = KeyCode::Kp4 as _;
  io[Key::Keypad5] = KeyCode::Kp5 as _;
  io[Key::Keypad6] = KeyCode::Kp6 as _;
  io[Key::Keypad7] = KeyCode::Kp7 as _;
  io[Key::Keypad8] = KeyCode::Kp8 as _;
  io[Key::Keypad9] = KeyCode::Kp9 as _;
  io[Key::KeypadDecimal] = KeyCode::KpDecimal as _;
  io[Key::KeypadDivide] = KeyCode::KpDivide as _;
  io[Key::KeypadMultiply] = KeyCode::KpMultiply as _;
  io[Key::KeypadSubtract] = KeyCode::KpSubtract as _;
  io[Key::KeypadAdd] = KeyCode::KpAdd as _;
  io[Key::KeypadEnter] = KeyCode::KpEnter as _;
  io[Key::KeypadEqual] = KeyCode::KpEqual as _;
  io[Key::LeftShift] = KeyCode::LeftShift as _;
  io[Key::LeftCtrl] = KeyCode::LeftControl as _;
  io[Key::LeftAlt] = KeyCode::LeftAlt as _;
  io[Key::LeftSuper] = KeyCode::LeftSuper as _;
  io[Key::RightShift] = KeyCode::RightShift as _;
  io[Key::RightCtrl] = KeyCode::RightControl as _;
  io[Key::RightAlt] = KeyCode::RightAlt as _;
  io[Key::RightSuper] = KeyCode::RightSuper as _;
  io[Key::Menu] = KeyCode::Menu as _;
  io[Key::Tab] = KeyCode::Tab as _;
}

#[cfg(feature = "macroquad")]
mod feature_macroquad {
  use super::*;
  use macroquad::input::utils::repeat_all_miniquad_input;

  impl ImGuiContext<'_> {
    pub fn setup_event_handler(&mut self) {
      repeat_all_miniquad_input(self, self.macroquad_event_id);
    }
  }

  /// Because I can't store ImGuiContext in a static global var,
  /// and I don't want to impl Send/Sync for it
  struct ImGuiContextSendSyncSpoof<'a>(ImGuiContext<'a>);

  unsafe impl<'a> Send for ImGuiContextSendSyncSpoof<'a> {}
  unsafe impl<'a> Sync for ImGuiContextSendSyncSpoof<'a> {}

  static mut CONTEXT: Option<ImGuiContextSendSyncSpoof> = None;

  pub fn get_imgui_context<'a>() -> &'static mut ImGuiContext<'a> {
    unsafe {
      // here since this also check if it's in the same thread
      let gl = macroquad::window::get_internal_gl();

      let context = CONTEXT.get_or_insert_with(|| {
        let ctx = ImGuiContext::new(gl.quad_context);
        ImGuiContextSendSyncSpoof(ctx)
      });

      &mut context.0
    }
  }
}
