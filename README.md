# ccpp

A simple to use build tool for C/C++ projects. The goal is to make building of
C/C++ projects as simple as possible. With ccpp you no longer have to worry
about adding new source files or subdirectories in your source files, all is
handled automatically.

Ofcourse `ccpp` is not for everyone, with this simlicity of use you loose the
flexibility of other build systems, but this is great tool for most simple
projects.

Currently only C projects are supported.

## Usage
Your project must have the folowing structure:
- `src/` directory with your source files
    - c source files and headers, only files with `.c` extension are compiled
    - there can also be any levels of subfolders with source files
- `ccpp.toml` configuration for ccpp

ccpp generates binaries and object files in folder `bin`.

### ccpp.toml
Only the name of the project is required, all other fields are optional and
they will have their default values if they are not present.
```toml
[project]
name = "my-app" # name of the project

[build]
# general build information for both build types
target = "my-app-bin" # name of the compiled binary, name of the project is
                      # used when it is not present
cc = "gcc" # name of the C compiler to use, if not present value of the CC
           # environment variable is used, when it is not set "cc" is used
ld = "gcc" # name of the linker to use, if not present value of the LD
           # enviromment variable is used, when it is not set "ld" is used
cflags = [] # flags for the compiler when compiling, this is empty by default
ldflags = [] # flags for the linker, this is empty by default

[debug_build]
# configuration for debug builds
target = "my-app-debug" # when set overwrites the value from [build]
cc = "gcc" # when set overwrites the value form [build]
ld = "gcc" # when set overwrites the value form [build]
cflags = [] # when set it is appended to the flags from [build]
ldflags = [] # when set it is appended to the flags from [build]

[release_build]
# configuration for release builds
target = "my-app-debug" # when set overwrites the value from [build]
cc = "gcc" # when set overwrites the value form [build]
ld = "gcc" # when set overwrites the value form [build]
cflags = [] # when set it is appended to the flags from [build]
ldflags = [] # when set it is appended to the flags from [build]
```

### CLI
- `ccpp build` build the project
- `ccpp run` build and run the project

See `ccpp help` for more information.
