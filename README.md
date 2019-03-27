# csvre

A simple tool for replacing data in CSV columns with regular
expressions.

## USAGE

    csvre [options] --column=COLUMN <regex> <replacement>
    csvre (-h | --help)
    csvre --version

## ARGUMENTS

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

## OPTIONS

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
