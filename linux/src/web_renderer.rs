use gtk::prelude::*;
use webkit::prelude::*;
use std::cell::RefCell;
use std::path::{Path, PathBuf};

use pulldown_cmark::{html as cmark_html, Options, Parser};

use crate::renderer::Renderer;

const HIGHLIGHT_JS: &str = include_str!("../resources/highlight.min.js");
const MERMAID_JS: &str = include_str!("../resources/mermaid.min.js");
const JSZIP_JS: &str = include_str!("../resources/jszip.min.js");
const DOCX_PREVIEW_JS: &str = include_str!("../resources/docx-preview.js");

pub struct WebRenderer {
    webview: webkit::WebView,
    current_path: RefCell<Option<PathBuf>>,
}

impl WebRenderer {
    pub const fn extensions() -> &'static [&'static str] {
        &[
            // Word docs (rendered via docmod CLI)
            "docx", "docmod", "doct",
            // HTML
            "html", "htm",
            // Markdown
            "md", "markdown",
            // Code — languages
            "swift", "cs", "py", "js", "ts", "tsx", "jsx", "go", "rs", "rb", "java",
            "kt", "scala", "c", "h", "cpp", "hpp", "m", "mm", "lua", "r", "pl",
            "php", "dart", "zig", "nim", "ex", "exs", "erl", "hs", "ml", "fs",
            "v", "sv", "vhdl", "asm", "s", "sql",
            // Shell
            "sh", "bash", "zsh", "fish", "bat", "ps1", "cmd",
            // Data / config
            "xml", "json", "yaml", "yml", "toml", "ini", "cfg", "conf",
            "csv", "tsv", "plist", "graphql", "proto",
            // Web / styles
            "css", "scss", "sass", "less",
            // Docs / text
            "rst", "txt", "log", "diff", "patch",
            // Config files
            "env", "editorconfig", "gitignore", "gitattributes", "dockerignore",
            "makefile", "cmake", "gradle",
            // Other
            "lock", "sum", "mod",
        ]
    }

    pub fn supports(ext: &str) -> bool {
        Self::extensions().contains(&ext)
    }

    pub fn new() -> Self {
        let webview = webkit::WebView::new();
        Self {
            webview,
            current_path: RefCell::new(None),
        }
    }

    fn ext_lower(path: &Path) -> String {
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default()
    }

    fn file_uri(path: &Path) -> String {
        glib::filename_to_uri(path, None)
            .map(|g| g.to_string())
            .unwrap_or_else(|_| format!("file://{}", path.to_string_lossy()))
    }

    fn lang_for(ext: &str) -> String {
        match ext {
            "rs" => "rust".into(),
            "py" => "python".into(),
            "js" | "jsx" => "javascript".into(),
            "ts" | "tsx" => "typescript".into(),
            "sh" | "bash" | "zsh" => "bash".into(),
            "yml" | "yaml" => "yaml".into(),
            "md" | "markdown" => "markdown".into(),
            "h" => "c".into(),
            "hpp" => "cpp".into(),
            "m" | "mm" => "objectivec".into(),
            "kt" => "kotlin".into(),
            "cs" => "csharp".into(),
            "rb" => "ruby".into(),
            "pl" => "perl".into(),
            "ex" | "exs" => "elixir".into(),
            "erl" => "erlang".into(),
            "hs" => "haskell".into(),
            "ml" => "ocaml".into(),
            "fs" => "fsharp".into(),
            "asm" | "s" => "x86asm".into(),
            "fish" => "shell".into(),
            "bat" | "cmd" => "dos".into(),
            "ps1" => "powershell".into(),
            "toml" | "cfg" | "conf" => "ini".into(),
            "plist" | "svg" => "xml".into(),
            "sass" => "scss".into(),
            "rst" | "txt" | "log" => "plaintext".into(),
            "patch" => "diff".into(),
            "proto" => "protobuf".into(),
            other => other.to_string(),
        }
    }

    fn load_html_file(&self, path: &Path) {
        // Preferred: let WebKit handle relative paths natively via file URI.
        let uri = Self::file_uri(path);
        self.webview.load_uri(&uri);
    }

    fn load_docx_file(&self, path: &Path) {
        // Render .docx natively in the browser via docx-preview (docxjs):
        // read the raw bytes, base64-encode them, and let the embedded
        // script rehydrate the bytes into a Blob + renderAsync.
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                self.show_error(&format!("Failed to read file: {}", e));
                return;
            }
        };

        use base64::{engine::general_purpose::STANDARD, Engine as _};
        let b64 = STANDARD.encode(&bytes);

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="color-scheme" content="light dark">
<style>
  :root {{ color-scheme: light dark; }}
  html, body {{ margin: 0; padding: 0; background: #e5e7eb; }}
  #container {{ padding: 20px 0; }}
  #container .docx-wrapper {{ background: #9CA3AF; padding: 30px; display: flex; flex-flow: column; align-items: center; }}
  #container .docx-wrapper > section.docx {{ background: #fff; box-shadow: 0 0 10px rgba(0,0,0,0.5); margin-bottom: 30px; }}
  #container .docx {{ color: #000; }}
  @media (prefers-color-scheme: dark) {{
    html, body {{ background: #1a1a1a; }}
    #container .docx-wrapper {{ background: #2a2a2a; }}
  }}
  .status {{ font-family: system-ui, sans-serif; padding: 40px; color: #333; text-align: center; }}
  .status.err {{ color: #c00; }}
</style>
<script>{jszip}</script>
<script>{docxjs}</script>
</head>
<body>
<div id="container"><div class="status">Rendering…</div></div>
<script>
(function() {{
  var b64 = "{b64}";
  try {{
    var bin = atob(b64);
    var len = bin.length;
    var bytes = new Uint8Array(len);
    for (var i = 0; i < len; i++) bytes[i] = bin.charCodeAt(i);
    var blob = new Blob([bytes], {{ type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document" }});
    var container = document.getElementById("container");
    container.innerHTML = "";
    docx.renderAsync(blob, container, null, {{
      className: "docx",
      inWrapper: true,
      ignoreWidth: false,
      ignoreHeight: false,
      ignoreFonts: false,
      breakPages: true,
      ignoreLastRenderedPageBreak: true,
      experimental: false,
      trimXmlDeclaration: true,
      useBase64URL: true,
      renderChanges: false,
      renderHeaders: true,
      renderFooters: true,
      renderFootnotes: true,
      renderEndnotes: true,
      debug: false
    }}).catch(function(err) {{
      container.innerHTML = '<div class="status err">Render failed: ' + (err && err.message ? err.message : err) + '</div>';
    }});
  }} catch (err) {{
    document.getElementById("container").innerHTML = '<div class="status err">Decode failed: ' + err.message + '</div>';
  }}
}})();
</script>
</body>
</html>"#,
            jszip = JSZIP_JS,
            docxjs = DOCX_PREVIEW_JS,
            b64 = b64,
        );

        let base_uri = Self::file_uri(path);
        self.webview.load_html(&html, Some(&base_uri));
    }

    fn load_docmod_file(&self, path: &Path) {
        match crate::docmod_cli::render(path) {
            Ok(html) => {
                // docmod produces a full HTML doc. Load directly so its
                // embedded styles/scripts run as-is.
                let base_uri = Self::file_uri(path);
                self.webview.load_html(&html, Some(&base_uri));
            }
            Err(msg) => {
                self.show_error(&msg);
            }
        }
    }

    fn load_markdown_file(&self, path: &Path) {
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.show_error(&format!("Failed to read file: {}", e));
                return;
            }
        };

        let mut opts = Options::empty();
        opts.insert(Options::ENABLE_TABLES);
        opts.insert(Options::ENABLE_FOOTNOTES);
        opts.insert(Options::ENABLE_STRIKETHROUGH);
        opts.insert(Options::ENABLE_TASKLISTS);
        opts.insert(Options::ENABLE_SMART_PUNCTUATION);
        opts.insert(Options::ENABLE_HEADING_ATTRIBUTES);

        let parser = Parser::new_ext(&raw, opts);
        let mut body = String::new();
        cmark_html::push_html(&mut body, parser);

        let wrapped = Self::wrap_document(&body);
        let base_uri = Self::file_uri(path);
        self.webview.load_html(&wrapped, Some(&base_uri));
    }

    fn load_code_file(&self, path: &Path) {
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                self.show_error(&format!("Failed to read file: {}", e));
                return;
            }
        };
        let ext = Self::ext_lower(path);
        let lang = Self::lang_for(&ext);
        let escaped = html_escape::encode_text(&raw).to_string();
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();
        let filename_escaped = html_escape::encode_text(&filename).to_string();
        let line_count = raw.lines().count().max(if raw.is_empty() { 0 } else { 1 });

        let body = format!(
            "<div class=\"file-header\">{} lines &middot; {}</div>\n<pre><code class=\"language-{}\">{}</code></pre>",
            line_count, filename_escaped, lang, escaped
        );

        let wrapped = Self::wrap_document(&body);
        let base_uri = Self::file_uri(path);
        self.webview.load_html(&wrapped, Some(&base_uri));
    }

    fn show_error(&self, message: &str) {
        let escaped = html_escape::encode_text(message).to_string();
        let html = format!(
            r#"<!DOCTYPE html><html><head><meta charset="utf-8">
<style>
body {{ font-family: system-ui, -apple-system, "Segoe UI", sans-serif; padding: 40px; color: #333; background: #fff; }}
h2 {{ color: #c00; }}
pre {{ white-space: pre-wrap; word-wrap: break-word; background: #f5f5f5; padding: 12px; border-radius: 6px; }}
@media (prefers-color-scheme: dark) {{
  body {{ background: #1a1a1a; color: #d4d4d4; }}
  pre {{ background: #252525; }}
}}
</style></head><body>
<h2>Error</h2>
<pre>{}</pre>
</body></html>"#,
            escaped
        );
        self.webview.load_html(&html, None);
    }

    /// Wraps rendered HTML body in the full template (head + scripts + styles).
    fn wrap_document(body_html: &str) -> String {
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="color-scheme" content="light dark">
<style>
:root {{
  color-scheme: light dark;
}}
* {{ box-sizing: border-box; }}
body {{
  font-family: system-ui, -apple-system, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
  font-size: 15px;
  line-height: 1.6;
  color: #1a1a1a;
  background: #ffffff;
  max-width: 900px;
  margin: 0 auto;
  padding: 24px 32px;
}}
h1, h2, h3, h4, h5, h6 {{ margin: 1em 0 0.5em; line-height: 1.25; }}
h1 {{ font-size: 1.8em; border-bottom: 1px solid #e5e7eb; padding-bottom: 0.3em; }}
h2 {{ font-size: 1.4em; border-bottom: 1px solid #e5e7eb; padding-bottom: 0.3em; }}
h3 {{ font-size: 1.2em; }}
p {{ margin: 0.75em 0; }}
a {{ color: #2563eb; }}
code {{
  font-family: "JetBrains Mono", "Fira Code", "SF Mono", Menlo, Consolas, monospace;
  font-size: 0.9em;
  background: #f3f4f6;
  padding: 2px 5px;
  border-radius: 3px;
}}
pre {{
  background: #f6f8fa;
  border-radius: 6px;
  overflow-x: auto;
  margin: 1em 0;
  padding: 12px 14px;
  line-height: 1.5;
}}
pre code {{
  background: none;
  padding: 0;
  font-size: 0.88em;
  border-radius: 0;
}}
blockquote {{
  margin: 1em 0;
  padding: 0 1em;
  border-left: 4px solid #d1d5db;
  color: #6b7280;
}}
table {{ border-collapse: collapse; width: 100%; margin: 1em 0; }}
th, td {{ border: 1px solid #d1d5db; padding: 8px 12px; text-align: left; }}
th {{ background: #f9fafb; font-weight: 600; }}
img {{ max-width: 100%; }}
hr {{ border: none; border-top: 1px solid #e5e7eb; margin: 2em 0; }}
ul, ol {{ padding-left: 2em; }}
li {{ margin: 0.25em 0; }}
.mermaid {{ display: flex; justify-content: center; margin: 1em 0; }}
.mermaid svg {{ max-width: 100%; height: auto; }}
.file-header {{
  color: #888;
  font-size: 12px;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid #e5e7eb;
}}
@media (prefers-color-scheme: dark) {{
  body {{ background: #1a1a1a; color: #d4d4d4; }}
  h1, h2 {{ border-bottom-color: #333; }}
  code {{ background: #2d2d2d; }}
  pre {{ background: #1e1e1e; }}
  blockquote {{ border-left-color: #555; color: #999; }}
  th, td {{ border-color: #444; }}
  th {{ background: #252525; }}
  hr {{ border-top-color: #333; }}
  a {{ color: #60a5fa; }}
  .file-header {{ border-bottom-color: #333; color: #888; }}
}}
</style>
<script>{hljs}</script>
<script>{mermaid}</script>
</head>
<body>
{body}
<script>
(function() {{
  // Convert mermaid code blocks to <div class="mermaid"> BEFORE mermaid runs.
  var blocks = document.querySelectorAll('pre > code.language-mermaid');
  for (var i = 0; i < blocks.length; i++) {{
    var code = blocks[i];
    var pre = code.parentNode;
    var div = document.createElement('div');
    div.className = 'mermaid';
    div.textContent = code.textContent;
    pre.parentNode.replaceChild(div, pre);
  }}
  try {{ if (window.hljs) hljs.highlightAll(); }} catch (e) {{}}
  try {{
    if (window.mermaid) {{
      var isDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
      mermaid.initialize({{ startOnLoad: true, theme: isDark ? 'dark' : 'default' }});
    }}
  }} catch (e) {{}}
}})();
</script>
</body>
</html>
"#,
            hljs = HIGHLIGHT_JS,
            mermaid = MERMAID_JS,
            body = body_html
        )
    }
}

impl Renderer for WebRenderer {
    fn widget(&self) -> gtk::Widget {
        self.webview.clone().upcast()
    }

    fn load(&self, path: &Path) {
        *self.current_path.borrow_mut() = Some(path.to_path_buf());
        let ext = Self::ext_lower(path);
        match ext.as_str() {
            "docx" => self.load_docx_file(path),
            "docmod" | "doct" => self.load_docmod_file(path),
            "html" | "htm" => self.load_html_file(path),
            "md" | "markdown" => self.load_markdown_file(path),
            _ => self.load_code_file(path),
        }
    }

    fn set_zoom(&self, level: f64) {
        self.webview.set_zoom_level(level);
    }
}
