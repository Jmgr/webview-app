use std::{thread, time::Duration};

use webview_app::{application::Application, webview::WebView};

#[test]
fn request_quit_closes_a_running_application() {
    let app = Application::new("de.uriegel.webviewapp.request.quit.test");

    app.on_activate(move |app| {
        let webview = WebView::builder(app).url("about:blank").build();
        webview.connect_request(|request, _, command, _| {
            if command == "quit" {
                request.quit_application();
                true
            } else {
                false
            }
        });

        let handle = webview.get_handle();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(1));
            WebView::eval(handle, "WebView.request('quit', {})");
        });

        webview
    });

    assert_eq!(app.run(), 0);
}
