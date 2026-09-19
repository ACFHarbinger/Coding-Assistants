//! Desktop subscriber for [`hub::InProcessBus`] (`platform.md` P1, #325).
//!
//! The bus itself lives in `hub` and does not know about Tauri. This
//! forwarder is one subscriber among potentially many: it copies each event
//! onto `AppHandle::emit` so existing frontend `listen` calls keep working.

use hub::{EventBus, InProcessBus};
use tauri::{AppHandle, Emitter};

pub fn spawn_tauri_forwarder(bus: InProcessBus, app: AppHandle) {
    let rx = bus.subscribe();
    std::thread::Builder::new()
        .name("ca-bus-tauri".into())
        .spawn(move || {
            while let Ok(event) = rx.recv() {
                let _ = app.emit(&event.topic, event.payload);
            }
        })
        .expect("ca-bus-tauri forwarder thread");
}
