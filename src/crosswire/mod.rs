pub mod types;

use types::*;

use std::process::Command;
use std::{cell::RefCell, rc::Rc};

use pipewire::{
    context::Context,
    keys,
    main_loop::MainLoop,
    registry::{GlobalObject, Registry},
    spa::utils::dict::DictRef,
    types::ObjectType,
};

use glib::clone;

use types::State;

pub fn thread_main(
    egui_sender: async_channel::Sender<PipewireMessage>,
    mut pw_receiver: pipewire::channel::Receiver<GuiMessage>,
) -> anyhow::Result<()> {
    let mainloop = MainLoop::new(None)?;
    let context = Context::new(&mainloop)?;
    let core = context.connect(None)?;
    let registry = Rc::new(RefCell::new(core.get_registry()?));

    let state = Rc::new(RefCell::new(State::default()));

    let receiver = pw_receiver.attach(
        mainloop.loop_(),
        clone!(@strong mainloop, @strong registry, @strong egui_sender => move |msg| match msg {
            GuiMessage::NodeSelected { name, id } => node_selected(&name, id),
            GuiMessage::NodeUnselected { name, id } => node_unselected(&name, id),
            GuiMessage::Terminate => {
                egui_sender.send_blocking(PipewireMessage::OkToClose).expect("Failed to send message");
                mainloop.quit();
            },
        }),
    );

    let _listener = registry
        .borrow_mut()
        .add_listener_local()
        .global(
            clone!(@strong egui_sender, @strong state => move |global| match global.type_ {
                ObjectType::Node => {
                    handle_node(global, &egui_sender, &state);
                }
                _ => (),
            }),
        )
        .global_remove(clone!(@strong egui_sender => move |id| {
            if let Some(Item::Node { .. }) = state.borrow_mut().remove(id) {
                egui_sender
                    .send_blocking(PipewireMessage::NodeRemoved { id })
                    .expect("Failed to send message");
            } else {
                //warn!(
                //    "Attempted to remove item with id {} that is not saved in state",
                //    id
                //);
            }
        }))
        .register();

    mainloop.run();
    pw_receiver = receiver.deattach();

    Ok(())
}

fn handle_node(
    node: &GlobalObject<&DictRef>,
    sender: &async_channel::Sender<PipewireMessage>,
    state: &Rc<RefCell<State>>,
) {
    let props = node
        .props
        .as_ref()
        .expect("Node object is missing properties");

    let node_name = props
        .get(&keys::NODE_DESCRIPTION)
        .or_else(|| props.get(&keys::NODE_NICK))
        .or_else(|| props.get(&keys::NODE_NAME))
        .unwrap_or_default();

    let media_class = |class: &str| {
        if class.contains("Sink") {
            Some(NodeType::Sink)
        } else if class.contains("Source") {
            Some(NodeType::Source)
        } else {
            None
        }
    };

    let node_type: Option<NodeType> = props
        .get("media.category")
        .and_then(|class| {
            if class.contains("Duplex") {
                None
            } else {
                props.get("media.class").and_then(media_class)
            }
        })
        .or_else(|| props.get("media.class").and_then(media_class));

    if let Some(node_type) = node_type {
        if node_type == NodeType::Sink {
            sender
                .send_blocking(PipewireMessage::NodeAdded {
                    name: node_name.to_string(),
                    id: node.id,
                })
                .expect("Failed to send message");
            state.borrow_mut().insert(node.id, Item::Node);
        }
    }
}

fn node_selected(name: &str, id: u32) {
    println!("Node {} with id {} selected", name, id);
}

fn node_unselected(name: &str, id: u32) {
    println!("Node {} with id {} unselected", name, id);
}

fn create_crosswire_sink() {
    let _ = Command::new("sh"
    ).arg("-c").arg(r#"pactl load-module module-null-sink sink_name=Crosswire sink_properties=device.description="Crosswire auido sink""#).spawn().expect("Couldn't create pipewire sink");
}
