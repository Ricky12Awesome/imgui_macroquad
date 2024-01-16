#![feature(never_type)]

use std::time::{Duration, Instant};

use imgui::{Condition, FontConfig, FontGlyphRanges};
use itertools::Itertools;
use macroquad::prelude::*;

use imgui_macroquad::{FontFamily, ImGuiContext};

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

const NOTO_SANS_FONT: &[u8] = include_bytes!("fonts/NotoSans-Regular.ttf");
const NOTOSANS_JP_FONT: &[u8] = include_bytes!("fonts/NotoSansJP-Regular.otf");

async fn _main() -> anyhow::Result<!> {
  let mut ctx = ImGuiContext::default();

  let mut noto_sans_family = FontFamily::new("NotoSans-Regular", 16.);

  noto_sans_family.add_font_from_bytes(NOTO_SANS_FONT);
  noto_sans_family.add_font_from_bytes_ex(
    NOTOSANS_JP_FONT,
    FontConfig {
      glyph_ranges: FontGlyphRanges::japanese(),
      ..Default::default()
    },
  );

  let noto_sans = ctx.add_font_family(noto_sans_family);

  ctx.set_default_font(noto_sans);

  ctx.setup(|ctx| {
    ctx.set_ini_filename(None);
  });

  let mut buf = String::new();

  let wait = Duration::from_millis(125);
  let mut zoom_wait = Instant::now() - wait;
  let mut font_size = 24f32;

  let texture = Texture2D::from_image(&gen_image(0.8));
  let id = ctx.bind_texture_id(texture.raw_miniquad_id());

  loop {
    let now = Instant::now();
    clear_background(Color::new(0.16, 0.16, 0.16, 1.));

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

    ctx.ui(|ctx, ui| {
      ui.show_demo_window(&mut true);
      ui.window("Window")
        .size([900., 900.], Condition::FirstUseEver)
        .build(|| {
          ui.text("良い");
          ui.input_text("Input", &mut buf).build();

          for (handle, family) in ctx.get_fonts() {
            let text = format!("[{:?}]: {}, {}", handle.id(), family.name(), family.size());
            ui.text(text);
          }

          if ui.image_button("image", id, [512., 512.]) {
            texture.update(&gen_image(rand::gen_range(0.0, 1.0)))
          }
        });
    });

    ctx.draw();

    next_frame().await;
  }
}

fn gen_image(hue: f32) -> Image {
  let w = 2048usize;
  let h = 2048usize;
  let mut buf = vec![0u32; w * h];

  for y in 0..h {
    for x in 0..w {
      let yp = y as f32 / h as f32;
      let xp = x as f32 / h as f32;

      let mut r = 0.;
      let mut g = 0.;
      let mut b = 0.;

      unsafe {
        imgui::sys::igColorConvertHSVtoRGB(hue, yp, 1. - xp, &mut r, &mut g, &mut b);
      }

      let rgba = [(255. * r) as u8, (255. * g) as u8, (255. * b) as u8, 255u8];

      buf[y + w * x] = u32::from_le_bytes(rgba);
    }
  }

  Image {
    bytes: buf.into_iter().flat_map(u32::to_le_bytes).collect_vec(),
    width: w as _,
    height: h as _,
  }
}
