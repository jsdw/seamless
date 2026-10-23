//! Helpers for formatting the generated TypeScript.

use core::fmt::{self, Write};
use seamless_info::RouteInfo;

const INDENT: &str = "    ";

/// Write `indent` levels of indentation.
pub fn write_indent<W: Write + ?Sized>(out: &mut W, indent: usize) -> fmt::Result {
    for _ in 0..indent {
        out.write_str(INDENT)?;
    }
    Ok(())
}

/// Write a doc comment for a route like the following:
///
/// ```text
/// /**
///  * POST /users.get_by_id
///  *
///  * Route description
///  */
/// ```
pub fn write_route_comment<W: Write + ?Sized>(
    out: &mut W,
    info: &RouteInfo,
    indent: usize,
) -> fmt::Result {
    write_indent(out, indent)?;
    writeln!(out, "/**")?;
    write_indent(out, indent)?;
    writeln!(
        out,
        " * {} /{}",
        Comment(&info.method),
        Comment(info.name.trim_start_matches('/'))
    )?;
    let description = info.description.trim();
    if !description.is_empty() {
        write_indent(out, indent)?;
        writeln!(out, " *")?;
        write_comment_lines(out, description, indent)?;
    }
    write_indent(out, indent)?;
    writeln!(out, " */")
}

/// Write each line of some text as ` * line`, as part of a block comment.
fn write_comment_lines<W: Write + ?Sized>(out: &mut W, text: &str, indent: usize) -> fmt::Result {
    for line in text.lines() {
        write_indent(out, indent)?;
        let line = line.trim();
        if line.is_empty() {
            writeln!(out, " *")?;
        } else {
            writeln!(out, " * {}", Comment(line))?;
        }
    }
    Ok(())
}

/// Write a doc comment for some object property. Single line descriptions are written
/// as `/** description */`, and longer ones are written as a block.
pub fn write_doc_comment<W: Write + ?Sized>(
    out: &mut W,
    description: &str,
    indent: usize,
) -> fmt::Result {
    let description = description.trim();
    if description.is_empty() {
        return Ok(());
    }
    write_indent(out, indent)?;
    if !description.contains('\n') {
        return writeln!(out, "/** {} */", Comment(description));
    }
    writeln!(out, "/**")?;
    write_comment_lines(out, description, indent)?;
    write_indent(out, indent)?;
    writeln!(out, " */")
}

/// Display an object key, quoting it if it isn't a valid identifier.
pub struct PropertyKey<'a>(pub &'a str);

impl fmt::Display for PropertyKey<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut chars = self.0.chars();
        let is_ident = chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
        if is_ident {
            f.write_str(self.0)
        } else {
            DoubleQuoted(self.0).fmt(f)
        }
    }
}

/// Display a string as a double quoted string literal.
pub struct DoubleQuoted<'a>(pub &'a str);

impl fmt::Display for DoubleQuoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_quoted(f, self.0, '"')
    }
}

/// Display a string as a single quoted string literal.
pub struct SingleQuoted<'a>(pub &'a str);

impl fmt::Display for SingleQuoted<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write_quoted(f, self.0, '\'')
    }
}

fn write_quoted(f: &mut fmt::Formatter<'_>, s: &str, quote: char) -> fmt::Result {
    f.write_char(quote)?;
    for c in s.chars() {
        match c {
            '\\' => f.write_str("\\\\")?,
            '\n' => f.write_str("\\n")?,
            '\r' => f.write_str("\\r")?,
            '\t' => f.write_str("\\t")?,
            c if c == quote => {
                f.write_char('\\')?;
                f.write_char(c)?;
            }
            c if c.is_control() || c == '\u{2028}' || c == '\u{2029}' => {
                write!(f, "\\u{{{:x}}}", c as u32)?;
            }
            c => f.write_char(c)?,
        }
    }
    f.write_char(quote)
}

/// Display some text such that it can't end the comment that it's in.
struct Comment<'a>(&'a str);

impl fmt::Display for Comment<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = self.0.split("*/");
        if let Some(first) = parts.next() {
            f.write_str(first)?;
        }
        for part in parts {
            f.write_str("*\\/")?;
            f.write_str(part)?;
        }
        Ok(())
    }
}
