#![feature(never_type)]

use imgui::Condition;
use macroquad::prelude::*;
use miniquad::window::dpi_scale;

use imgui_macroquad::create_imgui_context;

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
  loop {
    let mut ctx = create_imgui_context();

    clear_background(Color::new(0.16, 0.16, 0.16, 1.));

    ctx.raw_imgui().set_ini_filename(None);

    ctx.ui(|ui| {
      ui.window("Window")
        .size([200., 100.], Condition::Always)
        .build(|| {
          ui.text(format!("{}", dpi_scale()));
        });
    });

    ctx.draw();

    next_frame().await;
  }
}
