//! Markdown rendering component for AI chat messages
//!
//! Uses pulldown-cmark to parse Markdown and renders it as HTML elements.

use leptos::prelude::*;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

/// Render markdown content as HTML
#[component]
pub fn Markdown(
    /// The markdown content to render
    content: String,
) -> impl IntoView {
    let html = parse_markdown(&content);

    view! {
        <div
            class="markdown-content prose prose-sm dark:prose-invert max-w-none"
            inner_html=html
        />
    }
}

/// Parse markdown string to HTML
pub fn parse_markdown(content: &str) -> String {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);

    let parser = Parser::new_ext(content, options);
    let mut html_output = String::new();

    // Track state for proper nesting
    let mut in_code_block = false;
    let mut code_block_content = String::new();
    let mut code_language = String::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => html_output.push_str("<p class=\"mb-2 last:mb-0\">"),
                Tag::Heading { level, .. } => {
                    let (class, tag) = match level {
                        HeadingLevel::H1 => ("text-xl font-bold mb-2 mt-3", "h1"),
                        HeadingLevel::H2 => ("text-lg font-bold mb-2 mt-3", "h2"),
                        HeadingLevel::H3 => ("text-base font-semibold mb-1 mt-2", "h3"),
                        HeadingLevel::H4 => ("text-sm font-semibold mb-1 mt-2", "h4"),
                        HeadingLevel::H5 => ("text-sm font-medium mb-1 mt-1", "h5"),
                        HeadingLevel::H6 => ("text-xs font-medium mb-1 mt-1", "h6"),
                    };
                    html_output.push_str(&format!("<{} class=\"{}\">", tag, class));
                }
                Tag::BlockQuote(_) => {
                    html_output.push_str("<blockquote class=\"border-l-4 border-theme-accent pl-4 my-2 text-theme-secondary italic\">");
                }
                Tag::CodeBlock(kind) => {
                    in_code_block = true;
                    code_block_content.clear();
                    code_language = match kind {
                        pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                        pulldown_cmark::CodeBlockKind::Indented => String::new(),
                    };
                }
                Tag::List(Some(_)) => {
                    html_output.push_str("<ol class=\"list-decimal list-inside mb-2 space-y-1\">");
                }
                Tag::List(None) => {
                    html_output.push_str("<ul class=\"list-disc list-inside mb-2 space-y-1\">");
                }
                Tag::Item => html_output.push_str("<li class=\"text-theme-primary\">"),
                Tag::Emphasis => html_output.push_str("<em class=\"italic\">"),
                Tag::Strong => html_output.push_str("<strong class=\"font-semibold\">"),
                Tag::Strikethrough => html_output.push_str("<del class=\"line-through\">"),
                Tag::Link {
                    dest_url, title, ..
                } => {
                    let title_attr = if title.is_empty() {
                        String::new()
                    } else {
                        format!(" title=\"{}\"", escape_html(&title))
                    };
                    html_output.push_str(&format!(
                            "<a href=\"{}\" class=\"text-theme-accent hover:underline\" target=\"_blank\" rel=\"noopener noreferrer\"{}>",
                            escape_html(&dest_url),
                            title_attr
                        ));
                }
                Tag::Image {
                    dest_url, title, ..
                } => {
                    let title_attr = if title.is_empty() {
                        String::new()
                    } else {
                        format!(" title=\"{}\"", escape_html(&title))
                    };
                    html_output.push_str(&format!(
                        "<img src=\"{}\" class=\"max-w-full rounded my-2\"{} alt=\"",
                        escape_html(&dest_url),
                        title_attr
                    ));
                }
                Tag::Table(_) => {
                    html_output.push_str("<div class=\"overflow-x-auto my-2\"><table class=\"min-w-full border border-theme-primary rounded\">");
                }
                Tag::TableHead => {
                    html_output.push_str("<thead class=\"bg-theme-secondary\">");
                }
                Tag::TableRow => html_output.push_str("<tr>"),
                Tag::TableCell => {
                    html_output.push_str(
                        "<td class=\"px-3 py-1.5 border-b border-theme-primary text-sm\">",
                    );
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Paragraph => html_output.push_str("</p>"),
                TagEnd::Heading(level) => {
                    let tag = match level {
                        HeadingLevel::H1 => "h1",
                        HeadingLevel::H2 => "h2",
                        HeadingLevel::H3 => "h3",
                        HeadingLevel::H4 => "h4",
                        HeadingLevel::H5 => "h5",
                        HeadingLevel::H6 => "h6",
                    };
                    html_output.push_str(&format!("</{}>", tag));
                }
                TagEnd::BlockQuote(_) => html_output.push_str("</blockquote>"),
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    let lang_class = if !code_language.is_empty() {
                        format!(" language-{}", code_language)
                    } else {
                        String::new()
                    };
                    html_output.push_str(&format!(
                            "<pre class=\"bg-theme-tertiary rounded-lg p-3 my-2 overflow-x-auto\"><code class=\"text-sm font-mono text-theme-primary{}\">{}</code></pre>",
                            lang_class,
                            escape_html(&code_block_content)
                        ));
                }
                TagEnd::List(true) => html_output.push_str("</ol>"),
                TagEnd::List(false) => html_output.push_str("</ul>"),
                TagEnd::Item => html_output.push_str("</li>"),
                TagEnd::Emphasis => html_output.push_str("</em>"),
                TagEnd::Strong => html_output.push_str("</strong>"),
                TagEnd::Strikethrough => html_output.push_str("</del>"),
                TagEnd::Link => html_output.push_str("</a>"),
                TagEnd::Image => html_output.push_str("\" />"),
                TagEnd::Table => html_output.push_str("</table></div>"),
                TagEnd::TableHead => html_output.push_str("</thead><tbody>"),
                TagEnd::TableRow => html_output.push_str("</tr>"),
                TagEnd::TableCell => html_output.push_str("</td>"),
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    code_block_content.push_str(&text);
                } else {
                    html_output.push_str(&escape_html(&text));
                }
            }
            Event::Code(code) => {
                html_output.push_str(&format!(
                    "<code class=\"bg-theme-tertiary px-1.5 py-0.5 rounded text-sm font-mono text-theme-accent\">{}</code>",
                    escape_html(&code)
                ));
            }
            Event::Html(html) | Event::InlineHtml(html) => {
                // Pass through HTML (but be careful with this in production)
                html_output.push_str(&html);
            }
            Event::SoftBreak => html_output.push(' '),
            Event::HardBreak => html_output.push_str("<br />"),
            Event::Rule => {
                html_output.push_str("<hr class=\"my-4 border-theme-primary\" />");
            }
            Event::TaskListMarker(checked) => {
                let checkbox = if checked {
                    "<input type=\"checkbox\" checked disabled class=\"mr-2\" />"
                } else {
                    "<input type=\"checkbox\" disabled class=\"mr-2\" />"
                };
                html_output.push_str(checkbox);
            }
            Event::FootnoteReference(_) | Event::InlineMath(_) | Event::DisplayMath(_) => {
                // Not supported, skip
            }
        }
    }

    html_output
}

/// Escape HTML special characters
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_paragraph() {
        let html = parse_markdown("Hello, world!");
        assert!(html.contains("<p"));
        assert!(html.contains("Hello, world!"));
        assert!(html.contains("</p>"));
    }

    #[test]
    fn test_bold_text() {
        let html = parse_markdown("This is **bold** text");
        assert!(html.contains("<strong"));
        assert!(html.contains("bold"));
        assert!(html.contains("</strong>"));
    }

    #[test]
    fn test_italic_text() {
        let html = parse_markdown("This is *italic* text");
        assert!(html.contains("<em"));
        assert!(html.contains("italic"));
        assert!(html.contains("</em>"));
    }

    #[test]
    fn test_inline_code() {
        let html = parse_markdown("Use `code` here");
        assert!(html.contains("<code"));
        assert!(html.contains("code"));
        assert!(html.contains("</code>"));
    }

    #[test]
    fn test_code_block() {
        let html = parse_markdown("```sql\nSELECT * FROM users;\n```");
        assert!(html.contains("<pre"));
        assert!(html.contains("<code"));
        assert!(html.contains("SELECT"));
        assert!(html.contains("language-sql"));
    }

    #[test]
    fn test_unordered_list() {
        let html = parse_markdown("- Item 1\n- Item 2\n- Item 3");
        assert!(html.contains("<ul"));
        assert!(html.contains("<li"));
        assert!(html.contains("Item 1"));
        assert!(html.contains("</ul>"));
    }

    #[test]
    fn test_ordered_list() {
        let html = parse_markdown("1. First\n2. Second\n3. Third");
        assert!(html.contains("<ol"));
        assert!(html.contains("<li"));
        assert!(html.contains("First"));
        assert!(html.contains("</ol>"));
    }

    #[test]
    fn test_heading() {
        let html = parse_markdown("## Heading 2");
        assert!(html.contains("<h2"));
        assert!(html.contains("Heading 2"));
        assert!(html.contains("</h2>"));
    }

    #[test]
    fn test_link() {
        let html = parse_markdown("[Link](https://example.com)");
        assert!(html.contains("<a"));
        assert!(html.contains("href=\"https://example.com\""));
        assert!(html.contains("Link"));
        assert!(html.contains("</a>"));
    }

    #[test]
    fn test_escape_html() {
        let escaped = escape_html("<script>alert('xss')</script>");
        assert!(!escaped.contains('<'));
        assert!(!escaped.contains('>'));
        assert!(escaped.contains("&lt;"));
        assert!(escaped.contains("&gt;"));
    }

    #[test]
    fn test_complex_markdown() {
        let md = r#"
## Users Table

The **users** table has the following columns:

- `id` (Primary Key, Integer)
- `name` (Varchar)
- `email` (Varchar, Unique)

```sql
CREATE TABLE users (
    id INT PRIMARY KEY,
    name VARCHAR(255),
    email VARCHAR(255) UNIQUE
);
```
"#;
        let html = parse_markdown(md);
        assert!(html.contains("<h2"));
        assert!(html.contains("<strong"));
        assert!(html.contains("<ul"));
        assert!(html.contains("<code"));
        assert!(html.contains("<pre"));
    }
}
