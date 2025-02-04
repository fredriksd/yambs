use crate::manifest;

pub mod preprocessor;
pub mod types;

use crate::YAMBS_BUILD_DIR_VAR;
use crate::YAMBS_BUILD_TYPE;
use crate::YAMBS_MANIFEST_DIR;
use preprocessor::{Preprocessor, PreprocessorError, Variable};

// FIXME: Write tests!
pub fn parse(manifest_path: &std::path::Path) -> Result<manifest::ParsedManifest, ParseTomlError> {
    unsafe {
        let toml_content =
            String::from_utf8(std::fs::read(manifest_path).map_err(ParseTomlError::FailedToRead)?)
                .map_err(ParseTomlError::FailedToConvertUtf8)?;
        let mut preprocessor = Preprocessor::new()
            .with_var(Variable {
                key: "YAMBS_BUILD_DIR".to_string(),
                value: YAMBS_BUILD_DIR_VAR
                    .get_unchecked()
                    .as_path()
                    .display()
                    .to_string(),
            })
            .with_var(Variable {
                key: "YAMBS_MANIFEST_DIR".to_string(),
                value: YAMBS_MANIFEST_DIR
                    .get_unchecked()
                    .as_path()
                    .display()
                    .to_string(),
            })
            .with_var(Variable {
                key: "YAMBS_BUILD_TYPE".to_string(),
                value: YAMBS_BUILD_TYPE.get_unchecked().to_string(),
            });

        let manifest_parsed = preprocessor
            .parse(&toml_content)
            .map_err(ParseTomlError::Preprocessor)?;
        let metadata =
            std::fs::metadata(manifest_path).expect("Could not fetch metadata from yambs.json");
        let manifest_directory = manifest_path.parent().unwrap();
        Ok(manifest::ParsedManifest {
            manifest: manifest::Manifest {
                directory: manifest_directory.to_path_buf(),
                modification_time: metadata
                    .modified()
                    .expect("Could not fetch last modified time of manifest"),
            },
            data: parse_toml(&manifest_parsed, manifest_directory)?,
        })
    }
}

fn parse_toml(
    toml: &str,
    manifest_dir: &std::path::Path,
) -> Result<manifest::ManifestData, ParseTomlError> {
    let manifest_contents =
        toml::from_str::<types::RawManifestData>(toml).map_err(ParseTomlError::FailedToParse)?;
    manifest::ManifestData::from_raw(manifest_contents, manifest_dir)
        .map_err(ParseTomlError::FailedToCreateManifestData)
}

#[derive(thiserror::Error, Debug)]
pub enum ParseTomlError {
    #[error("Failed to parse TOML manifest file.")]
    FailedToParse(#[source] toml::de::Error),
    #[error("Failed to read TOML manifest file.")]
    FailedToRead(#[source] std::io::Error),
    #[error("Failed to convert UTF-8 bytes to string")]
    FailedToConvertUtf8(#[source] std::string::FromUtf8Error),
    #[error("Failed to create manifest data")]
    FailedToCreateManifestData(#[source] manifest::ParseManifestError),
    #[error("Preprocessor failed")]
    Preprocessor(#[source] PreprocessorError),
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::flags::CompilerFlags;
    use crate::manifest::ManifestData;
    use crate::targets::{Dependency, Executable, Library, Target};
    use types::{Define, DependencyData, IncludeSearchType, LibraryType, SourceData};

    struct TestFixture {
        pub tempdir: tempdir::TempDir,
    }

    impl TestFixture {
        pub fn new() -> Self {
            Self {
                tempdir: tempdir::TempDir::new("parse").unwrap(),
            }
        }

        pub fn create_dummy_file(&self, path_postfix: &std::path::Path) -> std::path::PathBuf {
            let path = self.tempdir.path().join(path_postfix);
            std::fs::File::create(&path).unwrap();
            path
        }
    }

    #[test]
    fn parse_produces_manifest_with_executable() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.tempdir.path().to_path_buf();

        fixture.create_dummy_file(&std::path::PathBuf::from("main.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("x.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("y.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("z.cpp"));

        let input = r#"
    [executable.x]
    sources = ['x.cpp', 'y.cpp', 'z.cpp', 'main.cpp']
    "#;
        {
            let manifest = parse_toml(input, &manifest_dir).unwrap();
            let executable = Executable {
                name: "x".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                dependencies: Vec::new(),
                defines: Vec::new(),
                compiler_flags: CompilerFlags::new(),
            };
            let expected = ManifestData {
                project_config: None,
                targets: vec![Target::Executable(executable)],
            };
            assert_eq!(manifest, expected);
        }
    }

    #[test]
    fn parse_produces_manifest_with_executable_with_custom_cxxflags() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.tempdir.path().to_path_buf();

        fixture.create_dummy_file(&std::path::PathBuf::from("main.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("x.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("y.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("z.cpp"));

        let input = r#"
    [executable.x]
    sources = ['x.cpp', 'y.cpp', 'z.cpp', 'main.cpp']
    cxxflags_append = ["-g", "-O2"]
    "#;
        {
            let manifest = parse_toml(input, &manifest_dir).unwrap();
            let executable = Executable {
                name: "x".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                dependencies: Vec::new(),
                defines: Vec::new(),
                compiler_flags: crate::flags::CompilerFlags {
                    c_flags: None,
                    cxx_flags: Some(crate::flags::CXXFlags::from_slice(&[
                        "-g".to_string(),
                        "-O2".to_string(),
                    ])),
                    cpp_flags: None,
                    ld_flags: None,
                    include_directories: vec![],
                    system_include_directories: vec![],
                },
            };
            let expected = ManifestData {
                project_config: None,
                targets: vec![Target::Executable(executable)],
            };
            assert_eq!(manifest, expected);
        }
    }

    #[test]
    fn parse_produces_manifest_with_multiple_executables() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.tempdir.path().to_path_buf();

        fixture.create_dummy_file(&std::path::PathBuf::from("main.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("x.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("y.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("z.cpp"));

        let input = r#"
    [executable.x]
    sources = ['x.cpp', 'y.cpp', 'z.cpp', 'main.cpp']

    [executable.y]
    sources = ['x.cpp', 'y.cpp', 'z.cpp', 'main.cpp']
    "#;
        {
            let manifest = parse_toml(input, &manifest_dir).unwrap();
            let executable_x = Executable {
                name: "x".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                dependencies: Vec::new(),
                defines: Vec::new(),
                compiler_flags: CompilerFlags::new(),
            };
            let executable_y = Executable {
                name: "y".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                dependencies: Vec::new(),
                defines: Vec::new(),
                compiler_flags: CompilerFlags::new(),
            };
            let expected = ManifestData {
                project_config: None,
                targets: vec![
                    Target::Executable(executable_x),
                    Target::Executable(executable_y),
                ],
            };
            assert_eq!(manifest, expected);
        }
    }

    #[test]
    fn parse_produces_manifest_with_one_library() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.tempdir.path().to_path_buf();

        fixture.create_dummy_file(&std::path::PathBuf::from("generator.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("x.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("y.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("z.cpp"));

        let input = r#"
    [library.MyLibraryData]
    sources = ['x.cpp', 'y.cpp', 'z.cpp', 'generator.cpp']
    "#;

        let manifest = parse_toml(input, &manifest_dir).unwrap();
        let library = Library {
            name: "MyLibraryData".to_string(),
            sources: vec![
                manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                manifest_dir.join(std::path::PathBuf::from("generator.cpp")),
            ],
            dependencies: Vec::new(),
            defines: Vec::new(),
            compiler_flags: CompilerFlags::new(),
            lib_type: LibraryType::default(),
        };
        let expected = ManifestData {
            project_config: None,
            targets: vec![Target::Library(library)],
        };
        assert_eq!(manifest, expected);
    }

    #[test]
    fn parse_produces_manifest_with_library_with_dependency() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.tempdir.path().to_path_buf();

        fixture.create_dummy_file(&std::path::PathBuf::from("generator.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("x.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("y.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("z.cpp"));

        let dep_project_path = fixture.create_dummy_file(&std::path::PathBuf::from("SomeProject"));
        let second_dep_project_path =
            fixture.create_dummy_file(&std::path::PathBuf::from("SomeSecondProject"));
        let toml_with_require_recipe = format!(
            r#"
    [library.MyLibraryData]
    sources = ['x.cpp', 'y.cpp', 'z.cpp', 'generator.cpp']

    [library.MyLibraryData.dependencies]
    SomeProject = {{ path = "{}" }}
    SomeSecondProject = {{ path = "{}" }}
    "#,
            dep_project_path.display(),
            second_dep_project_path.display()
        );

        let manifest = parse_toml(&toml_with_require_recipe, &manifest_dir).unwrap();
        let library = Library {
            name: "MyLibraryData".to_string(),
            sources: vec![
                manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                manifest_dir.join(std::path::PathBuf::from("generator.cpp")),
            ],
            dependencies: vec![
                Dependency {
                    name: "SomeProject".to_string(),
                    data: DependencyData::Source(SourceData {
                        path: dep_project_path,
                        origin: IncludeSearchType::Include,
                    }),
                },
                Dependency {
                    name: "SomeSecondProject".to_string(),
                    data: DependencyData::Source(SourceData {
                        path: second_dep_project_path,
                        origin: IncludeSearchType::Include,
                    }),
                },
            ],
            defines: Vec::new(),
            compiler_flags: CompilerFlags::new(),
            lib_type: LibraryType::default(),
        };
        let expected = ManifestData {
            project_config: None,
            targets: vec![Target::Library(library)],
        };
        assert_eq!(manifest, expected);
    }

    #[test]
    fn parse_produces_manifest_with_executable_with_custom_defines() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.tempdir.path().to_path_buf();

        fixture.create_dummy_file(&std::path::PathBuf::from("x.cpp"));

        let input = r#"
    [executable.x]
    sources = ['x.cpp']
    [[executable.x.defines]]
    macro = 'MYMACRO'
    value = '1'

    [[executable.x.defines]]
    macro = 'MYSECONDMACRO'
    value = '0'
    "#;
        {
            let manifest = parse_toml(input, &manifest_dir).unwrap();
            let executable = Executable {
                name: "x".to_string(),
                sources: vec![manifest_dir.join(std::path::PathBuf::from("x.cpp"))],
                dependencies: Vec::new(),
                defines: vec![
                    Define {
                        macro_: "MYMACRO".to_string(),
                        value: Some("1".to_string()),
                    },
                    Define {
                        macro_: "MYSECONDMACRO".to_string(),
                        value: Some("0".to_string()),
                    },
                ],
                compiler_flags: CompilerFlags::new(),
            };
            let expected = ManifestData {
                project_config: None,
                targets: vec![Target::Executable(executable)],
            };
            assert_eq!(manifest, expected);
        }
    }

    #[test]
    fn parse_produces_manifest_with_one_library_with_custom_defines() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.tempdir.path().to_path_buf();

        fixture.create_dummy_file(&std::path::PathBuf::from("x.cpp"));

        let input = r#"
    [library.MyLibraryData]
    sources = ['x.cpp']
    
    [[library.MyLibraryData.defines]]
    macro = 'MYMACRO'
    value = '1'

    [[library.MyLibraryData.defines]]
    macro = 'MYSECONDMACRO'
    value = '0'
    "#;

        let manifest = parse_toml(input, &manifest_dir).unwrap();
        let library = Library {
            name: "MyLibraryData".to_string(),
            sources: vec![manifest_dir.join(std::path::PathBuf::from("x.cpp"))],
            dependencies: Vec::new(),
            defines: vec![
                Define {
                    macro_: "MYMACRO".to_string(),
                    value: Some("1".to_string()),
                },
                Define {
                    macro_: "MYSECONDMACRO".to_string(),
                    value: Some("0".to_string()),
                },
            ],
            compiler_flags: CompilerFlags::new(),
            lib_type: LibraryType::default(),
        };
        let expected = ManifestData {
            project_config: None,
            targets: vec![Target::Library(library)],
        };
        assert_eq!(manifest, expected);
    }
}
