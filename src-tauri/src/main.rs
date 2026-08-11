// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // On Linux + Wayland + NVIDIA, GTK's native Wayland/EGL rendering path
    // causes severe GPU stalls in WebKitGTK at large (maximized) window sizes.
    // Force X11/XWayland, which uses NVIDIA's stable GLX path instead.
    // Must be set here in main(), before any GDK/GTK initialisation occurs.
    // GDK reads GDK_BACKEND at gtk_init time; setting it inside tauri::Builder
    // (in lib.rs run()) is too late because the session default
    // (GDK_BACKEND=wayland) has already been consumed.
    #[cfg(target_os = "linux")]
    {
        // Only override if not explicitly set by the caller.
        if std::env::var("GDK_BACKEND").is_err() {
            // SAFETY: single-threaded at this point — no other threads exist yet.
            unsafe {
                std::env::set_var("GDK_BACKEND", "x11");
                std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
            }
        }
    }

    tauri_app_lib::run()
}
