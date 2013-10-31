// From http://dev.w3.org/csswg/css-syntax/#tokenization

#[link(name = "css_lex", vers = "0.0")];

#[deriving(Eq)]
pub struct NumericValue {
    representation: ~str,
    value: f64,
    int_value: Option<i64>,
}

pub enum LexState {
    StartState,
    DqStringState,
    MaybeHashState,
    MaybeSuffixMatchState,
    SqStringState,
    MaybeSubstringMatchState,
    MaybeNumberState,
    FirstHyphenState,
    MaybeCommentState,
    MaybeCdoState,
    MaybeAtState,
    MaybeEscapeState,
    MaybePrefixMatchState,

}

#[deriving(Eq)]
pub struct SourceLocation {
    line: uint,  // First line is 1
    column: uint,  // First character of a line is at column 1
}

pub enum Token {
    Ident(~str),
    Function(~str),
    AtKeyword(~str),
    Hash(~str),
    IdHash(~str),
    String(~str),
    BadString,
    Url(~str),
    BadUrl,
    Delim(char),
    Number(~NumericValue),
    Percentage(~NumericValue),
    Dimension(~NumericValue, ~str),
    UnicodeRange(u32, u32),
    IncludeMatch,
    DashMatch,
    PrefixMatch,
    SuffixMatch,
    SubstringMatch,
    Column,
    Whitespace,
    CDO,
    CDC,
    Colon,
    Semicolon,
    Comma,
    LeftBracket,
    RightBracket,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
}

pub type Node = (Token, SourceLocation);

struct Tokenizer {
    // Won't be able to be an owned pointer, since will be shared across tasks
    priv input: ~str,
    priv length: uint,
    priv position: uint,
    priv line: uint,
    priv last_line_start: uint,
}

impl Tokenizer {
    #[inline]
    fn is_eof(&self) -> bool { self.position >= self.length }

    // Assumes non-EOF
    #[inline]
    fn current_char(&self) -> char { self.char_at(0) }

    #[inline]
    fn char_at(&self, offset: uint) -> char {
        self.input.char_at(self.position + offset)
    }

    #[inline]
    fn consume_char(&mut self) -> char {
        let range = self.input.char_range_at(self.position);
        self.position = range.next;
        range.ch
    }

    #[inline]
    fn starts_with(&self, needle: &str) -> bool {
        self.input.slice_from(self.position).starts_with(needle)
    }

    #[inline]
    fn new_line(&mut self) {
        if cfg!(test) {
            assert!(self.input.char_at(self.position - 1) == '\n')
        }
        self.line += 1;
        self.last_line_start = self.position;
    }

    // Checks whether the Tokenizer has at least `num` characters remaining
    #[inline]
    fn has_more(&self, num: uint) -> bool { self.position + num < self.length }
}

macro_rules! is_match(
    ($value:expr, $($pattern:pat)|+) => (
        match $value { $($pattern)|+ => true, _ => false }
    );
)

// From http://dev.w3.org/csswg/css-syntax/#consume-a-token
fn next_token(tokenizer: &mut Tokenizer) -> Option<Token> {
    if tokenizer.is_eof() {
        return None
    }
    let start_location = SourceLocation{
        line: tokenizer.line,
        // The start of the line is column 1:
        column: tokenizer.position - tokenizer.last_line_start + 1,
    };
    let c = tokenizer.current_char();
    let token = match c {
        '\t' | '\n' | ' ' => {
            while !tokenizer.is_eof() {
                match tokenizer.current_char() {
                    ' ' | '\t' => tokenizer.position += 1,
                    '\n' => {
                        tokenizer.position += 1;
                        tokenizer.new_line();
                    },
                    _ => break,
                }
            }
            WhiteSpace
        },
        "\"" => consume_string(tokenizer, false),
        '#' => {
            tokenizer.position += 1;
            if is_ident_start(tokenizer) { IDHash(consume_name(tokenizer)) }
                else if !tokenizer.is_eof() && match tokenizer.current_char() {
                'a'..'z' | 'A'..'Z' | '0'..'9' | '-' | '_' => true,
                '\\' => !tokenizer.starts_with("\\\n"),
                _ => c > '\x7F',  // Non-ASCII
            } { Hash(consume_name(tokenizer)) }
                else { Delim(c) }
        },
        '$' => {
            if tokenizer.starts_with("$=") { tokenizer.position += 2; SuffixMatch }
                else { tokenizer.position += 1; Delim(c) }
        },
        "'" => consume_string(tokenizer, true),
        "(" => { tokenizer.position += 1; LeftParen },
        ")" => { tokenizer.position += 1; RightParen },
        '*' => {
            if tokenizer.starts_with("*=") {
                tokenizer.position += 2;
                SubstringMatch
            } else { tokenizer.position += 1; Delim(c) }
        },
        '+' => {
            if (tokenizer.position + 1 < tokenizer.length
                && is_match!(tokenizer.char_at(1), '0'..'9')
                ) || (tokenizer.position + 2 < tokenizer.length
                      && tokenizer.char_at(1) == '.'
                      && is_match!(tokenizer.char_at(2), '0'..'9')
                      ) {
                consume_numeric(tokenizer)
            } else {
                tokenizer.position += 1;
                Delim(c)
            }
        },
        ',' => { tokenizer.position += 1; Comma },
        '-' => {
            if (
                tokenizer.position + 1 < tokenizer.length
                    && is_match!(tokenizer.char_at(1), '0'..'9')
                    ) || (
                tokenizer.position + 2 < tokenizer.length
                    && tokenizer.char_at(1) == '.'
                    && is_match!(tokenizer.char_at(2), '0'..'9')
                    ) {
                consume_numeric(tokenizer)
            } else if is_ident_start(tokenizer) {
                consume_ident_like(tokenizer)
            } else if tokenizer.starts_with("-->") {
                tokenizer.position += 3;
                CDC
            } else {
                tokenizer.position += 1;
                Delim(c)
            }
        },
        '.' => {
            if tokenizer.position + 1 < tokenizer.length
                && is_match!(tokenizer.char_at(1), '0'..'9') {
                consume_numeric(tokenizer)
            } else {
                tokenizer.position += 1;
                Delim(c)
            }
        }
        '0'..'9' => consume_numeric(tokenizer),
        ':' => { tokenizer.position += 1; Colon },
        ';' => { tokenizer.position += 1; Semicolon },
        '<' => {
            if tokenizer.starts_with("<!--") {
                tokenizer.position += 4;
                CDO
            } else {
                tokenizer.position += 1;
                Delim(c)
            }
        },
        '@' => {
            tokenizer.position += 1;
            if is_ident_start(tokenizer) { AtKeyword(consume_name(tokenizer)) }
                else { Delim(c) }
        },
        'u' | 'U' => {
            if tokenizer.position + 2 < tokenizer.length
                && tokenizer.char_at(1) == '+'
                && is_match!(tokenizer.char_at(2), '0'..'9' | 'a'..'f' | 'A'..'F' | '?')
                { consume_unicode_range(tokenizer) }
                else { consume_ident_like(tokenizer) }
        },
        'a'..'z' | 'A'..'Z' | '_' => consume_ident_like(tokenizer),
        '[' => SquareBracketBlock(consume_block(tokenizer, CloseSquareBracket)),
          '\\' => {
                if !tokenizer.starts_with("\\\n") { consume_ident_like(tokenizer) }
                    else { tokenizer.position += 1; Delim(c) }
            },
          ']' => { tokenizer.position += 1; CloseSquareBracket },
        '^' => {
            if tokenizer.starts_with("^=") { tokenizer.position += 2; PrefixMatch }
                else { tokenizer.position += 1; Delim(c) }
        },
        '{' => CurlyBracketBlock(consume_block_with_location(tokenizer, CloseCurlyBracket)),
          '|' => {
                if tokenizer.starts_with("|=") { tokenizer.position += 2; DashMatch }
                    else if tokenizer.starts_with("||") { tokenizer.position += 2; Column }
                    else { tokenizer.position += 1; Delim(c) }
            },
          '}' => { tokenizer.position += 1; CloseCurlyBracket },
        '~' => {
            if tokenizer.starts_with("~=") { tokenizer.position += 2; IncludeMatch }
                else { tokenizer.position += 1; Delim(c) }
        },
        _ => {
            if c > '\x7F' {  // Non-ASCII
                consume_ident_like(tokenizer)
            } else {
                tokenizer.position += 1;
                Delim(c)
            }
        },
    };
    Some((token, start_location))
}


#[inline]
fn consume_comments(tokenizer: &mut Tokenizer) {
    while tokenizer.starts_with("/*") {
        tokenizer.position += 2;  // +2 to consume "/*"
        while !tokenizer.is_eof() {
            match tokenizer.consume_char() {
                '*' => {
                    if !tokenizer.is_eof() && tokenizer.current_char() == '/' {
                        tokenizer.position += 1;
                        break
                    }
                },
                '\n' => tokenizer.new_line(),
                _ => ()
            }
        }
    }
}


fn consume_block(tokenizer: &mut Tokenizer, ending_token: ComponentValue) -> ~[ComponentValue] {
    tokenizer.position += 1;  // Skip the initial {[(
    let mut content = ~[];
    loop {
        match next_component_value(tokenizer) {
            Some((component_value, _location)) => {
                if component_value == ending_token { break }
                else { content.push(component_value) }
            },
            None => break,
        }
    }
    content
}


fn consume_block_with_location(tokenizer: &mut Tokenizer, ending_token: ComponentValue) -> ~[Node] {
    tokenizer.position += 1;  // Skip the initial {[(
    let mut content = ~[];
    loop {
        match next_component_value(tokenizer) {
            Some((component_value, location)) => {
                if component_value == ending_token { break }
                else { content.push((component_value, location)) }
            },
            None => break,
        }
    }
    content
}

// From http://dev.w3.org/csswg/css-syntax/#consume-a-string-token0
fn consume_string(tokenizer: &mut Tokenizer, single_quote: bool) -> Token {
    match consume_quoted_string(tokenizer, single_quote) {
        Some(value) => String(value),
        None => BadString
    }
}

// Return None on syntax error (ie. unescaped newline)
fn consume_quoted_string(tokenizer: &mut Tokenizer, single_quote: bool) -> Option<~str> {
    tokenizer.position += 1;  // Skip the initial quote
    let mut string: ~str = ~"";
    while !tokenizer.is_eof() {
        match tokenizer.consume_char() {
            "\"" if !single_quote => break,
            "'" if single_quote => break,
            '\n' => {
                tokenizer.position -= 1;
                return None;
            },
            "\\" => {
                if !tokenizer.is_eof() {
                    if tokenizer.current_char() == "\n" {  // Escaped newline
                        tokenizer.position += 1;
                        tokenizer.new_line();
                    }
                    else { string.push_char(consume_escape(tokenizer)) }
                }
                // else: escaped EOF, do nothing.
            }
            c => string.push_char(c),
        }
    }
    Some(string)
}


#[inline]
fn is_ident_start(tokenizer: &mut Tokenizer) -> bool {
    !tokenizer.is_eof() && match tokenizer.current_char() {
        'a'..'z' | 'A'..'Z' | '_' => true,
        '-' => tokenizer.position + 1 < tokenizer.length && match tokenizer.char_at(1) {
            'a'..'z' | 'A'..'Z' | '_' => true,
            '\\' => !tokenizer.input.slice_from(tokenizer.position + 1).starts_with("\\\n"),
            c => c > '\x7F',  // Non-ASCII
        },
        '\\' => !tokenizer.starts_with("\\\n"),
        c => c > '\x7F',  // Non-ASCII
    }
}


fn consume_ident_like(tokenizer: &mut Tokenizer) -> ComponentValue {
    let value = consume_name(tokenizer);
    if !tokenizer.is_eof() && tokenizer.current_char() == '(' {
        if value.eq_ignore_ascii_case("url") { consume_url(tokenizer) }
        else { Function(value, consume_block(tokenizer, CloseParenthesis)) }
    } else {
        Ident(value)
    }
}

fn consume_name(tokenizer: &mut Tokenizer) -> ~str {
    let mut value = ~"";
    while !tokenizer.is_eof() {
        let c = tokenizer.current_char();
        value.push_char(match c {
            'a'..'z' | 'A'..'Z' | '0'..'9' | '_' | '-'  => { tokenizer.position += 1; c },
            '\\' => {
                if tokenizer.starts_with("\\\n") { break }
                tokenizer.position += 1;
                consume_escape(tokenizer)
            },
            _ => if c > '\x7F' { tokenizer.consume_char() }  // Non-ASCII
                 else { break }
        })
    }
    value
}


fn consume_numeric(tokenizer: &mut Tokenizer) -> ComponentValue {
    // Parse [+-]?\d*(\.\d+)?([eE][+-]?\d+)?
    // But this is always called so that there is at least one digit in \d*(\.\d+)?
    let mut representation = ~"";
    let mut is_integer = true;
    if is_match!(tokenizer.current_char(), '-' | '+') {
         representation.push_char(tokenizer.consume_char())
    }
    while !tokenizer.is_eof() {
        match tokenizer.current_char() {
            '0'..'9' => representation.push_char(tokenizer.consume_char()),
            _ => break
        }
    }
    if tokenizer.position + 1 < tokenizer.length && tokenizer.current_char() == '.'
            && is_match!(tokenizer.char_at(1), '0'..'9') {
        is_integer = false;
        representation.push_char(tokenizer.consume_char());  // '.'
        representation.push_char(tokenizer.consume_char());  // digit
        while !tokenizer.is_eof() {
            match tokenizer.current_char() {
                '0'..'9' => representation.push_char(tokenizer.consume_char()),
                _ => break
            }
        }
    }
    if (
        tokenizer.position + 1 < tokenizer.length
        && is_match!(tokenizer.current_char(), 'e' | 'E')
        && is_match!(tokenizer.char_at(1), '0'..'9')
    ) || (
        tokenizer.position + 2 < tokenizer.length
        && is_match!(tokenizer.current_char(), 'e' | 'E')
        && is_match!(tokenizer.char_at(1), '+' | '-')
        && is_match!(tokenizer.char_at(2), '0'..'9')
    ) {
        is_integer = false;
        representation.push_char(tokenizer.consume_char());  // 'e' or 'E'
        representation.push_char(tokenizer.consume_char());  // sign or digit
        // If the above was a sign, the first digit it consumed below
        // and we make one extraneous is_eof() check.
        while !tokenizer.is_eof() {
            match tokenizer.current_char() {
                '0'..'9' => representation.push_char(tokenizer.consume_char()),
                _ => break
            }
        }
    }
    // TODO: handle overflow
    let value = NumericValue {
        int_value: if is_integer { Some(
            // Remove any + sign as int::from_str() does not parse them.
            if representation[0] != '+' as u8 {
                from_str(representation)
            } else {
                from_str(representation.slice_from(1))
            }.unwrap()
        )} else { None },
        value: from_str(representation).unwrap(),
        representation: representation,
    };
    if !tokenizer.is_eof() && tokenizer.current_char() == '%' {
        tokenizer.position += 1;
        Percentage(value)
    }
    else if is_ident_start(tokenizer) { Dimension(value, consume_name(tokenizer)) }
    else { Number(value) }
}


fn consume_url(tokenizer: &mut Tokenizer) -> ComponentValue {
    tokenizer.position += 1;  // Skip the ( of url(
    while !tokenizer.is_eof() {
        match tokenizer.current_char() {
            '\t' | '\n' | ' ' => tokenizer.position += 1,
            "\"" => return consume_quoted_url(tokenizer, false),
            '\'' => return consume_quoted_url(tokenizer, true),
            ')' => { tokenizer.position += 1; break },
            _ => return consume_unquoted_url(tokenizer),
        }
    }
    return URL(~"");

    fn consume_quoted_url(tokenizer: &mut Tokenizer, single_quote: bool) -> ComponentValue {
        match consume_quoted_string(tokenizer, single_quote) {
            Some(value) => consume_url_end(tokenizer, value),
            None => consume_bad_url(tokenizer),
        }
    }

    fn consume_unquoted_url(tokenizer: &mut Tokenizer) -> ComponentValue {
        let mut string = ~"";
        while !tokenizer.is_eof() {
            let next_char = match tokenizer.consume_char() {
                ' ' | '\t' => return consume_url_end(tokenizer, string),
                '\n' => {
                    tokenizer.new_line();
                    return consume_url_end(tokenizer, string)
                },
                ')' => break,
                '\x00'..'\x08' | '\x0B' | '\x0E'..'\x1F' | '\x7F'  // non-printable
                    | "\"" | '\'' | '(' => return consume_bad_url(tokenizer),
                '\\' => {
                    if !tokenizer.is_eof() && tokenizer.current_char() == '\n' {
                        return consume_bad_url(tokenizer)
                    }
                    consume_escape(tokenizer)
                },
                c => c
            };
            string.push_char(next_char)
        }
        URL(string)
    }

    fn consume_url_end(tokenizer: &mut Tokenizer, string: ~str) -> ComponentValue {
        while !tokenizer.is_eof() {
            match tokenizer.consume_char() {
                ' ' | '\t' => (),
                '\n' => tokenizer.new_line(),
                ')' => break,
                _ => return consume_bad_url(tokenizer)
            }
        }
        URL(string)
    }

    fn consume_bad_url(tokenizer: &mut Tokenizer) -> ComponentValue {
        // Consume up to the closing )
        while !tokenizer.is_eof() {
            match tokenizer.consume_char() {
                ')' => break,
                '\\' => tokenizer.position += 1, // Skip an escaped ')' or '\'
                '\n' => tokenizer.new_line(),
                _ => ()
            }
        }
        BadURL
    }
}



fn consume_unicode_range(tokenizer: &mut Tokenizer) -> ComponentValue {
    tokenizer.position += 2;  // Skip U+
    let mut hex = ~"";
    while hex.len() < 6 && !tokenizer.is_eof()
          && is_match!(tokenizer.current_char(), '0'..'9' | 'A'..'F' | 'a'..'f') {
        hex.push_char(tokenizer.consume_char());
    }
    let max_question_marks = 6u - hex.len();
    let mut question_marks = 0u;
    while question_marks < max_question_marks && !tokenizer.is_eof()
            && tokenizer.current_char() == '?' {
        question_marks += 1;
        tokenizer.position += 1
    }
    let start;
    let end;
    if question_marks > 0 {
        start = num::from_str_radix(hex + "0".repeat(question_marks), 16).unwrap();
        end = num::from_str_radix(hex + "F".repeat(question_marks), 16).unwrap();
    } else {
        start = num::from_str_radix(hex, 16).unwrap();
        hex = ~"";
        if !tokenizer.is_eof() && tokenizer.current_char() == '-' {
            tokenizer.position += 1;
            while hex.len() < 6 && !tokenizer.is_eof() {
                let c = tokenizer.current_char();
                match c {
                    '0'..'9' | 'A'..'F' | 'a'..'f' => {
                        hex.push_char(c); tokenizer.position += 1 },
                    _ => break
                }
            }
        }
        end = if hex.len() > 0 { num::from_str_radix(hex, 16).unwrap() } else { start }
    }
    UnicodeRange {start: start, end: end}
}


// Assumes that the U+005C REVERSE SOLIDUS (\) has already been consumed
// and that the next input character has already been verified
// to not be a newline.
fn consume_escape(tokenizer: &mut Tokenizer) -> char {
    if tokenizer.is_eof() { return '\uFFFD' }  // Escaped EOF
    let c = tokenizer.consume_char();
    match c {
        '0'..'9' | 'A'..'F' | 'a'..'f' => {
            let mut hex = str::from_char(c);
            while hex.len() < 6 && !tokenizer.is_eof() {
                let c = tokenizer.current_char();
                match c {
                    '0'..'9' | 'A'..'F' | 'a'..'f' => {
                        hex.push_char(c); tokenizer.position += 1 },
                    _ => break
                }
            }
            if !tokenizer.is_eof() {
                match tokenizer.current_char() {
                    ' ' | '\t' => tokenizer.position += 1,
                    '\n' => { tokenizer.position += 1; tokenizer.new_line() },
                    _ => ()
                }
            }
            static REPLACEMENT_CHAR: char = '\uFFFD';
            let c: u32 = num::from_str_radix(hex, 16).unwrap();
            if c != 0 {
                let c = char::from_u32(c);
                c.unwrap_or(REPLACEMENT_CHAR)
            } else {
                REPLACEMENT_CHAR
            }
        },
        c => c
    }
}
