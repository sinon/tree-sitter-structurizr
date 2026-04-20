use crate::FormatOptions;

// =============================================================================
// Conservative line-oriented formatter
// =============================================================================
//
// Structurizr DSL is block-oriented, but much of the user-visible syntax is still
// "one logical statement per line". The first printer therefore stays
// deliberately conservative:
//
// 1. It normalizes indentation, braces, and blank lines across the whole file.
// 2. It wraps selected long single-line statements using explicit `\`
//    continuations.
// 3. It preserves comment text, text-block payloads, and existing continuation
//    spelling rather than trying to reflow prose or canonicalize every possible
//    token sequence.
//
// This is intentionally not a fully general CST pretty-printer yet, but it gives
// the formatter a real, reusable implementation surface that matches the current
// policy contract.

/// Formats one Structurizr source document according to the provided policy.
#[must_use]
pub fn format_source(source: &str, options: &FormatOptions) -> String {
    let chunks = parse_chunks(source);
    let mut rendered = Vec::new();
    let mut block_stack = Vec::new();
    let mut previous: Option<ChunkMetadata> = None;

    for chunk in chunks {
        let indent_level = block_stack.len().saturating_sub(chunk.dedent_before());
        let parent_block = block_stack.last().copied();
        let metadata = chunk.metadata();

        let blank_lines = previous.map_or(0, |previous| {
            blank_lines_between(previous, metadata, parent_block).max(chunk.blank_before().min(1))
        });
        for _ in 0..blank_lines {
            rendered.push(String::new());
        }

        rendered.extend(chunk.render(indent_level, options));

        for _ in 0..chunk.dedent_before() {
            block_stack.pop();
        }
        for opened in chunk.opened_blocks() {
            block_stack.push(opened);
        }

        previous = Some(metadata);
    }

    if rendered.is_empty() {
        String::new()
    } else {
        rendered.join("\n") + "\n"
    }
}

#[derive(Debug, Clone)]
struct Chunk {
    blank_before: usize,
    kind: ChunkKind,
}

impl Chunk {
    const fn blank_before(&self) -> usize {
        self.blank_before
    }

    const fn dedent_before(&self) -> usize {
        match &self.kind {
            ChunkKind::ClosingBrace => 1,
            ChunkKind::CommentGroup { .. }
            | ChunkKind::TextBlock { .. }
            | ChunkKind::Statement { .. } => 0,
        }
    }

    fn opened_blocks(&self) -> Vec<BlockKind> {
        match &self.kind {
            ChunkKind::Statement {
                block_kind,
                opens_block,
                ..
            } if *opens_block => vec![*block_kind],
            ChunkKind::CommentGroup { .. }
            | ChunkKind::TextBlock { .. }
            | ChunkKind::ClosingBrace
            | ChunkKind::Statement { .. } => Vec::new(),
        }
    }

    fn metadata(&self) -> ChunkMetadata {
        match &self.kind {
            ChunkKind::ClosingBrace => ChunkMetadata {
                category: ChunkCategory::ClosingBrace,
                head: None,
            },
            ChunkKind::CommentGroup { .. } => ChunkMetadata {
                category: ChunkCategory::Comment,
                head: None,
            },
            ChunkKind::TextBlock { .. } => ChunkMetadata {
                category: ChunkCategory::Statement,
                head: None,
            },
            ChunkKind::Statement { head, .. } => ChunkMetadata {
                category: ChunkCategory::Statement,
                head: Some(head_kind_for(head)),
            },
        }
    }

    fn render(&self, indent_level: usize, options: &FormatOptions) -> Vec<String> {
        match &self.kind {
            ChunkKind::ClosingBrace => vec![format!("{}{}", indent(indent_level, options), "}")],
            ChunkKind::CommentGroup { lines } => render_comment_group(lines, indent_level, options),
            ChunkKind::TextBlock {
                header,
                body_lines,
                closing_line,
                ..
            } => {
                let mut rendered = Vec::with_capacity(body_lines.len() + 2);
                rendered.push(format!(
                    "{}{}",
                    indent(indent_level, options),
                    header.trim_start()
                ));
                rendered.extend(body_lines.iter().map(std::string::ToString::to_string));
                rendered.push(format!(
                    "{}{}",
                    indent(indent_level, options),
                    closing_line.trim_start()
                ));
                rendered
            }
            ChunkKind::Statement {
                lines,
                trailing_comment,
                continued,
                ..
            } => {
                if *continued {
                    let first_indent = indent(indent_level, options);
                    let continuation_indent = indent(indent_level + 1, options);
                    lines
                        .iter()
                        .enumerate()
                        .map(|(index, line)| {
                            let prefix = if index == 0 {
                                first_indent.as_str()
                            } else {
                                continuation_indent.as_str()
                            };
                            format!("{prefix}{}", line.trim_start())
                        })
                        .collect()
                } else {
                    format_single_statement(
                        &lines[0],
                        trailing_comment.as_deref(),
                        indent_level,
                        options,
                    )
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
enum ChunkKind {
    ClosingBrace,
    CommentGroup {
        lines: Vec<String>,
    },
    TextBlock {
        header: String,
        body_lines: Vec<String>,
        closing_line: String,
    },
    Statement {
        lines: Vec<String>,
        trailing_comment: Option<String>,
        continued: bool,
        opens_block: bool,
        head: String,
        block_kind: BlockKind,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ChunkMetadata {
    category: ChunkCategory,
    head: Option<HeadKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChunkCategory {
    ClosingBrace,
    Comment,
    Statement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeadKind {
    Workspace,
    Model,
    Views,
    Configuration,
    Properties,
    Name,
    Description,
    Styles,
    Theme,
    Themes,
    Branding,
    Terminology,
    ViewDefinition,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BlockKind {
    Workspace,
    Views,
    Other,
}

fn parse_chunks(source: &str) -> Vec<Chunk> {
    let lines = source
        .split('\n')
        .map(|line| line.trim_end_matches('\r').to_string())
        .collect::<Vec<_>>();
    let mut chunks = Vec::new();
    let mut index = 0;
    let mut blank_before = 0usize;

    while index < lines.len() {
        let line = &lines[index];
        if line.trim().is_empty() {
            blank_before += 1;
            index += 1;
            continue;
        }

        let trimmed = line.trim_start();
        if trimmed.starts_with("/*") {
            chunks.push(Chunk {
                blank_before,
                kind: ChunkKind::CommentGroup {
                    lines: consume_block_comment(&lines, &mut index),
                },
            });
            blank_before = 0;
            continue;
        }

        if is_line_comment(trimmed) {
            chunks.push(Chunk {
                blank_before,
                kind: ChunkKind::CommentGroup {
                    lines: consume_line_comments(&lines, &mut index),
                },
            });
            blank_before = 0;
            continue;
        }

        if line.trim() == "}" {
            chunks.push(Chunk {
                blank_before,
                kind: ChunkKind::ClosingBrace,
            });
            blank_before = 0;
            index += 1;
            continue;
        }

        let (code, trailing_comment) = split_trailing_comment(line);
        let quote_count = count_text_block_delimiters(&code);
        if quote_count % 2 == 1 {
            let (body_lines, closing_line) = consume_text_block(&lines, &mut index);
            chunks.push(Chunk {
                blank_before,
                kind: ChunkKind::TextBlock {
                    header: code,
                    body_lines,
                    closing_line,
                },
            });
            blank_before = 0;
            continue;
        }

        let statement_lines = consume_statement_lines(&lines, &mut index, code);

        let canonical = statement_lines[0].trim_start().to_string();
        let tokens = tokenize(&canonical);
        let head = tokens.first().map_or_else(String::new, Clone::clone);
        let opens_block = tokens.last().is_some_and(|token| token == "{");
        let continued = statement_lines.len() > 1;
        chunks.push(Chunk {
            blank_before,
            kind: ChunkKind::Statement {
                lines: statement_lines,
                trailing_comment,
                continued,
                opens_block,
                head: head.clone(),
                block_kind: block_kind_for_head(&head),
            },
        });
        blank_before = 0;
    }

    chunks
}

fn consume_block_comment(lines: &[String], index: &mut usize) -> Vec<String> {
    let first_line = &lines[*index];
    let mut comment_lines = vec![first_line.clone()];
    *index += 1;
    if first_line.trim_start().contains("*/") {
        return comment_lines;
    }

    while *index < lines.len() {
        let comment_line = &lines[*index];
        comment_lines.push(comment_line.clone());
        *index += 1;
        if comment_line.trim_start().contains("*/") {
            break;
        }
    }

    comment_lines
}

fn consume_line_comments(lines: &[String], index: &mut usize) -> Vec<String> {
    let mut comment_lines = vec![lines[*index].clone()];
    *index += 1;
    while *index < lines.len() {
        let next = &lines[*index];
        if next.trim().is_empty() || !is_line_comment(next.trim_start()) {
            break;
        }
        comment_lines.push(next.clone());
        *index += 1;
    }

    comment_lines
}

fn consume_text_block(lines: &[String], index: &mut usize) -> (Vec<String>, String) {
    let mut body_lines = Vec::new();
    let mut closing_line = String::new();
    *index += 1;
    while *index < lines.len() {
        let candidate = &lines[*index];
        if count_text_block_delimiters(candidate) % 2 == 1 {
            closing_line.clone_from(candidate);
            *index += 1;
            break;
        }
        body_lines.push(candidate.clone());
        *index += 1;
    }

    (body_lines, closing_line)
}

fn consume_statement_lines(lines: &[String], index: &mut usize, first_line: String) -> Vec<String> {
    let mut statement_lines = vec![first_line];
    *index += 1;
    let mut continued = ends_with_continuation(&statement_lines[0]);
    while continued && *index < lines.len() {
        let next = &lines[*index];
        statement_lines.push(next.clone());
        continued = ends_with_continuation(next);
        *index += 1;
    }

    statement_lines
}

fn render_comment_group(
    lines: &[String],
    indent_level: usize,
    options: &FormatOptions,
) -> Vec<String> {
    if lines
        .first()
        .is_some_and(|line| line.trim_start().starts_with("/*"))
    {
        return render_block_comment(lines, indent_level, options);
    }

    lines
        .iter()
        .map(|line| format!("{}{}", indent(indent_level, options), line.trim_start()))
        .collect()
}

fn render_block_comment(
    lines: &[String],
    indent_level: usize,
    options: &FormatOptions,
) -> Vec<String> {
    let base_indent = lines
        .first()
        .map_or(0, |line| leading_whitespace_width(line));
    lines
        .iter()
        .map(|line| {
            format!(
                "{}{}",
                indent(indent_level, options),
                strip_leading_whitespace_width(line, base_indent)
            )
        })
        .collect()
}

fn format_single_statement(
    line: &str,
    trailing_comment: Option<&str>,
    indent_level: usize,
    options: &FormatOptions,
) -> Vec<String> {
    let prefix = indent(indent_level, options);
    let tokens = tokenize(line.trim_start());

    if tokens.is_empty() {
        return vec![prefix];
    }

    let mut single_line = tokens.join(" ");
    if let Some(comment) = trailing_comment {
        single_line.push(' ');
        single_line.push_str(comment);
    }

    let target = options.line_width().target();
    if trailing_comment.is_some() || prefix.len() + single_line.len() <= target {
        return vec![format!("{prefix}{single_line}")];
    }

    let lines = wrap_tokens(&tokens, indent_level, options);
    if lines.is_empty() {
        vec![format!("{prefix}{single_line}")]
    } else {
        lines
    }
}

fn wrap_tokens(tokens: &[String], indent_level: usize, options: &FormatOptions) -> Vec<String> {
    if tokens.len() < 2 {
        return Vec::new();
    }

    let target = options.line_width().target();
    let prefix_end = wrap_prefix_end(tokens);
    let prefix_tokens = &tokens[..prefix_end];
    let suffix_tokens = &tokens[prefix_end..];
    if suffix_tokens.is_empty() {
        return Vec::new();
    }

    let continuation_prefix = indent(indent_level + 1, options);
    if suffix_tokens.len() == 1 && continuation_prefix.len() + suffix_tokens[0].len() > target {
        return Vec::new();
    }

    let mut lines = Vec::new();
    let mut current = format!(
        "{}{}",
        indent(indent_level, options),
        prefix_tokens.join(" ")
    );
    for token in suffix_tokens {
        let candidate = if current.ends_with('\\') || current.ends_with(' ') {
            format!("{current}{token}")
        } else {
            format!("{current} {token}")
        };
        if candidate.len() <= target || current == continuation_prefix {
            current = candidate;
            continue;
        }

        current.push_str(" \\");
        lines.push(current);
        current = format!("{continuation_prefix}{token}");
    }

    lines.push(current);
    lines
}

fn wrap_prefix_end(tokens: &[String]) -> usize {
    if let Some(arrow_index) = tokens.iter().position(|token| token == "->") {
        return (arrow_index + 2).min(tokens.len());
    }
    if let Some(assignment_index) = tokens.iter().position(|token| token == "=") {
        return (assignment_index + 2).min(tokens.len());
    }

    let head = tokens[0].to_ascii_lowercase();
    if matches!(head.as_str(), "include" | "themes" | "!include") {
        return 1;
    }

    if matches!(
        head.as_str(),
        "workspace"
            | "systemlandscape"
            | "systemcontext"
            | "container"
            | "component"
            | "dynamic"
            | "deployment"
            | "filtered"
            | "custom"
            | "image"
    ) {
        return tokens.len().min(2);
    }

    1
}

fn block_kind_for_head(head: &str) -> BlockKind {
    match head.to_ascii_lowercase().as_str() {
        "workspace" => BlockKind::Workspace,
        "views" => BlockKind::Views,
        _ => BlockKind::Other,
    }
}

fn blank_lines_between(
    previous: ChunkMetadata,
    next: ChunkMetadata,
    parent_block: Option<BlockKind>,
) -> usize {
    if matches!(previous.category, ChunkCategory::Comment)
        || matches!(
            next.category,
            ChunkCategory::ClosingBrace | ChunkCategory::Comment
        )
    {
        return 0;
    }

    match parent_block {
        None => usize::from(is_top_level_section(previous) && is_top_level_section(next)),
        Some(BlockKind::Workspace) => {
            if matches!(previous.category, ChunkCategory::ClosingBrace)
                && is_workspace_section(next)
            {
                return 1;
            }
            usize::from(is_workspace_section(previous) && is_workspace_section(next))
        }
        Some(BlockKind::Views) => {
            if matches!(previous.category, ChunkCategory::ClosingBrace) && is_views_major_item(next)
            {
                return 1;
            }
            usize::from(is_views_major_item(previous) && is_views_major_item(next))
        }
        Some(BlockKind::Other) => 0,
    }
}

const fn is_top_level_section(metadata: ChunkMetadata) -> bool {
    matches!(
        metadata.head,
        Some(
            HeadKind::Workspace
                | HeadKind::Model
                | HeadKind::Views
                | HeadKind::Configuration
                | HeadKind::Styles
        )
    )
}

const fn is_workspace_section(metadata: ChunkMetadata) -> bool {
    matches!(
        metadata.head,
        Some(
            HeadKind::Model
                | HeadKind::Views
                | HeadKind::Configuration
                | HeadKind::Properties
                | HeadKind::Name
                | HeadKind::Description
        )
    )
}

const fn is_views_major_item(metadata: ChunkMetadata) -> bool {
    matches!(
        metadata.head,
        Some(
            HeadKind::ViewDefinition
                | HeadKind::Styles
                | HeadKind::Themes
                | HeadKind::Theme
                | HeadKind::Branding
                | HeadKind::Terminology
                | HeadKind::Properties
        )
    )
}

fn tokenize(line: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let bytes = line.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }

        if line[index..].starts_with("->") {
            tokens.push("->".to_string());
            index += 2;
            continue;
        }

        if matches!(byte as char, '{' | '}' | '=') {
            tokens.push((byte as char).to_string());
            index += 1;
            continue;
        }

        if byte as char == '"' {
            let start = index;
            index += 1;
            while index < bytes.len() {
                if bytes[index] as char == '\\' {
                    index += 2;
                    continue;
                }
                if bytes[index] as char == '"' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            tokens.push(line[start..index].to_string());
            continue;
        }

        let start = index;
        while index < bytes.len() {
            let char = bytes[index] as char;
            if char.is_whitespace()
                || matches!(char, '{' | '}' | '=' | '"')
                || line[index..].starts_with("->")
            {
                break;
            }
            index += 1;
        }
        tokens.push(line[start..index].to_string());
    }

    tokens
}

fn split_trailing_comment(line: &str) -> (String, Option<String>) {
    let bytes = line.as_bytes();
    let mut index = 0usize;
    let mut in_string = false;
    while index < bytes.len() {
        if bytes[index] as char == '"' {
            in_string = !in_string;
            index += 1;
            continue;
        }
        if !in_string {
            if line[index..].starts_with("//") && can_start_inline_comment(bytes, index) {
                let code = line[..index].trim_end().to_string();
                let comment = line[index..].trim_start().to_string();
                return (code, Some(comment));
            }
            if bytes[index] as char == '#'
                && starts_hash_comment(&line[index..])
                && can_start_inline_comment(bytes, index)
            {
                let code = line[..index].trim_end().to_string();
                let comment = line[index..].trim_start().to_string();
                return (code, Some(comment));
            }
        }
        if bytes[index] as char == '\\' && in_string {
            index += 2;
            continue;
        }
        index += 1;
    }

    (line.trim_end().to_string(), None)
}

fn can_start_inline_comment(bytes: &[u8], index: usize) -> bool {
    index == 0 || bytes[index - 1].is_ascii_whitespace()
}

fn starts_hash_comment(text: &str) -> bool {
    text.as_bytes().first() == Some(&b'#')
        && text[1..].chars().next().is_some_and(char::is_whitespace)
}

fn count_text_block_delimiters(line: &str) -> usize {
    line.match_indices("\"\"\"").count()
}

fn ends_with_continuation(line: &str) -> bool {
    line.trim_end().ends_with('\\')
}

fn is_line_comment(trimmed: &str) -> bool {
    trimmed.starts_with("//") || starts_hash_comment(trimmed)
}

fn leading_whitespace_width(line: &str) -> usize {
    line.chars().take_while(|char| char.is_whitespace()).count()
}

fn strip_leading_whitespace_width(line: &str, width: usize) -> &str {
    for (stripped, (byte_index, char)) in line.char_indices().enumerate() {
        if stripped == width || !char.is_whitespace() {
            return &line[byte_index..];
        }
    }

    ""
}

fn head_kind_for(head: &str) -> HeadKind {
    match head.to_ascii_lowercase().as_str() {
        "workspace" => HeadKind::Workspace,
        "model" => HeadKind::Model,
        "views" => HeadKind::Views,
        "configuration" => HeadKind::Configuration,
        "properties" => HeadKind::Properties,
        "name" => HeadKind::Name,
        "description" => HeadKind::Description,
        "styles" => HeadKind::Styles,
        "theme" => HeadKind::Theme,
        "themes" => HeadKind::Themes,
        "branding" => HeadKind::Branding,
        "terminology" => HeadKind::Terminology,
        "systemlandscape" | "systemcontext" | "container" | "component" | "dynamic"
        | "deployment" | "filtered" | "custom" | "image" => HeadKind::ViewDefinition,
        _ => HeadKind::Other,
    }
}

fn indent(level: usize, options: &FormatOptions) -> String {
    " ".repeat(level * usize::from(options.indentation().width()))
}

#[cfg(test)]
mod tests {
    use indoc::indoc;

    use super::format_source;
    use crate::{
        CommentFormatPolicy, FormatOptions, Indentation, LineWidthPolicy, OverflowPolicy,
        SpacingPolicy, SyntaxErrorPolicy,
    };

    #[test]
    fn simple_blocks_are_reindented_and_normalized() {
        let formatted = format_source(
            indoc! {r#"
                workspace {
                model {
                user = person "User"
                }
                views{
                systemContext user "Context"{
                include *
                }
                }
                }
            "#},
            &FormatOptions::default(),
        );

        assert_eq!(
            formatted,
            indoc! {r#"
                workspace {
                    model {
                        user = person "User"
                    }

                    views {
                        systemContext user "Context" {
                            include *
                        }
                    }
                }
            "#}
        );
    }

    #[test]
    fn long_declarations_wrap_with_explicit_continuations() {
        let formatted = format_source(
            indoc! {r#"
                workspace {
                    model {
                        singlePageApplication = container "Single-Page Application" "Provides all of the Internet banking functionality to customers via their web browser." "JavaScript and Angular" "Web Browser"
                    }
                }
            "#},
            &FormatOptions::default(),
        );

        assert!(
            formatted.contains(
                "singlePageApplication = container \"Single-Page Application\" \\\n            \"Provides all of the Internet banking functionality to customers via their web browser.\" \\"
            ),
            "expected long declaration wrapping with explicit continuations:\n{formatted}"
        );
    }

    #[test]
    fn long_relationships_wrap_after_the_endpoint_pair() {
        let formatted = format_source(
            indoc! {r#"
                workspace {
                    model {
                        singlePageApplication -> signinController "Submits credentials to the Internet Banking System API and expects a session token" "JSON/HTTPS" "Current"
                    }
                }
            "#},
            &FormatOptions::default(),
        );

        assert!(
            formatted.contains(
                "singlePageApplication -> signinController \\\n            \"Submits credentials to the Internet Banking System API and expects a session token\" \"JSON/HTTPS\" \\"
            ),
            "expected long relationship wrapping:\n{formatted}"
        );
    }

    #[test]
    fn comments_are_not_reflowed() {
        let formatted = format_source(
            indoc! {r#"
                workspace {
                    views {
                        styles {
                    # TODO: we should tab-complete the known values for properties where the potential values are a known fixed set
                            element "Software System" {
                                background #999999
                            }
                        }
                    }
                }
            "#},
            &FormatOptions::default(),
        );

        assert!(formatted.contains("# TODO: we should tab-complete the known values for properties where the potential values are a known fixed set"));
    }

    #[test]
    fn existing_line_continuations_keep_their_text() {
        let formatted = format_source(
            indoc! {r#"
                workspace {
                    model {
                softwareSystem = \
                    softwareSystem \
                    "Software \
                    System"
                    }
                }
            "#},
            &FormatOptions::default(),
        );

        assert!(formatted.contains("softwareSystem = \\"));
        assert!(formatted.contains("\"Software \\"));
    }

    #[test]
    fn bare_urls_are_not_split_as_comments() {
        let formatted = format_source(
            "views {\n    themes https://example.com/theme-one.json\n}\n",
            &FormatOptions::default(),
        );

        assert!(formatted.contains("https://example.com/theme-one.json"));
        assert!(
            !formatted.contains("https: //example.com/theme-one.json"),
            "formatter should preserve bare URLs:\n{formatted}"
        );
    }

    #[test]
    fn hash_tab_comments_are_preserved() {
        let formatted = format_source(
            "workspace {\nmodel {\n#\tTabbed comment\nsystem = softwareSystem \"System\"\n}\n}\n",
            &FormatOptions::default(),
        );

        assert!(formatted.contains("#\tTabbed comment"));
    }

    #[test]
    fn multi_line_block_comments_preserve_relative_indentation() {
        let formatted = format_source(
            indoc! {r"
                workspace {
                model {
                    /*
                      first
                        second
                    */
                }
                }
            "},
            &FormatOptions::default(),
        );

        assert_eq!(
            formatted,
            indoc! {r"
                workspace {
                    model {
                        /*
                          first
                            second
                        */
                    }
                }
            "}
        );
    }

    #[test]
    fn text_block_closing_delimiters_are_reindented() {
        let formatted = format_source(
            "workspace{\nviews{\n!const SOURCE \"\"\"\nclass MyClass\n\"\"\"\n}\n}\n",
            &FormatOptions::default(),
        );

        assert_eq!(
            formatted,
            "workspace {\n    views {\n        !const SOURCE \"\"\"\nclass MyClass\n        \"\"\"\n    }\n}\n"
        );
    }

    #[test]
    fn themes_lists_wrap_without_splitting_urls() {
        let formatted = format_source(
            "views {\n    themes https://example.com/theme-one.json https://example.com/theme-two.json https://example.com/theme-three.json\n}\n",
            &test_options(60),
        );

        assert!(formatted.contains("themes https://example.com/theme-one.json \\"));
        assert!(formatted.contains("https://example.com/theme-two.json \\"));
        assert!(formatted.contains("https://example.com/theme-three.json"));
        assert!(
            !formatted.contains("https: //"),
            "wrapped URL list should not turn URLs into comments:\n{formatted}"
        );
    }

    #[test]
    fn long_view_headers_wrap_while_keeping_the_opening_brace_attached() {
        let formatted = format_source(
            "views {\n    systemContext payments \"payments-context-key\" \"Overview of the payments context\" {\n        include *\n    }\n}\n",
            &test_options(50),
        );

        assert!(formatted.contains("systemContext payments \"payments-context-key\" \\"));
        assert!(
            formatted.contains("\"Overview of the payments context\" {"),
            "wrapped view headers should keep the opening brace on the final header line:\n{formatted}"
        );
    }

    fn test_options(target: usize) -> FormatOptions {
        FormatOptions::new(
            Indentation::spaces(4),
            LineWidthPolicy::new(
                target,
                CommentFormatPolicy::Preserve,
                OverflowPolicy::BestEffort,
            ),
            SpacingPolicy::default(),
            SyntaxErrorPolicy::Refuse,
        )
    }
}
