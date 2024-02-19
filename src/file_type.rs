use std::ffi::OsStr;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Language {
    C,
    Cpp,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FileState {
    Source,
    SourceModule,
    Header,
    Precompiled,
    Object,
    Executable,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FileType {
    pub lang: Language,
    pub state: FileState,
}

impl FileType {
    pub fn from_ext(ext: &OsStr) -> Option<FileType> {
        if ext == "c" {
            Some(Self {
                lang: Language::C,
                state: FileState::Source,
            })
        } else if ext == "C"
            || ext == "cc"
            || ext == "cpp"
            || ext == "CPP"
            || ext == "c++"
            || ext == "cp"
            || ext == "cxx"
        {
            Some(Self {
                lang: Language::Cpp,
                state: FileState::Source,
            })
        } else if ext == "h" {
            Some(Self {
                lang: Language::C,
                state: FileState::Header,
            })
        } else if ext == "H"
            || ext == "hh"
            || ext == "hpp"
            || ext == "hxx"
            || ext == "h++"
        {
            Some(Self {
                lang: Language::Cpp,
                state: FileState::Header,
            })
        } else {
            None
        }
    }
}
