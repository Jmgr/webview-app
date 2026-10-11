#![cfg(target_os = "windows")]

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

use include_dir::{Dir, DirEntry, File};
use webview_app::{application::Application, webview::WebView};

static NESTED_ENTRIES: &[DirEntry<'static>] = &[
    DirEntry::File(File::new(
        "nested/dynamic.js",
        br#"export default "dynamic-import-loaded";"#,
    )),
    DirEntry::File(File::new("nested/data.json", br#"{"fetched":true}"#)),
    DirEntry::File(File::new("nested/empty.wasm", b"\0asm\x01\0\0\0")),
];

static ROOT_ENTRIES: &[DirEntry<'static>] = &[
    DirEntry::File(File::new(
        "index.html",
        br#"<!doctype html>
<link rel="stylesheet" href="./style.css?theme=regression">
<script type="module" src="./main.js?entry=1"></script>"#,
    )),
    DirEntry::File(File::new("style.css", b"body { color: rgb(1, 2, 3); }")),
    DirEntry::File(File::new(
        "static.js",
        br#"export const staticValue = "static-import-loaded";"#,
    )),
    DirEntry::File(File::new(
        "main.js",
        br#"import { staticValue } from "./static.js?module=1";

const signal = (command, data = {}) => {
    void WebView.request(command, data);
};

try {
    const dynamicValue = (await import("./nested/dynamic.js?module=2")).default;
    const fetched = await fetch("./nested/data.json?request=1").then(response => response.json());
    const wasm = await WebAssembly.instantiateStreaming(
        fetch("./nested/empty.wasm?cache=1")
    );
    const missingStatus = await fetch("./nested/missing.bin?request=2")
        .then(response => response.status);
    const stylesheetLoaded = getComputedStyle(document.body).color === "rgb(1, 2, 3)";

    if (staticValue !== "static-import-loaded"
        || dynamicValue !== "dynamic-import-loaded"
        || fetched.fetched !== true
        || !(wasm.instance instanceof WebAssembly.Instance)
        || missingStatus !== 404
        || !stylesheetLoaded) {
        throw new Error("A custom-scheme resource assertion failed");
    }

    signal("custom-scheme-regression-complete");
} catch (error) {
    signal("custom-scheme-regression-error", { message: String(error) });
}"#,
    )),
    DirEntry::Dir(Dir::new("nested", NESTED_ENTRIES)),
];

static WEBROOT: Dir<'static> = Dir::new("", ROOT_ENTRIES);

#[test]
fn custom_scheme_loads_module_wasm_and_nested_resources() {
    let app = Application::new("de.uriegel.webviewapp.custom-scheme-regression");
    let completed = Arc::new(AtomicBool::new(false));
    let failure = Arc::new(Mutex::new(None));

    app.on_activate({
        let completed = Arc::clone(&completed);
        let failure = Arc::clone(&failure);
        move |app| {
            let webview = WebView::builder(app).webroot(WEBROOT.clone()).build();
            let handle = webview.get_handle();
            let timeout_completed = Arc::clone(&completed);
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(15));
                if !timeout_completed.load(Ordering::Acquire) {
                    WebView::eval(
                        handle,
                        "void WebView.request('custom-scheme-regression-timeout', {})",
                    );
                }
            });

            webview.connect_request({
                let completed = Arc::clone(&completed);
                let failure = Arc::clone(&failure);
                move |request, _, command, json| {
                    match command.as_str() {
                        "custom-scheme-regression-complete" => {
                            completed.store(true, Ordering::Release);
                        }
                        "custom-scheme-regression-error" => {
                            *failure.lock().unwrap() = Some(json);
                        }
                        "custom-scheme-regression-timeout" => {
                            *failure.lock().unwrap() = Some("timed out".to_string());
                        }
                        _ => return false,
                    }
                    request.quit_application();
                    true
                }
            });
            webview
        }
    });

    assert_eq!(app.run(), 0);
    assert!(
        completed.load(Ordering::Acquire),
        "custom-scheme regression failed: {:?}",
        *failure.lock().unwrap()
    );
}
