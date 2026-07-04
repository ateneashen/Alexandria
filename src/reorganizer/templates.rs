use std::path::{Component, Path, PathBuf};

/// Context available to the reorganization template engine.
#[derive(Debug, Clone, Default)]
pub struct TemplateContext {
    pub file_type: String,
    pub extension: String,
    pub name: String,
    pub ext: String,
    pub group_name: String,
    pub group_kind: String,
    pub year: String,
    pub month: String,
    pub day: String,
    pub tag: String,
}

/// Render a template like "{file_type}/{name}.{ext}" into a relative path.
/// Tokens are replaced by sanitized values. Directory separators use '/' and
/// are converted to the host OS separator when the path is materialized.
pub fn render_template(template: &str, ctx: &TemplateContext) -> String {
    let mut result = template.to_string();

    let replacements = [
        ("{file_type}", ctx.file_type.as_str()),
        ("{extension}", ctx.extension.as_str()),
        ("{name}", ctx.name.as_str()),
        ("{ext}", ctx.ext.as_str()),
        ("{group_name}", ctx.group_name.as_str()),
        ("{group_kind}", ctx.group_kind.as_str()),
        ("{year}", ctx.year.as_str()),
        ("{month}", ctx.month.as_str()),
        ("{day}", ctx.day.as_str()),
        ("{tag}", ctx.tag.as_str()),
    ];

    for (token, value) in replacements {
        result = result.replace(token, value);
    }

    sanitize_path(&result)
}

/// Sanitize a relative path. Each path component gets cleaned individually so
/// that directory separators are preserved.
pub fn sanitize_path(input: &str) -> String {
    let mut output = Vec::new();
    for part in input.split('/') {
        let cleaned = sanitize_component(part);
        if !cleaned.is_empty() && cleaned != "." {
            output.push(cleaned);
        }
    }
    output.join("/")
}

/// Sanitize a single file or folder name.
/// - Replaces characters illegal on Windows (`< > : " / \ | ? *`) and control
///   characters (`0x00-0x1f`) by `_`.
/// - Removes trailing dots.
/// - Caps length at 200 characters.
pub fn sanitize_component(input: &str) -> String {
    let mut sanitized: String = input
        .chars()
        .map(|c| {
            if c as u32 <= 0x1f
                || c == '<'
                || c == '>'
                || c == ':'
                || c == '"'
                || c == '/'
                || c == '\\'
                || c == '|'
                || c == '?'
                || c == '*'
            {
                '_'
            } else {
                c
            }
        })
        .collect();

    // Drop trailing dots which are problematic on Windows.
    while sanitized.ends_with('.') {
        sanitized.pop();
    }

    // Limit component length while keeping valid UTF-8 boundaries.
    if sanitized.chars().count() > 200 {
        let truncated: String = sanitized.chars().take(200).collect();
        sanitized = truncated;
    }

    sanitized
}

/// Convert a '/'-separated relative path into a host OS path.
pub fn to_os_relative_path(relative: &str) -> PathBuf {
    relative.split('/').collect()
}

/// Join a target root with a relative rendered path and return an absolute path.
pub fn build_target_path(target_root: &Path, relative: &str) -> PathBuf {
    let mut path = target_root.to_path_buf();
    for component in relative.split('/') {
        if component == ".." || component == "." || component.is_empty() {
            continue;
        }
        path.push(component);
    }
    path
}

/// Ensure the path is absolute and does not contain parent directory traversal.
pub fn is_safe_relative_path(path: &str) -> bool {
    let p = Path::new(path);
    if p.is_absolute() {
        return false;
    }
    for component in p.components() {
        match component {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
            _ => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_and_sanitize() {
        let ctx = TemplateContext {
            file_type: "video".into(),
            extension: "MP4".into(),
            name: "My<File>:Name".into(),
            ext: "mp4".into(),
            group_name: "Show Name".into(),
            group_kind: "series".into(),
            year: "2024".into(),
            month: "01".into(),
            day: "31".into(),
            tag: "sci-fi".into(),
        };

        assert_eq!(
            render_template("{file_type}/{name}.{ext}", &ctx),
            "video/My_File__Name.mp4"
        );
        assert_eq!(
            render_template(
                "{group_kind}/{group_name}/{year}-{month}/{name}.{extension}",
                &ctx
            ),
            "series/Show Name/2024-01/My_File__Name.MP4"
        );
    }

    #[test]
    fn test_sanitize_trailing_dot() {
        assert_eq!(sanitize_component("folder."), "folder");
        assert_eq!(sanitize_component("file."), "file");
    }
}
