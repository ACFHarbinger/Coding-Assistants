//! Child webview for the DeepSeek web GUI inside the terminal grid.
//!
//! `add_child` is behind Tauri's `unstable` feature because child webviews
//! still have platform caveats (WebView2 child embedding). The remote page
//! is not listed in any capability `remote` URL, so its injected IPC bridge
//! cannot run app commands. Pop-out windows are denied. Only public https
//! navigations are accepted.

use std::net::IpAddr;

use tauri::{AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl};

const LABEL_PREFIX: &str = "deepseek-gui-";
const URL_ERROR: &str = "browser pane only opens public https pages";

#[derive(serde::Deserialize)]
pub struct BrowserPaneBounds {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    visible: bool,
}

pub fn navigation_allowed(url: &tauri::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some_and(|host| !host_blocked(host))
}

pub fn parse_browser_url(raw: &str) -> Result<tauri::Url, String> {
    let raw = raw.trim();
    if raw.is_empty() || raw.len() > 2048 {
        return Err(URL_ERROR.into());
    }
    let url = tauri::Url::parse(raw).map_err(|_| URL_ERROR.to_string())?;
    if !navigation_allowed(&url) {
        return Err(URL_ERROR.into());
    }
    Ok(url)
}

fn host_blocked(host: &str) -> bool {
    // `Url::host_str` keeps the brackets around an IPv6 address.
    let host = host
        .trim_end_matches('.')
        .trim_matches(|ch| ch == '[' || ch == ']')
        .to_ascii_lowercase();
    if host.is_empty()
        || host == "localhost"
        || host.ends_with(".localhost")
        || host == "tauri.localhost"
        || host.ends_with(".tauri.localhost")
    {
        return true;
    }
    host.parse::<IpAddr>().is_ok_and(ip_blocked)
}

fn ip_blocked(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            ip.is_loopback()
                || ip.is_private()
                || ip.is_link_local()
                || ip.is_unspecified()
                || ip.is_multicast()
                || ip.is_broadcast()
                || ip.is_documentation()
        }
        IpAddr::V6(ip) => {
            ip.is_loopback()
                || ip.is_unspecified()
                || ip.is_multicast()
                || ip.is_unique_local()
                || ip.is_unicast_link_local()
        }
    }
}

fn pane_label(leaf_id: &str) -> Result<String, String> {
    if leaf_id.is_empty()
        || leaf_id.len() > 64
        || !leaf_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err("browser pane id is invalid".into());
    }
    Ok(format!("{LABEL_PREFIX}{leaf_id}"))
}

fn check_bounds(bounds: &BrowserPaneBounds) -> Result<(), String> {
    for (name, value) in [
        ("x", bounds.x),
        ("y", bounds.y),
        ("width", bounds.width),
        ("height", bounds.height),
    ] {
        if !value.is_finite() {
            return Err(format!("browser pane {name} is not a finite number"));
        }
    }
    if !(-8000.0..=20_000.0).contains(&bounds.x) || !(-8000.0..=20_000.0).contains(&bounds.y) {
        return Err("browser pane position is outside the window".into());
    }
    if !(0.0..=20_000.0).contains(&bounds.width) || !(0.0..=20_000.0).contains(&bounds.height) {
        return Err("browser pane size is outside the allowed range".into());
    }
    Ok(())
}

fn shown(bounds: &BrowserPaneBounds) -> bool {
    bounds.visible && bounds.width >= 8.0 && bounds.height >= 8.0
}

fn apply_bounds(webview: &tauri::Webview, bounds: &BrowserPaneBounds) -> Result<(), String> {
    check_bounds(bounds)?;
    if !shown(bounds) {
        webview.hide().map_err(|err| err.to_string())?;
        return Ok(());
    }
    webview
        .set_position(LogicalPosition::new(bounds.x, bounds.y))
        .map_err(|err| err.to_string())?;
    webview
        .set_size(LogicalSize::new(bounds.width, bounds.height))
        .map_err(|err| err.to_string())?;
    webview.show().map_err(|err| err.to_string())?;
    Ok(())
}

fn main_window(app: &AppHandle) -> Result<tauri::Window, String> {
    app.get_window("main")
        .ok_or_else(|| "main window is not open".to_string())
}

#[tauri::command]
pub async fn browser_pane_open(
    app: AppHandle,
    leaf_id: String,
    url: String,
    bounds: BrowserPaneBounds,
) -> Result<(), String> {
    let label = pane_label(&leaf_id)?;
    let url = parse_browser_url(&url)?;
    check_bounds(&bounds)?;
    if let Some(existing) = app.get_webview(&label) {
        existing.navigate(url).map_err(|err| err.to_string())?;
        return apply_bounds(&existing, &bounds);
    }
    let window = main_window(&app)?;
    let builder = tauri::webview::WebviewBuilder::new(&label, WebviewUrl::External(url))
        .on_navigation(navigation_allowed)
        .on_new_window(|_url, _features| tauri::webview::NewWindowResponse::Deny)
        .disable_drag_drop_handler()
        .enable_clipboard_access()
        .devtools(cfg!(debug_assertions))
        .incognito(false);
    let width = bounds.width.max(1.0);
    let height = bounds.height.max(1.0);
    let webview = window
        .add_child(
            builder,
            LogicalPosition::new(bounds.x, bounds.y),
            LogicalSize::new(width, height),
        )
        .map_err(|err| err.to_string())?;
    apply_bounds(&webview, &bounds)
}

#[tauri::command]
pub async fn browser_pane_place(
    app: AppHandle,
    leaf_id: String,
    bounds: BrowserPaneBounds,
) -> Result<(), String> {
    let label = pane_label(&leaf_id)?;
    let Some(webview) = app.get_webview(&label) else {
        return Ok(());
    };
    apply_bounds(&webview, &bounds)
}

#[tauri::command]
pub async fn browser_pane_navigate(
    app: AppHandle,
    leaf_id: String,
    url: String,
) -> Result<(), String> {
    let label = pane_label(&leaf_id)?;
    let url = parse_browser_url(&url)?;
    let webview = app
        .get_webview(&label)
        .ok_or_else(|| "browser pane is not open".to_string())?;
    webview.navigate(url).map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn browser_pane_close(app: AppHandle, leaf_id: String) -> Result<(), String> {
    let label = pane_label(&leaf_id)?;
    if let Some(webview) = app.get_webview(&label) {
        webview.close().map_err(|err| err.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_url_is_the_deepseek_gui() {
        let url = parse_browser_url("https://chat.deepseek.com/").unwrap();
        assert_eq!(url.host_str(), Some("chat.deepseek.com"));
        assert!(navigation_allowed(&url));
    }

    #[test]
    fn rejects_non_public_targets() {
        for raw in [
            "",
            "http://chat.deepseek.com/",
            "javascript:alert(1)",
            "file:///etc/passwd",
            "data:text/html,hi",
            "https://user:secret@chat.deepseek.com/",
            "https://localhost/",
            "https://app.localhost/page",
            "https://tauri.localhost/",
            "https://127.0.0.1/",
            "https://10.1.2.3/admin",
            "https://192.168.0.8/",
            "https://[::1]/",
            "https://[fd00::1]/",
            "not a url",
        ] {
            assert!(parse_browser_url(raw).is_err(), "{raw}");
        }
    }

    #[test]
    fn allows_other_public_https_pages() {
        let url = parse_browser_url("https://example.com/docs?q=1#top").unwrap();
        assert_eq!(url.host_str(), Some("example.com"));
    }

    #[test]
    fn pane_label_rejects_path_characters() {
        assert!(pane_label("leaf-browser-1").is_ok());
        assert!(pane_label("../main").is_err());
        assert!(pane_label("leaf/browser").is_err());
        assert!(pane_label("").is_err());
    }
}
