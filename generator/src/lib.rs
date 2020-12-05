
use std::fs::File;
use std::path::Path;
use std::io::Write;
use mmk_parser;
pub struct MmkGenerator
{
    filename: File,
    content: mmk_parser::Mmk,
}

pub trait Generator
{
    fn new(filename: &Path, mmk_content: mmk_parser::Mmk ) -> Self;
    fn generate_makefile(self: &mut Self)        -> std::io::Result<()>;
    fn generate_header(self: &mut Self)          -> std::io::Result<()>;
    fn generate_rule_executable(self: &mut Self) -> std::io::Result<()>;
    fn generate_rule_package(self: &mut Self)    -> std::io::Result<()>;
}

impl Generator for MmkGenerator
{
    fn new(directory: &Path, mmk_content: mmk_parser::Mmk) -> MmkGenerator
    {
        let file = File::create(directory.join("makefile")).expect("Something went wrong");
        MmkGenerator{ filename: file, content: mmk_content}
    }

    fn generate_makefile(self: &mut Self) -> std::io::Result<()>
    {
        self.generate_header()?;
        if self.content.data.contains_key("MMK_EXECUTABLE")
        {
            self.generate_rule_executable()?;
        }
        else
        {
            self.generate_rule_package()?;
        }
        Ok(())
    }

    fn generate_header(self: &mut Self) -> std::io::Result<()>
    {
        self.filename.write(b"\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include /home/fredrik/bin/mymake/include/strict.mk\n\
        \n\
        # ----- DEFINITIONS -----\n\
        AR       := $(AR.static) # We generate only static static libraries.\n\
        CC       := /usr/bin/gcc -x c++ # GCC is the default compiler.\n\
        CP       := /usr/bin/cp  \n\
        CP_FORCE := -f \n\
        # ----- DEFAULT PHONIES -----\n\
        \n\
        .SUFFIXES:         # We do not use suffixes on makefiles.\n\
        .PHONY: all\n\
        .PHONY: package\n\
        .PHONY: install\n\
        .PHONY: uninstall\n\
        .PHONY: clean\n")?;
        Ok(())
    }


    fn generate_rule_package(self: &mut Self) -> std::io::Result<()>
    {
        let data = format!("\n\
        #Generated by MmkGenerator.generate_rule(). \n\
        .PHONY: package\n\
        package: {sources} {headers}\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) -c $^)\n\
        ", sources = self.content.to_string("MMK_SOURCES")
         , headers = self.content.to_string("MMK_HEADERS"));
        
        self.filename.write(data.as_bytes())?;
        Ok(())
    }

    fn generate_rule_executable(self: &mut Self) -> std::io::Result<()>
    {
        let data = format!("\n\
        #Generated by MmkGenerator.generate_rule(). \n\
        .PHONY: {executable}\n\
        {executable}: {sources} {headers}\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) $^ -o $@)\n\
        ",
        executable = self.content.to_string("MMK_EXECUTABLE"),
        sources = self.content.to_string("MMK_SOURCES"),
        headers = self.content.to_string("MMK_HEADERS"));
        
        self.filename.write(data.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempdir::TempDir;

    #[test]
    fn test_generate_makefile() -> std::io::Result<()>
    {
        let dir = TempDir::new("example")?;
        let mut mmk = mmk_parser::Mmk::new();
        mmk.data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        mmk.data.insert("MMK_EXECUTABLE".to_string(), vec!["main".to_string()]);
        let mut gen: MmkGenerator = Generator::new(&dir.path(), mmk);
        assert!(Generator::generate_makefile(&mut gen).is_ok());
        Ok(())
    }
    #[test]
    fn test_generate_header() -> std::io::Result<()>
    {
        let dir = TempDir::new("example")?;
        let test_file = dir.path().join("makefile");
        let mut mmk = mmk_parser::Mmk::new();
        mmk.data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        mmk.data.insert("MMK_EXECUTABLE".to_string(), vec!["main".to_string()]);
        let mut gen: MmkGenerator = Generator::new(&dir.path(), mmk);
        assert!(Generator::generate_header(&mut gen).is_ok());
        assert_eq!("\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include /home/fredrik/bin/mymake/include/strict.mk\n\
        \n\
        # ----- DEFINITIONS -----\n\
        AR       := $(AR.static) # We generate only static static libraries.\n\
        CC       := /usr/bin/gcc -x c++ # GCC is the default compiler.\n\
        CP       := /usr/bin/cp  \n\
        CP_FORCE := -f \n\
        # ----- DEFAULT PHONIES -----\n\
        \n\
        .SUFFIXES:         # We do not use suffixes on makefiles.\n\
        .PHONY: all\n\
        .PHONY: package\n\
        .PHONY: install\n\
        .PHONY: uninstall\n\
        .PHONY: clean\n", fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }
    #[test]
    fn test_generate_package() -> std::io::Result<()>
    {
        let dir = TempDir::new("example")?;
        let mut mmk = mmk_parser::Mmk::new();
        let test_file = dir.path().join("makefile");
        mmk.data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        mmk.data.insert("MMK_HEADERS".to_string(), vec!["filename.h".to_string(), "ofilename.h".to_string()]);
        let mut gen: MmkGenerator = Generator::new(&dir.path(), mmk);
        assert!(Generator::generate_rule_package(&mut gen).is_ok());
        assert_eq!("\
        \n\
        #Generated by MmkGenerator.generate_rule(). \n\
        .PHONY: package\n\
        package: filename.cpp ofilename.cpp filename.h ofilename.h\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) -c $^)\n\
        ", fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }

    #[test]
    fn test_generate_executable() -> std::io::Result<()>
    {
        let dir = TempDir::new("example")?;
        let mut mmk = mmk_parser::Mmk::new();
        let test_file = dir.path().join("makefile");
        mmk.data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        mmk.data.insert("MMK_EXECUTABLE".to_string(), vec!["x".to_string()]);
        let mut gen: MmkGenerator = Generator::new(&dir.path(), mmk);
        assert!(Generator::generate_rule_executable(&mut gen).is_ok());
        assert_eq!("\n\
        #Generated by MmkGenerator.generate_rule(). \n\
        .PHONY: x\n\
        x: filename.cpp ofilename.cpp \n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) $^ -o $@)\n\
        ", fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }
}
