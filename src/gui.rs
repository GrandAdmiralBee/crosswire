use crate::crosswire::types::*;

pub struct CrosswireWindow {
    pw_sender: pipewire::channel::Sender<GuiMessage>,
    egui_receiver: async_channel::Receiver<PipewireMessage>,

    checkboxes: Vec<CrosswireNode>,
    ok_to_close: bool,
}

impl CrosswireWindow {
    pub fn new(
        pw_sender: pipewire::channel::Sender<GuiMessage>,
        egui_receiver: async_channel::Receiver<PipewireMessage>,
    ) -> Self {
        Self {
            pw_sender,
            egui_receiver,
            checkboxes: Vec::new(),
            ok_to_close: false,
        }
    }

    fn handle_events(&mut self) {
        if let Ok(msg) = self.egui_receiver.try_recv() {
            match msg {
                PipewireMessage::OkToClose => self.ok_to_close = true,
                PipewireMessage::NodeAdded { name, id } => self.checkboxes.push(CrosswireNode {
                    node: Node { name, id },
                    selected: false,
                }),
                PipewireMessage::NodeRemoved { id } => self.checkboxes.retain(|x| x.node.id != id),
            }
        }
    }
}

impl eframe::App for CrosswireWindow {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events();
        if self.ok_to_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Select audio devices for output");
            self.checkboxes.iter_mut().for_each(|checkbox| {
                if ui
                    .checkbox(&mut checkbox.selected, checkbox.node.name.clone())
                    .clicked()
                {
                    if checkbox.selected {
                        self.pw_sender
                            .send(GuiMessage::NodeSelected {
                                name: checkbox.node.name.clone(),
                                id: checkbox.node.id,
                            })
                            .expect("Failed to send message");
                    } else {
                        self.pw_sender
                            .send(GuiMessage::NodeUnselected {
                                name: checkbox.node.name.clone(),
                                id: checkbox.node.id,
                            })
                            .expect("Failed to send message");
                    }
                }
            })
        });

        if ctx.input(|i| i.viewport().close_requested()) {
            if !self.ok_to_close {
                self.pw_sender
                    .send(GuiMessage::Terminate)
                    .expect("Failed to send message");
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }
    }
}
