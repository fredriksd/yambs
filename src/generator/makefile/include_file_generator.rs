use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use indoc;

use crate::compiler::Linker;
use crate::compiler::StdLibCXX;
use crate::compiler::Type;
use crate::errors::FsError;
use crate::generator::{GeneratorError, Sanitizer, UtilityGenerator};
use crate::toolchain::Toolchain;
use crate::utility;

pub(crate) struct IncludeFileGenerator<'generator> {
    file: Option<File>,
    output_directory: std::path::PathBuf,
    args: HashMap<&'generator str, String>,
    toolchain: &'generator Toolchain,
}

impl<'generator> IncludeFileGenerator<'generator> {
    pub fn new(output_directory: &std::path::Path, toolchain: &'generator Toolchain) -> Self {
        utility::create_dir(output_directory).unwrap();

        IncludeFileGenerator {
            file: None,
            output_directory: output_directory.to_path_buf(),
            args: HashMap::new(),
            toolchain,
        }
    }

    fn create_mk_file(&mut self, filename_prefix: &str) {
        let mut filename = std::path::PathBuf::from(filename_prefix);
        filename.set_extension("mk");
        let file =
            utility::create_file(&self.output_directory.join(filename.to_str().unwrap())).unwrap();
        self.file = Some(file);
    }

    pub fn get_sanitizers(&self) -> String {
        let result = self.args.get("sanitizers");
        if result.is_some() {
            return format!("-fsanitize={}", result.unwrap());
        }
        String::new()
    }

    pub fn print_build_directory(&self) -> &str {
        self.output_directory.to_str().unwrap()
    }

    fn warnings_from_compiler_type(&self) -> Vec<&str> {
        let compiler = &self.toolchain.cxx_compiler;

        let mut warning_flags = vec![
            "-Wall",
            "-Wextra",
            "-Wshadow",
            "-Wnon-virtual-dtor",
            "-Wold-style-cast",
            "-Wcast-align",
            "-Wunused",
            "-Woverloaded-virtual",
            "-Wpedantic",
            "-Wconversion",
            "-Wsign-conversion",
            "-Wnull-dereference",
            "-Wdouble-promotion",
        ];

        match compiler.compiler_info.compiler_type {
            Type::Gcc => warning_flags.extend_from_slice(&[
                "-Wmisleading-indentation",
                "-Wduplicated-cond",
                "-Wduplicated-branches",
                "-Wlogical-op",
                "-Wuseless-cast",
            ]),
            Type::Clang => (),
        }
        warning_flags
    }

    fn select_stdlib_impl(&self) -> String {
        let stdlib = &self.toolchain.cxx_compiler.stdlib;
        match stdlib {
            StdLibCXX::LibStdCXX => "".to_string(),
            StdLibCXX::LibCXX => "-stdlib=libc++".to_string(),
        }
    }

    fn generate_linker_selection(&self) -> String {
        let compiler = &self.toolchain.cxx_compiler;
        let linker = &compiler.linker;

        let linker_statement = match linker {
            Linker::Gold => "LDFLAGS += -fuse-ld=gold".to_string(),
            Linker::Ld => "LDFLAGS += -fuse-ld=ld".to_string(),
            Linker::LLD => "LDFLAGS += -fuse-ld=lld".to_string(),
            _ => "LDFLAGS += ".to_string(),
        };
        linker_statement
    }

    fn generate_warnings_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("warnings");
        let data = indoc::formatdoc!("\
        #Generated by IncludeFileGenerator.generate_warnings_mk. DO NOT EDIT.

        include {def_directory}/defines.mk

        # Warning flags generated for compiler type {compiler_type}
        CXXFLAGS += \\
        {warnings}

        CXXFLAGS += {cpp_version}

        #-Wall                     # Reasonable and standard
        #-Wextra                   # Warn if indentation implies blocks where blocks do not exist.
        #-Wmisleading-indentation  # Warn if if / else chain has duplicated conditions
        #-Wduplicated-cond         # Warn if if / else branches has duplicated conditions
        #-Wduplicated-branches     # warn the user if a variable declaration shadows one from a parent context
        #-Wshadow                  # warn the user if a class with virtual functions has a non-virtual destructor. This helps
        #-Wnon-virtual-dtor        # catch hard to track down memory errors
        #-Wold-style-cast          # warn for C-style casts
        #-Wcast-align              # warn for potential performance problem casts
        #-Wunused                  # warn on anything being unused
        #-Woverloaded-virtual      # warn if you overload (not override) a virtual function
        #-Wpedantic                # warn if non-standard C++ is used
        #-Wconversion              # warn on type conversions that may lose data
        #-Wsign-conversion         # warn on sign conversions
        #-Wnull-dereference        # warn if a null dereference is detected
        #-Wdouble-promotion        # warn if float is implicit promoted to double
        #-Wformat=2                # warn on security issues around functions that format output (ie printf)
        ", 
        cpp_version = self.print_cpp_version(),
        def_directory = self.print_build_directory(),
        warnings = self.warnings_from_compiler_type().join("\\\n"),
        compiler_type = self.toolchain.cxx_compiler.compiler_info.compiler_type.to_string(),
        );
        self.file
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(std::path::PathBuf::from("warnings.mk"), e))?;
        Ok(())
    }

    fn generate_debug_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("debug");
        let data = indoc::formatdoc!(
            "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf

        {flags_sanitizer}

        # When building with sanitizer options, certain linker options must be added.
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.
        # This is done to support address space layout randomization (ASLR).
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization
        ",
            flags_sanitizer = self.generate_flags_sanitizer()
        );
        self.file
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(std::path::PathBuf::from("debug.mk"), e))?;
        Ok(())
    }

    fn generate_release_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("release");
        let data = indoc::indoc!(
            "\
        #Generated by IncludeFileGenerator.generate_release_mk. DO NOT EDIT.\n\
        CXXFLAGS += -O3\\
                    -DNDEBUG
        "
        )
        .to_string();
        self.file
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(std::path::PathBuf::from("release.mk"), e))?;
        Ok(())
    }

    fn generate_default_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("default_make");
        let data = indoc::indoc!(
            "\
        # Automatic dependency generation: Makes GCC generate the dependencies needed for a cpp file
        # excluding system header files.
        CPPFLAGS +=-MMD\\
                   -MP

        # Additional CXX flags to be passed to the compiler
        CXXFLAGS += -pthread\\
                    -fPIC # Generate Position Independent code suitable for use in a shared library.

        # Additional AR flags being passed to the static library linker
        ARFLAGS = rs
        "
        )
        .to_string();
        self.file
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(std::path::PathBuf::from("default.mk"), e))?;
        Ok(())
    }

    fn generate_defines_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("defines");

        let data = indoc::formatdoc!(
            "\
        # Defines.mk\n\
        # Contains a number of defines determined from YAMBS configuration time.\n\
        \n\
        {compiler_conditional_flags}\n\
        CP := /usr/bin/cp\n\
        CP_FORCE := -f

        # Select linker if any specified in the toolchain file
        {linker_selection}

        # Select stdlibc++ implementation based on toolchain file.
        # Will be empty if not specified.
        CXXFLAGS += {stdlib}
        \n\
        ",
            compiler_conditional_flags = self.generate_toolchain_defines(),
            linker_selection = self.generate_linker_selection(),
            stdlib = self.select_stdlib_impl(),
        );
        self.file
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(std::path::PathBuf::from("defines.mk"), e))?;
        Ok(())
    }

    fn generate_toolchain_defines(&self) -> String {
        let compiler_path = &self.toolchain.cxx_compiler.compiler_exe;
        let archiver_path = self.toolchain.archiver.path.clone();
        indoc::formatdoc!(
            "
        # Toolchain definitions\n
        CXX := {}
        AR := {}
        ",
            compiler_path.display(),
            archiver_path.display(),
        )
    }
}

impl<'generator> UtilityGenerator<'generator> for IncludeFileGenerator<'generator> {
    fn generate_build_files(&'generator mut self) -> Result<(), GeneratorError> {
        self.generate_warnings_mk()?;
        self.generate_debug_mk()?;
        self.generate_default_mk()?;
        self.generate_defines_mk()?;
        self.generate_release_mk()
    }

    fn add_cpp_version(&mut self, version: &str) {
        self.args.insert("C++", version.to_string().to_lowercase());
    }

    fn print_cpp_version(&'generator self) -> &str {
        if self.args.contains_key("C++") {
            match self.args.get("C++").unwrap().as_str() {
                "c++98" => "-std=c++98",
                "c++03" => "-std=c++03",
                "c++11" => "-std=c++11",
                "c++14" => "-std=c++14",
                "c++17" => "-std=c++17",
                "c++20" => "-std=c++20",
                _ => "-std=c++20",
            }
        } else {
            "-std=c++20"
        }
    }

    fn generate_flags_sanitizer(&self) -> String {
        if self.args.contains_key("sanitizers") {
            return format!(
                "\
            CXXFLAGS += {sanitizers}\n\
            \n\
            LDFLAGS += {sanitizers}",
                sanitizers = self.get_sanitizers()
            );
        }
        String::new()
    }
}

impl<'generator> Sanitizer for IncludeFileGenerator<'generator> {
    fn set_sanitizer(&mut self, sanitizer: &str) {
        let mut sanitizer_str = String::new();
        match sanitizer {
            "address" => sanitizer_str.push_str("address "), // sanitizer_str.push_str("address kernel-adress hwaddress pointer-compare pointer-subtract"),
            "thread" => sanitizer_str.push_str("thread -fPIE -pie "),
            "leak" => sanitizer_str.push_str("leak "),
            "undefined" => sanitizer_str.push_str("undefined "),
            _ => (),
        }
        self.args.insert("sanitizers", sanitizer_str);
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use crate::tests::EnvLock;
    use pretty_assertions::assert_eq;
    use tempdir::TempDir;

    use super::*;

    fn produce_include_path(base_dir: TempDir) -> std::path::PathBuf {
        let build_dir = std::path::PathBuf::from(".build");
        let output_directory = base_dir.path().join(build_dir).join("make_include");
        output_directory
    }

    fn construct_generator<'generator>(path: &std::path::Path) -> IncludeFileGenerator<'generator> {
        IncludeFileGenerator::new(path, crate::compiler::Compiler::new().unwrap())
    }

    #[test]
    fn add_cpp_version_cpp98_test() -> Result<(), GeneratorError> {
        let _lock = EnvLock::lock("CXX", "gcc");
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++98");
        assert_eq!(gen.args["C++"], "c++98");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp11_test() -> Result<(), GeneratorError> {
        let _lock = EnvLock::lock("CXX", "gcc");
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++11");
        assert_eq!(gen.args["C++"], "c++11");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp14_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++14");
        assert_eq!(gen.args["C++"], "c++14");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp17_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++17");
        assert_eq!(gen.args["C++"], "c++17");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp17_uppercase_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("C++17");
        assert_eq!(gen.args["C++"], "c++17");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp20_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++20");
        assert_eq!(gen.args["C++"], "c++20");
        Ok(())
    }

    #[test]
    fn generate_warnings_mk_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("warnings.mk");
        gen.generate_warnings_mk().unwrap();
        assert_eq!(format!(indoc::indoc!("\
        #Generated by IncludeFileGenerator.generate_warnings_mk. DO NOT EDIT.

        include {def_directory}/defines.mk


        GLINUX_WARNINGS := -Wall \\
                          -Wextra \\
                          -Wshadow \\
                          -Wnon-virtual-dtor \\
                          -Wold-style-cast \\
                          -Wcast-align \\
                          -Wunused \\
                          -Woverloaded-virtual \\
                          -Wpedantic \\
                          -Wconversion \\
                          -Wsign-conversion \\
                          -Wnull-dereference \\
                          -Wdouble-promotion \\
                          -Wformat=2


        ifeq ($(CXX_USES_GCC), true)
            CXXFLAGS += $(GLINUX_WARNINGS) \\
                        -Wmisleading-indentation \\
                        -Wduplicated-cond \\
                        -Wduplicated-branches \\
                        -Wlogical-op \\
                        -Wuseless-cast


       else ifeq ($(CXX_USES_CLANG), true)
            CXXFLAGS += $(GLINUX_WARNINGS)
       endif

       CXXFLAGS += -std=c++20

        #-Wall                     # Reasonable and standard
        #-Wextra                   # Warn if indentation implies blocks where blocks do not exist.
        #-Wmisleading-indentation  # Warn if if / else chain has duplicated conditions
        #-Wduplicated-cond         # Warn if if / else branches has duplicated conditions
        #-Wduplicated-branches     # warn the user if a variable declaration shadows one from a parent context
        #-Wshadow                  # warn the user if a class with virtual functions has a non-virtual destructor. This helps
        #-Wnon-virtual-dtor        # catch hard to track down memory errors
        #-Wold-style-cast          # warn for C-style casts
        #-Wcast-align              # warn for potential performance problem casts
        #-Wunused                  # warn on anything being unused
        #-Woverloaded-virtual      # warn if you overload (not override) a virtual function
        #-Wpedantic                # warn if non-standard C++ is used
        #-Wconversion              # warn on type conversions that may lose data
        #-Wsign-conversion         # warn on sign conversions
        #-Wnull-dereference        # warn if a null dereference is detected
        #-Wdouble-promotion        # warn if float is implicit promoted to double
        #-Wformat=2                # warn on security issues around functions that format output (ie printf)
        "),
        def_directory = gen.print_build_directory()), fs::read_to_string(file_name.to_str().unwrap()).unwrap());
        Ok(())
    }

    #[test]
    fn generate_debug_mk_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("debug.mk");
        gen.generate_debug_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf
        
        \n
        # When building with sanitizer options, certain linker options must be added.
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.
        # This is done to support address space layout randomization (ASLR).
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization
        "
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn generate_debug_mk_with_address_sanitizer_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("debug.mk");
        gen.set_sanitizer("address");
        gen.generate_debug_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf

        CXXFLAGS += -fsanitize=address 

        LDFLAGS += -fsanitize=address 

        # When building with sanitizer options, certain linker options must be added.
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.
        # This is done to support address space layout randomization (ASLR).
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization
        "
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn generate_debug_mk_with_thread_sanitizer_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("debug.mk");
        gen.set_sanitizer("thread");
        gen.generate_debug_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf

        CXXFLAGS += -fsanitize=thread -fPIE -pie 

        LDFLAGS += -fsanitize=thread -fPIE -pie 

        # When building with sanitizer options, certain linker options must be added.
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.
        # This is done to support address space layout randomization (ASLR).
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization
        "
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn generate_release_mk_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("release.mk");
        gen.generate_release_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
        #Generated by IncludeFileGenerator.generate_release_mk. DO NOT EDIT.\n\
        CXXFLAGS += -O3\\
                    -DNDEBUG
        "
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn generate_default_mk_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("default_make.mk");
        gen.generate_default_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
        # Automatic dependency generation: Makes GCC generate the dependencies needed for a cpp file
        # excluding system header files.
        CPPFLAGS +=-MMD\\
                   -MP
       
        # Additional CXX flags to be passed to the compiler
        CXXFLAGS += -pthread\\
                    -fPIC # Generate Position Independent code suitable for use in a shared library.

        # Additional AR flags being passed to the static library linker
        ARFLAGS = rs\n"
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn generate_flags_sanitizer_no_sanitizers_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let gen = construct_generator(&output_directory);
        let actual = gen.generate_flags_sanitizer();
        let expected = String::new();
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn generate_flags_sanitizer_address_sanitizer_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());

        let mut gen = construct_generator(&output_directory);
        gen.set_sanitizer("address");
        let actual = gen.generate_flags_sanitizer();
        let expected = indoc::indoc!(
            "\
            CXXFLAGS += -fsanitize=address 

            LDFLAGS += -fsanitize=address ",
        );
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn generate_flags_sanitizer_thread_sanitizer_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut gen = construct_generator(&output_directory);
        gen.set_sanitizer("thread");
        let actual = gen.generate_flags_sanitizer();
        let expected = indoc::indoc!(
            "\
            CXXFLAGS += -fsanitize=thread -fPIE -pie 

            LDFLAGS += -fsanitize=thread -fPIE -pie ",
        );
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn generate_defines_mk_test() -> std::io::Result<()> {
        let _lock = EnvLock::lock("CXX", "gcc");
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("defines.mk");
        gen.generate_defines_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
    # Defines.mk\n\
    # Contains a number of defines determined from YAMBS configuration time.\n\
    \n\
    CXX_USES_GCC := true\n\
    CXX_USES_CLANG := false\n\
    \n\
    CP := /usr/bin/cp\n\
    CP_FORCE := -f\n\
    \n"
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }
}
