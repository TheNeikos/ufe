use std::fmt::Write;

use ariadne::{ColorGenerator, FnCache, Label, Report};

use super::UserFacingError;

struct Context {
    _max_width: usize,
    first_run: bool,
}

impl Context {
    fn go_to_inner(&self) -> Context {
        Context {
            first_run: false,
            ..*self
        }
    }
}

/// Render a chain of errors to the user, meant to be displayed on the terminal
pub fn render_for_terminal(error: &UserFacingError, max_width: usize) -> String {
    let context = Context {
        _max_width: max_width,
        first_run: true,
    };
    render_for_terminal_inner(error, &context)
}

fn render_for_terminal_inner(error: &UserFacingError, context: &Context) -> String {
    let mut output = String::new();
    writeln!(&mut output, "{}", &error.error.summary).unwrap();

    if let Some(extended) = &error.error.extended_reason {
        writeln!(&mut output, "\n{}", extended).unwrap();
    }

    for fh in &error.error.file_highlights {
        let mut colors = ColorGenerator::new();

        let mut report = Report::build(ariadne::ReportKind::Error, fh.path.as_str(), 0);

        for l in &fh.labels {
            let color = colors.next();
            report = report.with_label(
                Label::new((fh.path.as_str(), l.range.clone()))
                    .with_message(l.message.clone())
                    .with_color(color),
            );
        }

        let report = report.finish();
        let mut src = FnCache::new(|_a: &&str| Ok(fh.content.clone()));
        let mut buf = vec![];
        report.write_for_stdout(&mut src, &mut buf).unwrap();
        write!(&mut output, "{}", String::from_utf8(buf).unwrap()).unwrap();
    }

    if !error.related.is_empty() {
        if context.first_run {
            writeln!(&mut output, "Detailed informations:").unwrap();
        }

        for err in &error.related {
            let inner_context = context.go_to_inner();
            write!(
                &mut output,
                "{}",
                render_for_terminal_inner(err, &inner_context)
            )
            .unwrap();
        }
    }

    output
}
