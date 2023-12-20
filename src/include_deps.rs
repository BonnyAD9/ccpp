use utf8_chars::{BufReadCharsExt, Chars};

use crate::err::Result;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

macro_rules! next_chr {
    ($chars:ident, $res:expr) => {
        if let Some(c) = $chars.next() {
            c?;
        } else {
            return Ok($res);
        }
    };
}

pub struct IncFile {
    pub path: PathBuf,
    // when true file included as `"file"` otherwise included as `<file>`
    pub relative: bool,
}

struct CharReader<'a, R>
where
    R: BufRead,
{
    chars: Chars<'a, R>,
    cur: char,
}

impl<'a, R> CharReader<'a, R>
where
    R: BufRead,
{
    pub fn new(read: &'a mut R) -> Self {
        Self {
            chars: read.chars(),
            cur: ' ',
        }
    }

    fn esc_read_while<F>(&mut self, f: F) -> Result<String>
    where
        F: Fn(char) -> bool,
    {
        let mut res = String::new();

        loop {
            if self.cur == '\\' {
                next_chr!(self, res);
                if self.cur != '\n' {
                    if !f(self.cur) {
                        break Ok(res);
                    }
                    res.push(self.cur);
                    continue;
                }
                next_chr!(self, res);
                continue;
            }

            if !f(self.cur) {
                break Ok(res);
            }

            res.push(self.cur);
            next_chr!(self, res);
        }
    }

    fn esc_skip_while<F>(&mut self, f: F) -> Result<()>
    where
        F: Fn(char) -> bool,
    {
        loop {
            if self.cur == '\\' {
                next_chr!(self, ());
                if self.cur != '\n' {
                    if !f(self.cur) {
                        break Ok(());
                    }
                    continue;
                }
                next_chr!(self, ());
                continue;
            }

            if !f(self.cur) {
                break Ok(());
            }

            next_chr!(self, ());
        }
    }
}

impl<'a, R> Iterator for CharReader<'a, R>
where
    R: BufRead,
{
    type Item = Result<char>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.chars.next() {
            Some(Ok(c)) => {
                self.cur = c;
                Some(Ok(c))
            }
            Some(Err(e)) => Some(Err(e.into())),
            None => None,
        }
    }
}

pub fn get_included_files(file: &Path) -> Result<Vec<IncFile>> {
    let mut res = vec![];

    let mut file = BufReader::new(File::open(file)?);
    let mut chars = CharReader::new(&mut file);

    next_chr!(chars, res);

    let mut prev_newline = true;
    loop {
        match chars.cur {
            '\n' => {
                prev_newline = true;
                next_chr!(chars, res);
            }
            c if c.is_whitespace() => next_chr!(chars, res),
            '#' if prev_newline => {
                if let Some(f) = read_macro(&mut chars)? {
                    res.push(f);
                    prev_newline = true;
                }
            }
            '\'' => {
                prev_newline = false;
                read_char(&mut chars)?;
            }
            '"' => {
                prev_newline = false;
                read_string(&mut chars)?;
            }
            '/' => {
                next_chr!(chars, res);
                if chars.cur == '*' {
                    read_multiline_comment(&mut chars)?;
                    prev_newline = false;
                } else if chars.cur == '/' {
                    read_line_comment(&mut chars)?;
                    prev_newline = false;
                } else {
                    prev_newline = false;
                    next_chr!(chars, res);
                }
            }
            _ => {
                prev_newline = false;
                next_chr!(chars, res);
            }
        }
    }
}

fn read_macro<'a, R>(chars: &mut CharReader<'a, R>) -> Result<Option<IncFile>>
where
    R: BufRead,
{
    next_chr!(chars, None);
    chars.esc_skip_while(|c| c.is_whitespace())?;

    let mac = chars.esc_read_while(|c| c.is_alphanumeric())?;

    if mac != "include" {
        return chars.esc_skip_while(|c| c != '\n').map(|_| None);
    }

    chars.esc_skip_while(|c| c.is_whitespace())?;

    match chars.cur {
        '<' => {
            next_chr!(chars, None);
            let res = chars.esc_read_while(|c| c != '>')?;
            next_chr!(chars, None);
            Ok(Some(IncFile {
                path: res.into(),
                relative: false,
            }))
        }
        '"' => {
            next_chr!(chars, None);
            let res = chars.esc_read_while(|c| c != '"')?;
            next_chr!(chars, None);
            Ok(Some(IncFile {
                path: res.into(),
                relative: false,
            }))
        }
        _ => chars.esc_skip_while(|c| c != '\n').map(|_| None),
    }
}

fn read_char<'a, R>(chars: &mut CharReader<'a, R>) -> Result<()>
where
    R: BufRead,
{
    next_chr!(chars, ());
    while chars.cur != '\'' {
        if chars.cur == '\\' {
            next_chr!(chars, ());
        }
        next_chr!(chars, ());
    }

    Ok(())
}

fn read_string<'a, R>(chars: &mut CharReader<'a, R>) -> Result<()>
where
    R: BufRead,
{
    next_chr!(chars, ());
    while chars.cur != '"' {
        if chars.cur == '\\' {
            next_chr!(chars, ());
        }
        next_chr!(chars, ());
    }

    Ok(())
}

fn read_multiline_comment<'a, R>(chars: &mut CharReader<'a, R>) -> Result<()>
where
    R: BufRead,
{
    loop {
        if chars.cur != '*' {
            next_chr!(chars, ());
            continue;
        }

        next_chr!(chars, ());
        if chars.cur == '/' {
            next_chr!(chars, ());
            break Ok(());
        }
    }
}

fn read_line_comment<'a, R>(chars: &mut CharReader<'a, R>) -> Result<()>
where
    R: BufRead,
{
    chars.esc_skip_while(|c| c != '\n')
}
