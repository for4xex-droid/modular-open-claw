/*
 * Aiome - The Autonomous AI Operating System
 * Copyright (C) 2026 motivationstudio, LLC
 *
 * Licensed under the Business Source License 1.1.
 */

use aiome_core::error::AiomeError;
use ammonia::Builder as AmmoniaBuilder;
use minijinja::{context, Environment};
use std::collections::HashSet;

/// Builder for generating secure, themed HTML reports.
pub struct HtmlReportBuilder {
    env: Environment<'static>,
    tokens_css: String,
    sections: Vec<(String, String)>,
    summary: String,
}

impl HtmlReportBuilder {
    pub fn new(tokens_css: String) -> Result<Self, AiomeError> {
        let mut env = Environment::new();
        env.add_template(
            "base",
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>Aiome Report</title>
    <style>{{ tokens_css | safe }}</style>
    <style>
        body { font-family: var(--font-family-base, sans-serif); background: var(--bg-primary); color: var(--text-primary); margin: 0; padding: 2rem; }
        .report-summary { font-size: 1.1rem; color: var(--text-muted); margin-bottom: 2rem; border-bottom: 1px solid var(--white-10); padding-bottom: 1rem; }
        .section { margin-bottom: 3rem; background: var(--bg-secondary); border-radius: var(--radius-lg); padding: 1.5rem; border: 1px solid var(--white-05); }
        .section-title { font-family: var(--font-family-display, sans-serif); color: var(--accent-cyan); margin-top: 0; }
        table { width: 100%; border-collapse: collapse; margin: 1rem 0; }
        td, th { padding: 0.8rem; border-bottom: 1px solid var(--white-10); text-align: left; }
        th { color: var(--text-muted); font-weight: 500; }
        .aiome-feedback-btn { background: var(--accent-cyan-20); border: 1px solid var(--accent-cyan); color: var(--accent-cyan); padding: 0.5rem 1rem; border-radius: var(--radius-md); cursor: pointer; font-size: 0.85rem; font-weight: 600; transition: all 0.2s; margin-top: 1rem; }
        .aiome-feedback-btn:hover { background: var(--accent-cyan); color: var(--bg-primary); }
    </style>
</head>
<body>
    {% if summary %}
    <div class="report-summary">{{ summary }}</div>
    {% endif %}

    {% for section in sections %}
    <div class="section">
        <h2 class="section-title">{{ section.title }}</h2>
        <div class="section-content">{{ section.content | safe }}</div>
    </div>
    {% endfor %}

    <script>
        document.addEventListener('click', function(e) {
            // Handle JS Bridge feedback
            var target = e.target.closest('[data-aiome-feedback]');
            if (target) {
                var prompt = target.getAttribute('data-aiome-feedback');
                var autoSend = target.getAttribute('data-autosend') === 'true';
                if (prompt) {
                    window.parent.postMessage({ type: 'AIOME_PROMPT_FEEDBACK', payload: prompt, autoSend: autoSend }, '*');
                }
                return;
            }
            
            // Force external links to open in a new tab securely
            var link = e.target.closest('a');
            if (link && link.href) {
                link.setAttribute('target', '_blank');
                link.setAttribute('rel', 'noopener noreferrer');
            }
        });
    </script>
</body>
</html>"#,
        )
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to add base template: {}", e),
        })?;

        Ok(Self {
            env,
            tokens_css,
            sections: Vec::new(),
            summary: String::new(),
        })
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    pub fn add_section(
        mut self,
        title: impl Into<String>,
        content_html: impl Into<String>,
    ) -> Self {
        self.sections.push((title.into(), content_html.into()));
        self
    }

    /// Sanitize the HTML content using a relaxed policy for reports.
    pub fn sanitize(html: &str) -> String {
        let mut tags = HashSet::new();
        tags.extend([
            "div",
            "span",
            "p",
            "h1",
            "h2",
            "h3",
            "h4",
            "h5",
            "h6",
            "table",
            "thead",
            "tbody",
            "tr",
            "td",
            "th",
            "ul",
            "ol",
            "li",
            "a",
            "img",
            "code",
            "pre",
            "svg",
            "path",
            "circle",
            "rect",
            "line",
            "polyline",
            "polygon",
            "g",
            "style",
            "details",
            "summary",
            "br",
            "hr",
            "blockquote",
            "button",
        ]);

        let mut generic_attributes = HashSet::new();
        generic_attributes.extend([
            "class",
            "style",
            "id",
            "title",
            // SVG standard attributes
            "viewBox",
            "xmlns",
            "width",
            "height",
            "preserveAspectRatio",
            "fill",
            "stroke",
            "stroke-width",
            "stroke-linecap",
            "stroke-linejoin",
            "opacity",
            "transform",
            // SVG shape attributes
            "d",
            "cx",
            "cy",
            "r",
            "x",
            "y",
            "rx",
            "ry",
            "points",
            "stroke-dasharray",
            "stroke-dashoffset",
            // SVG text attributes
            "text-anchor",
            "alignment-baseline",
            "font-size",
            "font-family",
            "font-weight",
            // Aiome specific JS bridge attributes
            "data-aiome-feedback",
            "data-autosend",
        ]);

        let mut builder = AmmoniaBuilder::default();
        builder
            .tags(tags)
            .generic_attributes(generic_attributes)
            .clean_content_tags(HashSet::new())
            .link_rel(Some("noopener noreferrer"))
            .clean(html)
            .to_string()
    }

    pub fn build(self) -> Result<String, AiomeError> {
        let tmpl = self
            .env
            .get_template("base")
            .map_err(|e| AiomeError::Infrastructure {
                reason: format!("Failed to get report template: {}", e),
            })?;

        let mut sanitized_sections = Vec::with_capacity(self.sections.len());
        for (title, content) in self.sections {
            sanitized_sections.push(context! {
                title => title,
                content => Self::sanitize(&content),
            });
        }

        tmpl.render(context! {
            tokens_css => self.tokens_css,
            summary => self.summary,
            sections => sanitized_sections,
        })
        .map_err(|e| AiomeError::Infrastructure {
            reason: format!("Failed to render HTML report: {}", e),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitization_removes_scripts() {
        let input = "<div>Safe</div><script>alert('xss')</script>";
        let sanitized = HtmlReportBuilder::sanitize(input);
        assert!(!sanitized.contains("<script>"), "Should remove script tags");
        assert!(
            sanitized.contains("<div>Safe</div>"),
            "Should keep safe div tags"
        );
    }

    #[test]
    fn test_sanitization_allows_tables() {
        let input = "<table><tr><td>Data</td></tr></table>";
        let sanitized = HtmlReportBuilder::sanitize(input);
        assert!(
            sanitized.contains("<table>"),
            "Should allow table tags for reports"
        );
    }

    #[test]
    fn test_render_injects_css() {
        let builder = HtmlReportBuilder::new("/* tokens */".to_string()).unwrap();
        let html = builder.build().unwrap();
        assert!(
            html.contains("<style>/* tokens */</style>"),
            "Should inject CSS tokens"
        );
    }

    #[test]
    fn test_render_sections_and_sanitizes() {
        let builder = HtmlReportBuilder::new(String::new())
            .unwrap()
            .with_summary("Report Summary")
            .add_section(
                "Results",
                "<div class='test'>Content</div><script>bad</script>",
            );

        let html = builder.build().unwrap();
        assert!(html.contains("Report Summary"), "Should include summary");
        assert!(html.contains("Results"), "Should include section title");
        assert!(
            html.contains("<div class=\"test\">Content</div>"),
            "Should keep sanitized content"
        );
        assert!(
            !html.contains("<script>bad</script>"),
            "Should sanitize section content"
        );
    }

    #[test]
    fn test_sanitization_allows_svg_attributes() {
        let input = r#"<svg viewBox="0 0 100 100" width="50" height="50"><path d="M10 10 H 90 V 90 H 10 Z" fill="red" stroke="blue" /></svg>"#;
        let sanitized = HtmlReportBuilder::sanitize(input);
        assert!(
            sanitized.contains("viewBox"),
            "Should keep viewBox attribute: {}",
            sanitized
        );
        assert!(
            sanitized.contains("d=\"M10 10 H 90 V 90 H 10 Z\""),
            "Should keep d attribute: {}",
            sanitized
        );
        assert!(
            sanitized.contains("fill=\"red\""),
            "Should keep fill attribute: {}",
            sanitized
        );
    }
}
