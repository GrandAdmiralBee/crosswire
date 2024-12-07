mod crosswire;
mod gui;

fn main() {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let (pw_sender, pw_receiver) = pipewire::channel::channel();
    let (egui_sender, egui_receiver) = async_channel::unbounded();
    let pw_thread = std::thread::spawn(move || crosswire::thread_main(egui_sender, pw_receiver));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Crosswire",
        options,
        Box::new(|_cc| {
            Ok(Box::new(gui::CrosswireWindow::new(
                pw_sender,
                egui_receiver,
            )))
        }),
    )
    .unwrap();
}
