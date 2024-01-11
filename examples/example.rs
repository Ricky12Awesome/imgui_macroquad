#![feature(never_type)]

use std::ptr::slice_from_raw_parts;
use std::time::{Duration, Instant};

use imgui::Condition;
use macroquad::prelude::*;

use imgui_macroquad::get_imgui_context;

fn conf() -> Conf {
  Conf {
    window_title: "Example".into(),
    high_dpi: true,
    window_width: 1920,
    window_height: 1080,
    ..Default::default()
  }
}

#[macroquad::main(conf)]
async fn main() {
  // auto-complete sometimes doesn't work here bc macro, so _main
  _main().await.unwrap();
}

const NOTOSANS_FONT: &[u8] = include_bytes!("fonts/NotoSans-Regular.ttf");

async fn _main() -> anyhow::Result<!> {
  let ctx = get_imgui_context();

  let notosans = ctx.add_font_from_bytes("NotoSans-Regular", NOTOSANS_FONT);

  ctx.set_default_font(notosans);

  ctx.setup(|ctx| {
    ctx.set_ini_filename(None);
  });

  let mut buf = String::new();

  let wait = Duration::from_millis(125);
  let mut zoom_wait = Instant::now() - wait;
  let mut font_size = 24f32;

  let w = 2048usize;
  let h = 2048usize;
  let mut pixels = vec![0u32; w * h];

  for y in 0..h {
    for x in 0..w {
      let yp = y as f32 / h as f32;
      let xp = x as f32 / h as f32;

      let mut r = 0.;
      let mut g = 0.;
      let mut b = 0.;

      unsafe {
        imgui::sys::igColorConvertHSVtoRGB(0.1, yp, 1. - xp, &mut r, &mut g, &mut b);
      }

      let rgba = [(255. * r) as u8, (255. * g) as u8, (255. * b) as u8, 255u8];

      pixels[y + w * x] = u32::from_le_bytes(rgba);
    }
  }

  let pixels = slice_from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4);
  let pixels = unsafe { &*pixels };

  let image = Texture2D::from_rgba8(w as _, h as _, pixels);
  let id = ctx.bind_texture_id(image.raw_miniquad_id());

  loop {
    let now = Instant::now();
    clear_background(Color::new(0.16, 0.16, 0.16, 1.));

    ctx.setup_event_handler();

    if is_key_down(KeyCode::LeftControl) {
      let (wait, multi) = if is_key_down(KeyCode::LeftShift) {
        (wait * 2, 2.)
      } else {
        (wait, 1.25)
      };

      if is_key_down(KeyCode::Minus) && now >= zoom_wait {
        font_size /= multi;
        font_size = font_size.floor();
        zoom_wait = now + wait;
        ctx.set_font_size(font_size);
      }

      if is_key_down(KeyCode::Equal) && now >= zoom_wait {
        font_size *= multi;
        font_size = font_size.floor();
        zoom_wait = now + wait;
        ctx.set_font_size(font_size);
      }
    }

    ctx.ui(|ui| {
      ui.show_demo_window(&mut true);
      ui.window("Window")
        .size([900., 900.], Condition::FirstUseEver)
        .build(|| {
          ui.input_text("Input", &mut buf).build();
          ui.image_button("image", id, [512., 512.]);
        });
    });

    ctx.draw();

    next_frame().await;
  }
}
