use snippet::{AnnotationType, Snippet};
use std::fmt;

#[derive(Debug)]
pub struct DisplayList {
    pub body: Vec<DisplayLine>,
}

fn format_header(snippet: &Snippet) -> Vec<DisplayLine> {
    let mut header = vec![];

    let title_annotation = snippet
        .title_annotation_pos
        .and_then(|pos| snippet.annotations.get(pos));

    if let Some(annotation) = title_annotation {
        let annotation_type = match annotation.annotation_type {
            AnnotationType::Error => "error",
            AnnotationType::Warning => "warning",
        };
        let id = annotation.id.clone().unwrap_or("E0000".to_string());
        let label = annotation.label.clone().unwrap_or("".to_string());
        header.push(DisplayLine::RawLine(format!(
            "{}[{}]: {}",
            annotation_type, id, label
        )));
    }

    let main_annotation = snippet
        .main_annotation_pos
        .and_then(|pos| snippet.annotations.get(pos));

    if let Some(_annotation) = main_annotation {
        let path = snippet.slice.origin.clone().unwrap_or("".to_string());
        let row = 52;
        let col = 1;
        header.push(DisplayLine::RawLine(format!(
            "  --> {}:{}:{}",
            path, row, col
        )));
    }
    return header;
}

fn format_body(mut snippet: Snippet) -> Vec<DisplayLine> {
    let mut body = vec![];

    let mut current_line = snippet.slice.line_start;
    let mut current_index = 0;
    let mut line_index_ranges = vec![];

    for line in snippet.slice.source.lines() {
        body.push(DisplayLine::SourceLine {
            lineno: current_line,
            inline_marks: vec![],
            content: line.to_string(),
        });
        let line_length = line.chars().count() + 1;
        line_index_ranges.push((current_index, current_index + line_length));
        current_line += 1;
        current_index += line_length + 1;
    }

    let mut annotation_line_count = 0;
    for idx in 0..body.len() {
        let (line_start, line_end) = line_index_ranges[idx];
        snippet.annotations.drain_filter(|annotation| {
            let body_idx = idx + annotation_line_count;
            match annotation.range {
                (Some(start), ..) if start > line_end => false,
                (Some(start), Some(end)) if start >= line_start && end <= line_end => {
                    let range = (start - line_start, end - line_start);
                    body.insert(
                        body_idx + 1,
                        DisplayLine::AnnotationLine {
                            inline_marks: vec![],
                            range,
                            label: annotation.label.clone().unwrap_or("".to_string()),
                            annotation_type: DisplayAnnotationType::from(
                                annotation.annotation_type,
                            ),
                        },
                    );
                    annotation_line_count += 1;
                    true
                }
                (Some(start), Some(end))
                    if start >= line_start && start <= line_end && end > line_end =>
                {
                    if start - line_start == 0 {
                        match body[body_idx] {
                            DisplayLine::SourceLine {
                                ref mut inline_marks,
                                ..
                            } => {
                                inline_marks.push(DisplayMark::AnnotationStart);
                            }
                            _ => {}
                        }
                    } else {
                        let range = (start - line_start, start - line_start + 1);
                        body.insert(
                            body_idx + 1,
                            DisplayLine::AnnotationLine {
                                inline_marks: vec![DisplayMark::AnnotationThrough],
                                range,
                                label: annotation.label.clone().unwrap_or("".to_string()),
                                annotation_type: DisplayAnnotationType::MultilineStart,
                            },
                        );
                        annotation_line_count += 1;
                    }
                    false
                }
                (Some(start), Some(end)) if start < line_start && end > line_end => {
                    match body[body_idx] {
                        DisplayLine::SourceLine {
                            ref mut inline_marks,
                            ..
                        } => {
                            inline_marks.push(DisplayMark::AnnotationThrough);
                        }
                        _ => {}
                    }
                    false
                }
                (Some(start), Some(end))
                    if start < line_start && end >= line_start && end <= line_end =>
                {
                    match body[body_idx] {
                        DisplayLine::SourceLine {
                            ref mut inline_marks,
                            ..
                        } => {
                            inline_marks.push(DisplayMark::AnnotationThrough);
                        }
                        _ => {}
                    }
                    let range = (end - line_start, end - line_start + 1);
                    body.insert(
                        body_idx + 1,
                        DisplayLine::AnnotationLine {
                            inline_marks: vec![DisplayMark::AnnotationThrough],
                            range,
                            label: annotation.label.clone().unwrap_or("".to_string()),
                            annotation_type: DisplayAnnotationType::MultilineEnd,
                        },
                    );
                    annotation_line_count += 1;
                    true
                }
                _ => false,
            }
        });
    }

    // Fold
    let mut no_annotation_lines_counter = 0;
    let mut idx = 0;
    while idx < body.len() {
        match body[idx] {
            DisplayLine::AnnotationLine { .. } => {
                if no_annotation_lines_counter > 10 {
                    let fold_start = idx - no_annotation_lines_counter + 5;
                    let fold_end = idx - 2;
                    let fold_len = fold_end - fold_start;

                    let slice = &[DisplayLine::FoldLine];

                    body.splice(fold_start..fold_end, slice.iter().cloned());
                    idx -= fold_len - 1;
                }
                no_annotation_lines_counter += 0;
            }
            _ => no_annotation_lines_counter += 1,
        }
        idx += 1;
    }

    body.insert(0, DisplayLine::EmptySourceLine);
    body.push(DisplayLine::EmptySourceLine);
    return body;
}

impl From<Snippet> for DisplayList {
    fn from(snippet: Snippet) -> Self {
        let header = format_header(&snippet);
        let body = format_body(snippet);

        DisplayList {
            body: [&header[..], &body[..]].concat(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DisplayLine {
    RawLine(String),
    EmptySourceLine,
    SourceLine {
        lineno: usize,
        inline_marks: Vec<DisplayMark>,
        content: String,
    },
    AnnotationLine {
        inline_marks: Vec<DisplayMark>,
        range: (usize, usize),
        label: String,
        annotation_type: DisplayAnnotationType,
    },
    FoldLine,
}

#[derive(Debug, Clone)]
pub enum DisplayAnnotationType {
    Error,
    Warning,
    MultilineStart,
    MultilineEnd,
}

impl From<AnnotationType> for DisplayAnnotationType {
    fn from(at: AnnotationType) -> Self {
        match at {
            AnnotationType::Error => DisplayAnnotationType::Error,
            AnnotationType::Warning => DisplayAnnotationType::Warning,
        }
    }
}

#[derive(Debug, Clone)]
pub enum DisplayMark {
    AnnotationThrough,
    AnnotationStart,
}

impl fmt::Display for DisplayMark {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DisplayMark::AnnotationThrough => write!(f, "|"),
            DisplayMark::AnnotationStart => write!(f, "/"),
        }
    }
}
