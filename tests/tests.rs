use std::env;
use std::ffi::OsStr;
use std::io::{self, Write, Read, BufRead, BufReader};
use std::path::PathBuf;
use std::process::{self, Command, Stdio};

fn xsvre_exe() -> io::Result<PathBuf> {
    let mut exe = env::current_exe()?;
    exe.pop();
    exe.pop();
    exe.push("csvre");
    Ok(exe)
}

fn command<I, S>(args: I, input: &[u8]) -> process::Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut child = Command::new(xsvre_exe().unwrap())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();

    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(input).unwrap();
    }

    child.wait_with_output().unwrap()
}

#[test]
fn numeric_column() {
    let output = command(
        &["-c", "1", "\\s+", ""],
        b"\
column1,column2,column3
foo,bar,baz
frob,n i z,lorem
ipsum,dolor,sit
",
    );

    assert!(output.status.success());

    assert_eq!(
        "\
column1,column2,column3
foo,bar,baz
frob,niz,lorem
ipsum,dolor,sit
",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn named_column() {
    let output = command(
        &["-c", "column2", "\\s+", ""],
        b"\
column1,column2,column3
foo,bar,baz
frob,n i z,lorem
ipsum,dolor,sit
",
    );

    assert!(output.status.success());

    assert_eq!(
        "\
column1,column2,column3
foo,bar,baz
frob,niz,lorem
ipsum,dolor,sit
",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn named_column_without_headers_fails() {
    let output = command(
        &["-c", "column2", "-n", "\\s+", ""],
        b"\
column1,column2,column3
foo,bar,baz
frob,n i z,lorem
ipsum,dolor,sit
",
    );

    assert!(!output.status.success());
}

#[test]
fn no_headers() {
    let output = command(
        &["-c", "1", "-n", "\\w+", "HELLO"],
        b"\
column1,column2,column3
foo,bar,baz
frob,n i z,lorem
ipsum,dolor,sit
",
    );

    assert!(output.status.success());

    assert_eq!(
        "\
column1,HELLO,column3
foo,HELLO,baz
frob,HELLO HELLO HELLO,lorem
ipsum,HELLO,sit
",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn change_delimiter() {
    let output = command(
        &["-c", "1", "-d", ";", "\\s+", ""],
        b"\
column1;column2;column3
foo;bar;baz
frob;n i z;lorem
ipsum;dolor;sit
",
    );

    assert!(output.status.success());

    assert_eq!(
        "\
column1;column2;column3
foo;bar;baz
frob;niz;lorem
ipsum;dolor;sit
",
        String::from_utf8_lossy(&output.stdout)
    );
}


#[test]
fn byte_mode() {
    let output = command(
        &["-c", "1", "-b", "(?-u)\\s+", ""],
        b"\
column1,column2,column3
foo,\0ar,baz
frob,\xc0 i z,lorem
ipsum,dolor,sit
",
    );

    assert!(output.status.success());

    assert_eq!(
        &b"\
column1,column2,column3
foo,\0ar,baz
frob,\xc0iz,lorem
ipsum,dolor,sit
"[..],
        output.stdout.as_slice()
    );
}
