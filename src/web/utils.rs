pub fn rate_limited_page(user_id: &str) -> String {
    format!(r#"<!doctype html><html><head><meta charset="utf-8"><title>Too many</title></head>
<body><h1>Slow down</h1><p>User {user_id} hit the per-minute limit. Please retry shortly.</p></body></html>"#)
}

pub fn error_page(err: &str) -> String {
    let safe = html_escape::encode_text(err);
    format!(
        r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Error</title></head>
<body><h1>Oops!</h1><p>Failed to generate HTML.</p><pre style="white-space:pre-wrap">{safe}</pre></body></html>"#
    )
}