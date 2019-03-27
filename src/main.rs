use std::error::Error;
use std::io;
use std::process;

use csv;
use docopt;
use regex;
use serde_derive::Deserialize;

#[derive(Debug)]
enum MyError {
    ColumnNotFound,
    Csv(csv::Error),
    Io(io::Error),
    Regex(regex::Error),
    ParseInt(std::num::ParseIntError),
}

impl Error for MyError {}

impl std::fmt::Display for MyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MyError::ColumnNotFound => write!(f, "column not found"),
            MyError::Csv(e) => e.fmt(f),
            MyError::Io(e) => e.fmt(f),
            MyError::Regex(e) => e.fmt(f),
            MyError::ParseInt(e) => e.fmt(f),
        }
    }
}

impl From<csv::Error> for MyError {
    fn from(error: csv::Error) -> Self {
        if error.is_io_error() {
            let error = error.into_kind();
            match error {
                csv::ErrorKind::Io(e) => MyError::Io(e),
                _ => unreachable!(),
            }
        } else {
            MyError::Csv(error)
        }
    }
}

impl From<io::Error> for MyError {
    fn from(error: io::Error) -> Self {
        MyError::Io(error)
    }
}

impl From<regex::Error> for MyError {
    fn from(error: regex::Error) -> Self {
        MyError::Regex(error)
    }
}

impl From<std::num::ParseIntError> for MyError {
    fn from(error: std::num::ParseIntError) -> Self {
        MyError::ParseInt(error)
    }
}

const USAGE: &'static str = "
csvre

A simple tool for replacing data in CSV columns with regular
expressions.

USAGE:

    csvre [options] --column=COLUMN <regex> <replacement>
    csvre (-h | --help)
    csvre --version

ARGUMENTS:

    <regex>

        Regular expression used for matching.

        For syntax documentation, see
        https://docs.rs/regex/1.1.2/regex/#syntax

        Some information about unicode handling can be found from
        https://docs.rs/regex/1.1.2/regex/#unicode

    <replacement>

        Replacement string.

        You can reference named capture groups in the regex with $name and
        ${name} syntax. You can also use integers to reference capture
        groups with $0 being the whole match, $1 the first group and so on.

        If a capture group is not valid (name does not exist or index is
        invalid), it is replaced with the empty string.

        To insert a literal $, use $$.

OPTIONS:

    -h, --help

        Show this message.

    --version

        Show the version number.

    -d DELIM, --delimiter=DELIM

        Field delimiter. This is used for both input and output.
        [default: ,]

    -c COLUMN, --column=COLUMN

        Which column to operate on.

        You can either use the column name or zero based index. If
        you specify --no-headers, then you can only use the index
        here.

    -n, --no-headers

        The input does not have a header row.

        If you use this option, you can do matching against the first
        row of input.

    -b, --bytes

        Don't assume utf-8 input, work on raw bytes instead.

        See https://docs.rs/regex/1.1.2/regex/bytes/index.html#syntax
        for differences to the normal matching rules.
";

#[derive(Deserialize)]
struct Args {
    arg_regex: String,
    arg_replacement: String,
    flag_delimiter: String,
    flag_column: String,
    flag_no_headers: bool,
    flag_bytes: bool,
}

fn main() {
    match run() {
        Ok(()) => (),
        Err(error) => {
            match error {
                MyError::Io(ref error) => {
                    if error.kind() == io::ErrorKind::BrokenPipe {
                        return;
                    }
                }
                _ => (),
            }
            eprintln!("error: {}", error);
            process::exit(1);
        }
    }
}

fn run() -> Result<(), MyError> {
    let version = format!(
        "{}.{}.{}",
        env!("CARGO_PKG_VERSION_MAJOR"),
        env!("CARGO_PKG_VERSION_MINOR"),
        env!("CARGO_PKG_VERSION_PATCH")
    );

    let args: Args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.help(true).version(Some(version)).deserialize())
        .unwrap_or_else(|e| e.exit());

    let delimiter = args.flag_delimiter.as_bytes()[0];
    let column_str = args.flag_column;

    // (Ab)use Result as kind of an Either type ... :-)

    let re = if args.flag_bytes {
        Err(regex::bytes::Regex::new(&args.arg_regex)?)
    } else {
        Ok(regex::Regex::new(&args.arg_regex)?)
    };

    let replacement = if args.flag_bytes {
        Err(args.arg_replacement.as_bytes())
    } else {
        Ok(args.arg_replacement.as_str())
    };

    let mut reader = csv::ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(!args.flag_no_headers)
        .flexible(true)
        .from_reader(io::stdin());

    let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .flexible(true)
        .from_writer(io::stdout());

    // If we have headers, and we cannot parse column as an integer,
    // then we try to check if the column is included in the headers.
    let column_index: usize = if reader.has_headers() {
        reader.byte_headers()?;
        match column_str.parse() {
            Ok(n) => n,
            Err(_) => {
                if args.flag_bytes {
                    reader.byte_headers()?
                        .iter()
                        .position(|x| x == column_str.as_bytes())
                        .ok_or(MyError::ColumnNotFound)?
                } else {
                    reader.headers()?
                        .iter()
                        .position(|x| x == column_str)
                        .ok_or(MyError::ColumnNotFound)?
                }
            }
        }
    } else {
        column_str.parse()?
    };

    if args.flag_bytes {
        run_bytes(
            &mut reader,
            &mut writer,
            column_index,
            re.as_ref().unwrap_err(),
            replacement.unwrap_err(),
        )?;
    } else {
        run_string(
            &mut reader,
            &mut writer,
            column_index,
            re.as_ref().unwrap(),
            replacement.unwrap(),
        )?;
    }

    writer.flush()?;

    Ok(())
}

fn run_string<R, W>(
    reader: &mut csv::Reader<R>,
    writer: &mut csv::Writer<W>,
    column_index: usize,
    re: &regex::Regex,
    replacement: &str,
) -> Result<(), MyError>
where
    R: io::Read,
    W: io::Write,
{
    let mut record_in = csv::StringRecord::new();
    let mut record_out = csv::StringRecord::new();

    if reader.has_headers() {
        writer.write_record(reader.headers()?)?;
    }

    while reader.read_record(&mut record_in)? {
        record_out.clear();

        for index in 0..record_in.len() {
            let field = record_in.get(index).unwrap();
            let result = if index == column_index {
                re.replace_all(field, replacement)
            } else {
                std::borrow::Cow::Borrowed(field)
            };
            record_out.push_field(&result);
        }

        writer.write_record(&record_out)?;
    }

    Ok(())
}

fn run_bytes<R, W>(
    reader: &mut csv::Reader<R>,
    writer: &mut csv::Writer<W>,
    column_index: usize,
    re: &regex::bytes::Regex,
    replacement: &[u8],
) -> Result<(), MyError>
where
    R: io::Read,
    W: io::Write,
{
    let mut record_in = csv::ByteRecord::new();
    let mut record_out = csv::ByteRecord::new();

    if reader.has_headers() {
        writer.write_byte_record(reader.byte_headers()?)?;
    }

    while reader.read_byte_record(&mut record_in)? {
        record_out.clear();

        for index in 0..record_in.len() {
            let field = record_in.get(index).unwrap();
            let result = if index == column_index {
                re.replace_all(field, replacement)
            } else {
                std::borrow::Cow::Borrowed(field)
            };
            record_out.push_field(&result);
        }

        writer.write_byte_record(&record_out)?;
    }

    Ok(())
}
