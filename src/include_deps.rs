use utf8_chars::{BufReadCharsExt, Chars};

use crate::err::Result;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

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

macro_rules! next_chr {
    ($chars:ident, $res:ident) => {
        if let Some(c) = $chars.next() {
            c?;
        } else {
            return Ok($res);
        }
    };
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
            },
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
                }
            }
            _ => {
                prev_newline = false;
                next_chr!(chars, res);
            }
        }
    }

    Ok(res)
}

fn read_macro<'a, R>(chars: &mut CharReader<'a, R>) -> Result<Option<IncFile>>
where
    R: BufRead,
{
    todo!()
}

fn read_char<'a, R>(chars: &mut CharReader<'a, R>) -> Result<()>
where
    R: BufRead,
{
    todo!()
}

fn read_string<'a, R>(chars: &mut CharReader<'a, R>) -> Result<()>
where
    R: BufRead,
{
    todo!()
}

fn read_multiline_comment<'a, R>(chars: &mut CharReader<'a, R>) -> Result<()>
where
    R: BufRead,
{
    todo!()
}

fn read_line_comment<'a, R>(chars: &mut CharReader<'a, R>) -> Result<()>
where
    R: BufRead,
{
    todo!()
}
