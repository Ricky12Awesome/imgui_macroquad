#![feature(never_type)]

use imgui::Condition;
use macroquad::prelude::*;
use miniquad::window::dpi_scale;

use imgui_macroquad::get_imgui_context;

fn conf() -> Conf {
  Conf {
    window_title: "Example".into(),
    high_dpi: true,
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

  loop {
    clear_background(Color::new(0.16, 0.16, 0.16, 1.));

    ctx.setup_event_handler();

    ctx.ui(|ui| {
      ui.show_demo_window(&mut true);
      ui.window("Window")
        .size([200., 100.], Condition::FirstUseEver)
        .build(|| {
          ui.text(format!("{}", dpi_scale()));
          ui.input_text("Input", &mut buf).build();
        });
    });

    ctx.draw();

    next_frame().await;
  }
}
