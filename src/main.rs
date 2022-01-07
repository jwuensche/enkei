use wayland_client::{Main, global_filter};


use std::sync::{Arc, RwLock, mpsc::channel};

use wayland_client::{
    protocol::{wl_compositor, wl_output},
    Display, GlobalManager,
};

mod outputs;
mod output;
mod schema;
mod opengl;
mod metadata;
mod messages;
mod watchdog;
mod worker;

use thiserror::Error;

use khronos_egl as egl;
// global api object
use egl::API as egl;

use outputs::{
    Output,
    handle_output_events,
};

#[derive(Error, Debug)]
pub enum ApplicationError {
    #[error("Could not access the member `{0}` in some struct.")]
    AccessError(String),
}

impl<'a> From<outputs::OutputError<'a>> for ApplicationError {
    fn from(err: outputs::OutputError<'a>) -> Self {
        match err {
            outputs::OutputError::KeyNotDefined(key) => Self::AccessError(key.into()),
        }
    }
}

fn main() -> Result<(), ApplicationError> {
    let display = Display::connect_to_env().unwrap();
    let mut event_queue = display.create_event_queue();
    let attached_display = (*display).clone().attach(event_queue.token());

    let wl_outputs = Arc::new(RwLock::new(Vec::new()));
    let pass_outputs = Arc::clone(&wl_outputs);

    let (message_tx, message_rx) = channel();
    let tx = message_tx.clone();
    let globals = GlobalManager::new_with_cb(
        &attached_display,
        // Let's use the global filter macro provided with the wayland-client crate here
        // The advantage of this that we will get all initially advertised objects (like WlOutput) as a freebe here and don't have to concern with getting
        // all available ones later.
        global_filter!(
            [wl_output::WlOutput, 2, move |output: Main<wl_output::WlOutput>, _: DispatchData| {
                println!("Got a new WlOutput instance!");
                let new_output = Arc::new(RwLock::new(Output::new(output.clone())));
                let pass = Arc::clone(&new_output);
                let added = tx.clone();
                output.quick_assign(move |_, event, _| {
                    handle_output_events(&pass, event, &added);
                });
                let mut lock = pass_outputs.write().unwrap();
                lock.push(new_output);
                drop(lock);
            }]
        )
    );
    event_queue
        .sync_roundtrip(&mut (), |_, _, _| unreachable!())
        .unwrap();

    /*
     * Initialize Watchdogs for Suspension Cycles
    */
    watchdog::sleeping::initialize(message_tx.clone());



    worker::work(globals, display, message_rx, message_tx, event_queue)?;
    Ok(())
}
