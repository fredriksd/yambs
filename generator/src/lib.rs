
use std::fs::File;
use std::io::Write;

use dependency::Dependency;
use error::MyMakeError;

#[allow(dead_code)]
pub struct MmkGenerator
{
    filename: Option<File>,
    dependency: Dependency,
    output_directory: std::path::PathBuf,
    debug: bool,
}

fn create_dir(dir: &std::path::PathBuf) -> Result<(), MyMakeError> {
    if !dir.is_dir() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}


fn create_file(dir: &std::path::PathBuf, filename: &str) -> Result<File, MyMakeError> {
    let file = dir.join(filename);
    if file.is_file() {
        match std::fs::remove_file(&file) {
            Ok(()) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error removing {:?}: {}", file, err))),
        };
    }
    let filename = File::create(&file)?;
    Ok(filename)
}


fn print_full_path(os: &mut String, dir: &str, filename: &str, no_newline: bool) {
    os.push_str(dir);
    os.push_str("/");
    os.push_str(filename);
    if !no_newline {
        os.push_str(" \\\n");
    }
}

impl MmkGenerator {
    pub fn new(dependency: &Dependency, build_directory: &std::path::PathBuf) -> Result<MmkGenerator, MyMakeError> {
        let output_directory = dependency.path().parent().unwrap().join(&build_directory);
        create_dir(&output_directory)?;
        
        Ok(MmkGenerator{ filename: None, dependency: dependency.clone(), output_directory, debug: false})
    }


    pub fn replace_generator(&mut self, dependency: &Dependency, build_directory: &std::path::PathBuf) {
        let gen = MmkGenerator::new(dependency, build_directory).unwrap();
        self.dependency       = gen.dependency;
        self.output_directory = gen.output_directory;
        self.create_makefile();
    }


    pub fn create_makefile(&mut self) {
        let filename = create_file(&self.output_directory, "makefile").unwrap();
        self.filename = Some(filename);
    }


    fn use_subdir(&mut self, dir: std::path::PathBuf) -> Result<(), MyMakeError>{
        let new_output_dir = self.output_directory.join(dir);
        create_dir(&new_output_dir)?;
        self.output_directory = new_output_dir;
        Ok(())
    }


    fn create_subdir(&self, dir: std::path::PathBuf) -> Result<(), MyMakeError> {
        create_dir(&self.output_directory.join(dir))
    }

    #[allow(dead_code)]
    fn pop_dir(&mut self) {
        self.output_directory.pop();
    }


    pub fn make_object_rule(&self, mmk_data: &mmk_parser::Mmk) -> String {
        let mut formatted_string = String::new();
        let parent_path = &self.dependency.path().parent().unwrap().to_str().unwrap();
        let mut object = String::new();

        if mmk_data.data.contains_key("MMK_SOURCES") {
            for source in &mmk_data.data["MMK_SOURCES"] {
                if let Some(source_path) = mmk_data.source_file_path(source) {
                    self.create_subdir(source_path).unwrap();
                }

                if source.ends_with(".cpp") {
                    object = source.replace(".cpp", ".o");
                }
                if source.ends_with(".cc") {
                    object = source.replace(".cc", ".o");
                }

                formatted_string.push_str(self.output_directory.to_str().unwrap());
                formatted_string.push_str("/");
                formatted_string.push_str(&object);
                formatted_string.push_str(": \\\n");
                formatted_string.push_str("\t");
                formatted_string.push_str(parent_path);
                formatted_string.push_str("/");
                formatted_string.push_str(source);
                formatted_string.push_str("\n");
                formatted_string.push_str(&format!("\t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) \
                                                          $(WARNINGS) {dependencies} -I{path_str} $< -c -o $@)\n\n"
                , dependencies = self.print_dependencies()
                , path_str = parent_path));
            }
        }
        formatted_string.trim_end().to_string()
    }


    fn print_header_includes(&self) -> String {
        let mut formatted_string = String::new();
        let mmk_data = &self.dependency.mmk_data();
        let mut include_file = String::new();
        if mmk_data.data.contains_key("MMK_SOURCES") {
            for source in &mmk_data.data["MMK_SOURCES"] {
                if source.ends_with(".cpp") {
                    include_file = source.replace(".cpp", ".d");
                }
                if source.ends_with(".cc") {
                    include_file = source.replace(".cc", ".d");
                }
                
                formatted_string.push_str("sinclude ");
                formatted_string.push_str(self.output_directory.to_str().unwrap());
                formatted_string.push_str("/");
                formatted_string.push_str(&include_file);
                formatted_string.push_str("\n");
            }
        }
        formatted_string
    }


    pub fn print_required_dependencies_libraries(self: &Self) -> String {
        let mut formatted_string = String::new();
        for dependency in  self.dependency.requires().borrow().iter() {
            if dependency.borrow().library_name() != "" {
                let required_dep = dependency.borrow();
                let mut output_directory = required_dep.get_build_directory().clone();
                if self.debug {
                    output_directory = output_directory.join("debug");
                }
                else {
                    output_directory = output_directory.join("release");
                }
                formatted_string.push_str("\t");
                print_full_path(&mut formatted_string, 
                                output_directory.to_str().unwrap(),
                                &required_dep.library_name(),
                                false);
            }
        }
        formatted_string
    }


    pub fn print_mandatory_libraries(self: &Self) -> String {
        let mut formatted_string = String::new();
        formatted_string.push_str("-lstdc++");
        formatted_string
    }


    fn print_library_name(&self) -> String {
        let mut formatted_string = String::new();
        print_full_path(&mut formatted_string,
                        self.output_directory.to_str().unwrap(),
                        &self.dependency.library_name(),
                        true);
        formatted_string
    }


    fn print_prerequisites(self: &Self) -> String {
        let mut formatted_string = String::new();
        let mut object = String::new();
        if self.dependency.mmk_data().data.contains_key("MMK_SOURCES") {
            formatted_string.push_str("\\\n");
            for source in &self.dependency.mmk_data().data["MMK_SOURCES"] {
                if source.ends_with(".cpp") {
                    object = source.replace(".cpp", ".o");
                }
                if source.ends_with(".cc") {
                    object = source.replace(".cc", ".o");
                }
                formatted_string.push_str("\t");
                print_full_path(&mut formatted_string,
                                self.output_directory.to_str().unwrap(),
                                &object,
                                false);
            }
        }
        formatted_string.push_str(&self.print_required_dependencies_libraries());
        formatted_string.push_str("\t");
        formatted_string.push_str(&self.print_mandatory_libraries());
        formatted_string
    }


    fn print_dependencies(&self) -> String {
        let mut formatted_string = self.dependency.mmk_data().to_string("MMK_DEPEND");
        formatted_string.push_str(&self.dependency.mmk_data().to_string("MMK_SYS_INCLUDE"));
        formatted_string
    }


    pub fn generate_makefiles(&mut self, dependency: &mut Dependency) -> Result<(), MyMakeError> {
        if !&dependency.is_makefile_made()
        {
            &dependency.makefile_made();            
            self.generate_makefile()?;
        }
        for required_dependency in dependency.requires().borrow().iter()
        {
            if !required_dependency.borrow().is_makefile_made()
            {
                required_dependency.borrow_mut().makefile_made();
                let mut build_directory = std::path::PathBuf::from(".build");
                if self.debug {
                    build_directory.push("debug");
                }
                else {
                    build_directory.push("release");
                }
                self.replace_generator(&required_dependency.borrow(),
                                                 &build_directory);
                self.generate_makefile()?;
            }
            self.generate_makefiles(&mut required_dependency.borrow_mut())?;
        }
        Ok(())
    }


    pub fn debug(&mut self) {
        self.debug = true;
        self.use_subdir(std::path::PathBuf::from("debug")).unwrap();
    }


    pub fn release(&mut self) {
        if !self.debug {
            self.use_subdir(std::path::PathBuf::from("release")).unwrap();
        }
    }


    fn print_release(&self) -> &str {
        "include /home/fredrik/bin/mymake/include/release.mk\n"
    }


    fn print_debug(&self) -> &str {
        if self.debug {
            "include /home/fredrik/bin/mymake/include/debug.mk\n"
        }
        else {
            self.print_release()
        }
    }
}

pub trait Generator
{
    fn generate_makefile(self: &mut Self)        -> Result<(), MyMakeError>;
    fn generate_header(self: &mut Self)          -> Result<(), MyMakeError>;
    fn generate_rule_executable(self: &mut Self) -> Result<(), MyMakeError>;
    fn generate_rule_package(self: &mut Self)    -> Result<(), MyMakeError>;
    fn generate_appending_flags(&mut self)       -> Result<(), MyMakeError>;
    fn print_ok(self: &Self);    
}

impl Generator for MmkGenerator
{
    fn generate_makefile(self: &mut Self) -> Result<(), MyMakeError> {
        self.create_makefile();
        self.generate_header()?;
        self.generate_appending_flags()?;
        if self.dependency.mmk_data().data.contains_key("MMK_EXECUTABLE") && 
           self.dependency.mmk_data().data["MMK_EXECUTABLE"] != {[""]}
        {
            self.generate_rule_executable()?;
        }
        else
        {
            self.generate_rule_package()?;
        }
        self.print_ok();
        Ok(())
    }


    fn generate_header(self: &mut Self) -> Result<(), MyMakeError> {
        let data = format!("\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include /home/fredrik/bin/mymake/include/strict.mk\n\
        include /home/fredrik/bin/mymake/include/default_make.mk\n\
        {debug}\
        \n\
        # ----- DEFINITIONS -----\n\
        CC       := /usr/bin/gcc        # GCC is the default compiler.\n\
        CP       := /usr/bin/cp  \n\
        CP_FORCE := -f \n\
        # ----- DEFAULT PHONIES -----\n\
        \n\
        .SUFFIXES:         # We do not use suffixes on makefiles.\n\
        .PHONY: all\n\
        .PHONY: package\n\
        .PHONY: install\n\
        .PHONY: uninstall\n\
        .PHONY: clean\n", 
        debug = self.print_debug());
        
        match self.filename.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error creating header for {:?}: {}", self.filename, err))),
        };
        Ok(())
    }


    fn generate_rule_package(self: &mut Self) -> Result<(), MyMakeError> {
        let data = format!("\n\
        #Generated by MmkGenerator.generate_rule_package(). \n\
        \n\
        {package}: {prerequisites}\n\
        \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\
        \n\
        {sources_to_objects}\n\
        \n\
        {include_headers}\n\
        ", prerequisites = self.print_prerequisites()
         , package      = self.print_library_name()
         , sources_to_objects = self.make_object_rule(&self.dependency.mmk_data())
         , include_headers = self.print_header_includes());
        
        match self.filename.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error creating package rule for {:?}: {}", self.filename, err))),
        };
        Ok(())
    }


    fn generate_rule_executable(self: &mut Self) -> Result<(), MyMakeError> {
        let data = format!("\n\
        #Generated by MmkGenerator.generate_rule_executable(). \n\
        \n\
        .PHONY: {executable}\n\
        {executable}: {prerequisites}\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) {dependencies} $^ -o $@)\n\
        \n\
        {sources_to_objects}\n\
        \n\
        {include_headers}\n\
        ",
        executable         = self.dependency.mmk_data().to_string("MMK_EXECUTABLE"),
        prerequisites      = self.print_prerequisites(),
        dependencies       = self.print_dependencies(),
        sources_to_objects = self.make_object_rule(&self.dependency.mmk_data()),
        include_headers = self.print_header_includes());
        
        match self.filename.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error creating executable rule for {:?}: {}", self.filename, err))),
        };
        Ok(())
    }


    fn generate_appending_flags(&mut self) -> Result<(), MyMakeError> {
        let mut data = String::new();

        if self.dependency.mmk_data().data.contains_key("MMK_CXXFLAGS_APPEND") {
            data.push_str(&format!("CXXFLAGS += {cxxflags}\n", 
            cxxflags = self.dependency.mmk_data().to_string("MMK_CXXFLAGS_APPEND")).to_owned());
        }

        if self.dependency.mmk_data().data.contains_key("MMK_CPPFLAGS_APPEND") {
            data.push_str(&format!("CPPFLAGS += {cppflags}\n", 
            cppflags = self.dependency.mmk_data().to_string("MMK_CPPFLAGS_APPEND")).to_owned());
        }

        if !data.is_empty() {
            match self.filename.as_ref().unwrap().write(data.as_bytes()) {
                Ok(_) => (),
                Err(err) => return Err(MyMakeError::from(format!("Error creating executable rule for {:?}: {}", self.filename, err))),
            };
        }
        Ok(())
    }


    fn print_ok(self: &Self) -> () {
        print!(".");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempdir::TempDir;
    use pretty_assertions::assert_eq;

    #[test]
    fn generate_makefile_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data_mut().data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.mmk_data_mut().data.insert("MMK_EXECUTABLE".to_string(), vec!["main".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, &output_dir).unwrap();
        assert!(Generator::generate_makefile(&mut gen).is_ok());
        Ok(())
    }


    #[test]
    fn generate_header_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        let mut gen = MmkGenerator::new(&dependency, &output_dir).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_header(&mut gen).is_ok());
        assert_eq!("\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include /home/fredrik/bin/mymake/include/strict.mk\n\
        include /home/fredrik/bin/mymake/include/default_make.mk\n\
        include /home/fredrik/bin/mymake/include/release.mk\n\
        \n\
        # ----- DEFINITIONS -----\n\
        CC       := /usr/bin/gcc        # GCC is the default compiler.\n\
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
    fn generate_header_test_with_debug() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        let mut gen = MmkGenerator::new(&dependency, &output_dir).unwrap();        
        gen.debug();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_header(&mut gen).is_ok());
        assert_eq!("\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include /home/fredrik/bin/mymake/include/strict.mk\n\
        include /home/fredrik/bin/mymake/include/default_make.mk\n\
        include /home/fredrik/bin/mymake/include/debug.mk\n\
        \n\
        # ----- DEFINITIONS -----\n\
        CC       := /usr/bin/gcc        # GCC is the default compiler.\n\
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
    fn generate_package_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data_mut().data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.add_library_name();
        dependency.mmk_data_mut().data.insert("MMK_DEPEND".to_string(), vec!["/some/dependency".to_string(), "/some/new/dependency".to_string()]);

        let mut gen = MmkGenerator::new(&dependency, &output_dir).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_rule_package(&mut gen).is_ok());
        assert_eq!(format!("\n\
        #Generated by MmkGenerator.generate_rule_package(). \n\
        \n\
        {directory}/.build/libtmp.a: \\\n\
        \t{directory}/.build/filename.o \\\n\
        \t{directory}/.build/ofilename.o \\\n\
        \t-lstdc++\n\
        \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\
        \n\
        {directory}/.build/filename.o: \\\n\
        \t{directory}/filename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency -I/some/new/dependency -I{directory} $< -c -o $@)\n\
        \n\
        {directory}/.build/ofilename.o: \\\n\
        \t{directory}/ofilename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency -I/some/new/dependency -I{directory} $< -c -o $@)\n\
        \n\
        sinclude {directory}/.build/filename.d\n\
        sinclude {directory}/.build/ofilename.d\n\
        \n", 
        directory = dir.path().to_str().unwrap()), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_executable_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data_mut().data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        dependency.mmk_data_mut().data.insert("MMK_EXECUTABLE".to_string(), vec!["x".to_string()]);
        dependency.mmk_data_mut().data.insert("MMK_DEPEND".to_string(), vec!["/some/dependency".to_string(), "/some/new/dependency".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, &output_dir).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_rule_executable(&mut gen).is_ok());
        assert_eq!(format!("\n\
        #Generated by MmkGenerator.generate_rule_executable(). \n\
        \n\
        .PHONY: x\n\
        x: \\\n\
        \t{directory}/.build/filename.o \\\n\
        \t{directory}/.build/ofilename.o \\\n\
        \t-lstdc++\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency -I/some/new/dependency $^ -o $@)\n\
        \n\
        {directory}/.build/filename.o: \\\n\
        \t{directory}/filename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency -I/some/new/dependency -I{directory} $< -c -o $@)\n\
        \n\
        {directory}/.build/ofilename.o: \\\n\
        \t{directory}/ofilename.cpp\n\
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I/some/dependency -I/some/new/dependency -I{directory} $< -c -o $@)\n\
        \n\
        sinclude {directory}/.build/filename.d\n\
        sinclude {directory}/.build/ofilename.d\n\
        \n",
        directory = dir.path().to_str().unwrap()), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_appending_flags_test_cxxflags() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data_mut().data.insert("MMK_CXXFLAGS_APPEND".to_string(), vec!["-pthread".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, &output_dir).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_appending_flags(&mut gen).is_ok());
        assert_eq!(format!("\
        CXXFLAGS += -pthread\n\
        "), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_appending_flags_test_cppflags() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data_mut().data.insert("MMK_CPPFLAGS_APPEND".to_string(), vec!["-somesetting".to_string()]);
        let mut gen = MmkGenerator::new(&dependency, &output_dir).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_appending_flags(&mut gen).is_ok());
        assert_eq!(format!("\
        CPPFLAGS += -somesetting\n\
        "), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn generate_appending_flags_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data_mut().data.insert("MMK_CXXFLAGS_APPEND".to_string(), vec!["-pthread".to_string()]);
        dependency.mmk_data_mut().data.insert("MMK_CPPFLAGS_APPEND".to_string(), vec!["-somesetting".to_string()]);

        let mut gen = MmkGenerator::new(&dependency, &output_dir).unwrap();
        gen.create_makefile();
        let test_file = gen.output_directory.join("makefile");
        assert!(Generator::generate_appending_flags(&mut gen).is_ok());
        assert_eq!(format!("\
        CXXFLAGS += -pthread\n\
        CPPFLAGS += -somesetting\n\
        "), fs::read_to_string(test_file.to_str().unwrap()).unwrap());
        Ok(())
    }


    #[test]
    fn print_header_includes_test() -> std::io::Result<()> {
        let dir = TempDir::new("example")?;
        let output_dir = std::path::PathBuf::from(".build");
        let mut dependency = Dependency::from(&dir.path().join("mymakeinfo.mmk"));
        dependency.mmk_data_mut().data.insert("MMK_SOURCES".to_string(), vec!["filename.cpp".to_string(), "ofilename.cpp".to_string()]);
        let gen = MmkGenerator::new(&dependency, &output_dir).unwrap();
        let actual = gen.print_header_includes();
        let expected = format!("sinclude {directory}/.build/filename.d\n\
                                       sinclude {directory}/.build/ofilename.d\n",
                                       directory = dir.path().to_str().unwrap());
        assert_eq!(actual, expected);
        Ok(())
    }
}
