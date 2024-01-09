#![feature(never_type)]

use std::ptr::slice_from_raw_parts;
use std::time::{Duration, Instant};

use imgui::Condition;
use macroquad::color::hsl_to_rgb;
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

async fn _main() -> anyhow::Result<!> {
  let ctx = get_imgui_context();

  ctx.setup(|ctx| {
    ctx.set_ini_filename(None);
  });

  let mut buf = String::new();

  let wait = Duration::from_millis(125);
  let mut zoom = Instant::now() - wait;

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
        imgui::sys::igColorConvertHSVtoRGB(0.1, yp, 1.- xp, &mut r, &mut g, &mut b);
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
        (wait * 2, 1.)
      } else {
        (wait, 0.25)
      };

      if is_key_down(KeyCode::Minus) && now >= zoom {
        ctx.raw_imgui().io_mut().font_global_scale -= multi;
        zoom = now + wait;
      }

      if is_key_down(KeyCode::Equal) && now >= zoom {
        ctx.raw_imgui().io_mut().font_global_scale += multi;
        zoom = now + wait;
      }
    }

    let scale = ctx.raw_imgui().io_mut().font_global_scale;

    ctx.ui(|ui| {
      ui.show_demo_window(&mut true);
      ui.window("Window")
        .size([900., 900.], Condition::FirstUseEver)
        .build(|| {
          ui.text(format!("{}", scale));
          ui.text(format!("{:?}", id));
          ui.text(format!("{:?}", image.raw_miniquad_id()));

          ui.input_text("Input", &mut buf).build();
          ui.image_button("image", id, [512., 512.]);
        });
    });

    ctx.draw();

    next_frame().await;
  }
}
