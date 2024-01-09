#![feature(never_type)]

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

async fn _main() -> anyhow::Result<!> {
  let ctx = get_imgui_context();

  ctx.raw_imgui().set_ini_filename(None);

  let mut buf = String::new();

  let wait = Duration::from_millis(125);
  let mut zoom = Instant::now() - wait;

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
        .size([200., 100.], Condition::FirstUseEver)
        .build(|| {
          ui.text(format!("{}", scale));
          ui.input_text("Input", &mut buf).build();
        });
    });

    ctx.draw();

    next_frame().await;
  }
}
