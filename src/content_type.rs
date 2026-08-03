pub fn get(uri: &str) -> String {
    // Use only the parsed path for MIME detection, so query strings and fragments cannot become
    // part of the extension. Callers may also pass a relative asset path rather than a full URL.
    let path = url::Url::parse(uri)
        .ok()
        .map(|uri| uri.path().to_owned())
        .unwrap_or_else(|| {
            uri.split_once(['?', '#'])
                .map_or(uri, |(path, _)| path)
                .to_owned()
        });

    // Unknown content is binary by default; labeling arbitrary bytes as text can corrupt assets.
    mime_guess::from_path(path)
        .first_or_octet_stream()
        .essence_str()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::get;

    #[test]
    fn returns_webassembly_streaming_mime_type() {
        assert_eq!(
            get("req://webroot/pkg/app.wasm?cache=1"),
            "application/wasm"
        );
    }

    #[test]
    fn ignores_queries_on_relative_asset_paths() {
        assert_eq!(get("nested/module.js?v=2"), "text/javascript");
    }

    #[test]
    fn uses_binary_mime_type_for_unknown_files() {
        assert_eq!(get("asset.unknown-extension"), "application/octet-stream");
    }
}
