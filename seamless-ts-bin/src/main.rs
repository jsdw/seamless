use clap::Parser;
use std::fmt;
use std::io::{self, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

/// Generate TypeScript types from the JSON info describing a seamless API.
///
/// The API info is read from stdin, or from the file or URL given by --file or
/// --url, and the generated TypeScript is written to stdout.
#[derive(Parser)]
#[command(name = "seamless-ts", version)]
struct Args {
    /// Read the API info from this file instead of stdin.
    #[arg(short, long, value_name = "PATH")]
    file: Option<PathBuf>,
    /// Fetch the API info from this URL instead of stdin.
    #[arg(short, long, value_name = "URL", conflicts_with = "file")]
    url: Option<String>,
    /// A header to send when fetching from --url, eg 'Authorization: Bearer foo'.
    /// Can be given multiple times.
    // Dev note: --header/-H for consistency with curl.
    #[arg(
        short = 'H',
        long = "header",
        value_name = "HEADER",
        value_parser = parse_header,
        requires = "url",
        conflicts_with = "file"
    )]
    headers: Vec<(String, String)>,
}

/// Parse a header given as 'Name: value'.
fn parse_header(header: &str) -> Result<(String, String), String> {
    let (name, value) = header
        .split_once(':')
        .ok_or("expected a header like 'Name: value'")?;
    Ok((name.trim().to_owned(), value.trim().to_owned()))
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Args) -> Result<(), String> {
    let route_info = match (args.file, args.url) {
        (Some(path), _) => std::fs::read_to_string(&path)
            .map_err(|e| format!("Could not read '{}': {e}", path.display()))?,
        (_, Some(url)) => args
            .headers
            .iter()
            .fold(ureq::get(&url), |req, (name, value)| {
                req.header(name, value)
            })
            .call()
            .and_then(|mut res| res.body_mut().read_to_string())
            .map_err(|e| format!("Could not fetch '{url}': {e}"))?,
        (None, None) => {
            let mut s = String::new();
            io::stdin()
                .read_to_string(&mut s)
                .map_err(|e| format!("Could not read from stdin: {e}"))?;
            s
        }
    };

    let mut out = IoWriter::new(BufWriter::new(io::stdout().lock()));
    let write_error = |e: io::Error| format!("Could not write to stdout: {e}");
    match seamless_ts::generate_types(&route_info, &mut out) {
        Ok(()) => {}
        Err(seamless_ts::Error::Write(_)) => {
            return Err(write_error(out.take_error()));
        }
        Err(e) => return Err(e.to_string()),
    }
    out.inner.flush().map_err(write_error)
}

/// Adapts an [`io::Write`] to a [`fmt::Write`], keeping hold of any IO error
/// so that we can report it (since [`fmt::Error`] carries no information).
struct IoWriter<W> {
    inner: W,
    error: Option<io::Error>,
}

impl<W> IoWriter<W> {
    fn new(inner: W) -> Self {
        IoWriter { inner, error: None }
    }

    fn take_error(&mut self) -> io::Error {
        self.error
            .take()
            .unwrap_or_else(|| io::Error::other("formatting error"))
    }
}

impl<W: io::Write> fmt::Write for IoWriter<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.inner.write_all(s.as_bytes()).map_err(|e| {
            self.error = Some(e);
            fmt::Error
        })
    }
}
