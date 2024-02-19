use utf8_chars::{BufReadCharsExt, Chars};

use crate::{dependency::DepFile, err::Result};
use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
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

pub enum IncFile {
    User(PathBuf),
    System(PathBuf),
    ExpModule(String),
    ImpModule(String),
    ExpImpModule(String),
    UserModule(PathBuf),
    SystemModule(PathBuf),
    ExpUserModule(PathBuf),
    ExpSystemModule(PathBuf),
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

    fn read_while<F>(&mut self, f: F) -> Result<String> where F: Fn(char) -> bool {
        let mut res = String::new();
        self.read_to_while(f, &mut res)?;
        Ok(res)
    }

    fn read_to_while<F>(&mut self, f: F, res: &mut String) -> Result<()> where F: Fn(char) -> bool {
        loop {
            if !f(self.cur) {
                return Ok(());
            }
            res.push(self.cur);
            next_chr!(self, ());
        }
    }

    fn skip_while<F>(&mut self, f: F) -> Result<()> where F: Fn(char) -> bool {
        loop {
            if !f(self.cur) {
                return Ok(());
            }
            next_chr!(self, ());
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

pub fn get_included_files(file: &DepFile) -> Result<Vec<IncFile>> {
    let mut res = vec![];

    let mut file = BufReader::new(File::open(file)?);
    let mut chars = CharReader::new(&mut file);

    next_chr!(chars, res);

    let mut module_section = true;
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
                } else if chars.cur == '/' {
                    read_line_comment(&mut chars)?;
                } else {
                    prev_newline = false;
                    next_chr!(chars, res);
                }
            }
            _ => {
                prev_newline = false;
                if module_section {
                    if let (f, true) = read_module(&mut chars)? {
                        if let Some(f) = f {
                            res.push(f);
                        }
                    } else {
                        module_section = false;
                    }
                } else {
                    next_chr!(chars, res);
                }
            }
        }
    }
}

fn read_macro<R>(chars: &mut CharReader<R>) -> Result<Option<IncFile>>
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
            Ok(Some(IncFile::System(res.into())))
        }
        '"' => {
            next_chr!(chars, None);
            let res = chars.esc_read_while(|c| c != '"')?;
            next_chr!(chars, None);
            Ok(Some(IncFile::User(res.into())))
        }
        _ => chars.esc_skip_while(|c| c != '\n').map(|_| None),
    }
}

fn read_char<R>(chars: &mut CharReader<R>) -> Result<()>
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

fn read_string<R>(chars: &mut CharReader<R>) -> Result<()>
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

fn read_multiline_comment<R>(chars: &mut CharReader<R>) -> Result<()>
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

fn read_line_comment<R>(chars: &mut CharReader<R>) -> Result<()>
where
    R: BufRead,
{
    chars.esc_skip_while(|c| c != '\n')
}

fn read_module<R>(chars: &mut CharReader<R>) -> Result<(Option<IncFile>, bool)> where R: BufRead {
    let kw = chars.read_while(char::is_alphabetic)?;
    match kw.as_str() {
        "module" => read_module_decl(chars),
        "export" => read_export_decl(chars),
        "import" => read_import_decl(chars),
        _ => Ok((None, false)),
    }
}

fn read_module_decl<R>(chars: &mut CharReader<R>) -> Result<(Option<IncFile>, bool)> where R: BufRead {
    chars.skip_while(char::is_whitespace)?;
    if chars.cur == ';' {
        next_chr!(chars, (None, true));
        return Ok((None, true));
    }

    let res = read_export_module_decl(chars)?;
    if let (Some(IncFile::ExpModule(m)), b) = res {
        Ok((Some(IncFile::ImpModule(m)), b))
    } else {
        Ok(res)
    }
}

fn read_export_decl<R>(chars: &mut CharReader<R>) -> Result<(Option<IncFile>, bool)> where R: BufRead {
    chars.skip_while(char::is_whitespace)?;
    let kw = chars.read_while(char::is_alphabetic)?;
    match kw.as_str() {
        "module" => read_export_module_decl(chars),
        "import" => {
            let (m, b) = read_import_decl(chars)?;
            let m = match m {
                Some(IncFile::ImpModule(m)) => Some(IncFile::ExpImpModule(m)),
                Some(IncFile::UserModule(m)) => Some(IncFile::ExpUserModule(m)),
                Some(IncFile::SystemModule(m)) => Some(IncFile::ExpSystemModule(m)),
                m => m,
            };
            Ok((m, b))
        }
        _ => Ok((None, false))
    }
}

fn read_export_module_decl<R>(chars: &mut CharReader<R>) -> Result<(Option<IncFile>, bool)> where R: BufRead {
    chars.skip_while(char::is_whitespace)?;
    let mut m = chars.read_while(|c| c.is_alphanumeric() || c == '.')?;

    chars.skip_while(char::is_whitespace)?;
    if chars.cur == ';' {
        let res = (Some(IncFile::ExpModule(m)), true);
        next_chr!(chars, res);
        return Ok(res);
    }
    if chars.cur != ':' {
        let res = (Some(IncFile::ExpModule(m)), true);
        return Ok(res);
    }
    m.push(':');

    chars.skip_while(char::is_whitespace)?;
    chars.read_to_while(|c| c.is_alphanumeric() || c == '.', &mut m)?;

    let res = (Some(IncFile::ExpModule(m)), true);
    if chars.cur == ';' {
        next_chr!(chars, res);
    }
    Ok(res)
}

fn read_import_decl<R>(chars: &mut CharReader<R>) -> Result<(Option<IncFile>, bool)> where R: BufRead {
    chars.skip_while(char::is_whitespace)?;
    match chars.cur {
        '<' => {
            let f = chars.read_while(|c| c != '>')?;
            return Ok((Some(IncFile::SystemModule(f.into())), true));
        }
        '"' => {
            let f = chars.read_while(|c| c != '"')?;
            return Ok((Some(IncFile::UserModule(f.into())), true));
        }
        _ => {
            let res = read_export_module_decl(chars)?;
            if let (Some(IncFile::ExpModule(m)), b) = res {
                Ok((Some(IncFile::ImpModule(m)), b))
            } else {
                Ok(res)
            }
        }
    }
}
