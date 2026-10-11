use std::{env, fs, path::PathBuf};

use gtk::prelude::*;
use gtk::ApplicationWindow;

use crate::request::Request;
use crate::{bounds::Bounds, params::Params};

use super::webkitview::WebViewHandle;
use super::webkitview::{WebkitViewParams, WebkitView};

#[derive(Clone)]
pub struct WebView {
    window: ApplicationWindow,
    pub webview: WebkitView
}

impl WebView {
    pub fn new(params: Params)->Self {
        let config_dir = 
        if params.save_bounds
            {config_dir(&params.app.get_appid())} 
        else
            {None};
        let bounds = config_dir
            .as_deref()
            .and_then(Bounds::restore)
            .unwrap_or(params.bounds);

        let webkitview_params = WebkitViewParams {
            url: &params.url.unwrap_or("about:plain"),
            debug_url: params.debug_url,
            query_string: params.query_string,
            default_contextmenu: params.default_contextmenu,
            devtools: params.devtools,
            console_logging: params.console_logging,
            background_color: params.background_color,
            webroot: params.webroot,
        };

        let builder = 
            match params.builder_path {
                Some(builder_path) if params.with_builder.is_some() => 
                    gtk::Builder::from_resource(&builder_path),
                _ => 
                    gtk::Builder::from_string(get_resource_ui()) 
            };
        let webview = WebkitView::new(&builder, webkitview_params);

        let window: ApplicationWindow = builder.object("window").unwrap();
        params.title.inspect(|t| window.set_title(Some(t)));
        window.set_application(Some(&params.app.app.app));
        window.set_default_width(bounds.width.unwrap_or(800));
        window.set_default_height(bounds.height.unwrap_or(800));

        if let Some(config_dir) = config_dir {
            let gtkwindow = window.clone();
            window.connect_close_request(move|_| {
                let bounds = Bounds {
                    x: None,
                    y: None,
                    width: Some(gtkwindow.width()),
                    height: Some(gtkwindow.height()),
                    is_maximized: gtkwindow.is_maximized()
                };
                bounds.save(&config_dir);
                false.into()
            });
        }   
        if let Some(with_builder) = params.with_builder {
            with_builder(&builder);
        }

        window.present();           
        webview.webview.grab_focus();
        Self {
            window,
            webview 
        }
    }

    pub fn can_close(&self, val: impl Fn()->bool + 'static) {
        self.window.connect_close_request(move|_| (val() == false).into());
    }

    // TODO &str
    pub fn connect_request<F: Fn(&Request, String, String, String) -> bool + 'static>(
        &self,
        on_request: F,
    ) {
        self.webview.connect_request(on_request);
    }   

    pub fn get_handle(&self)->WebViewHandle {
        self.webview.get_handle()
    }

    pub fn start_evaluate_script(handle: crate::webview::WebViewHandle, script: &str) {    
        WebkitView::start_evaluate_script(handle, script);
    }
}

fn get_resource_ui()->&'static str {
r##"
<?xml version='1.0' encoding='UTF-8'?>
<interface>
    <requires lib="gtk" version="4.12"/>
    <requires lib="libadwaita" version="1.0"/>
    <requires lib="webkitgtk" version="6.0"/>
    <object class="GtkApplicationWindow" id="window">
        <property name="titlebar">
            <object class="AdwHeaderBar"/>
        </property>
        <child>
            <object class="WebKitWebView" type-func="webkit_web_view_get_type" id="webview">
                <property name="settings">
                    <object class="WebKitSettings" id="webkit_settings" />
                </property>
            </object>
       </child>
    </object>
</interface>
"##
}

/// The directory window bounds are saved in: `$XDG_CONFIG_HOME/<appid>`, or `~/.config/<appid>`
/// when `XDG_CONFIG_HOME` is not set. Returns `None`, so that bounds are not saved, when neither
/// variable is set or the directory cannot be created.
fn config_dir(appid: &str)->Option<String> {
    // The XDG base directory specification says to ignore relative (and empty) paths.
    let base = env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    let Some(base) = base else {
        eprintln!("Neither XDG_CONFIG_HOME nor HOME is set, window bounds will not be saved");
        return None;
    };

    let config_dir = base.join(appid);
    match fs::create_dir_all(&config_dir) {
        Ok(()) => Some(config_dir.to_string_lossy().to_string()),
        Err(err) => {
            eprintln!("Could not create {}, window bounds will not be saved: {:?}", config_dir.display(), err);
            None
        }
    }
}
