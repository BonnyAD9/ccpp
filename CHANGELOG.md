# CHANGELOG

## current
- Add the `new` command to create new projects
- Colorful CLI

## v0.1.0
- build C projects
- set build configuration in `ccpp.toml` (project name, compiler, compiler flags, linker, linker flags, different configuration for debug and release)
- properly determine dependencies (based on includes) and build only files that need to be rebuilt
- build and run with a single command
- choose between debug/release mode
- clean the build files
