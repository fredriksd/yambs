use crate::parser::compiler_flags::CompilerFlags;

#[derive(Debug, PartialEq, Eq)]
pub struct Executable {
    pub name: String,
    pub main: std::path::PathBuf,
    pub sources: Vec<std::path::PathBuf>,
    pub dependencies: Vec<Dependency>,
    pub compiler_flags: Option<CompilerFlags>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Library {
    pub name: String,
    pub main: std::path::PathBuf,
    pub sources: Vec<std::path::PathBuf>,
    pub dependencies: Vec<Dependency>,
    pub compiler_flags: Option<CompilerFlags>,
    pub lib_type: LibraryType,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct RawExecutableData {
    #[serde(flatten)]
    pub common_raw: RawCommonData,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct RawLibraryData {
    #[serde(flatten)]
    pub common_raw: RawCommonData,
    #[serde(default)]
    pub lib_type: LibraryType,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct RawCommonData {
    pub main: std::path::PathBuf,
    pub sources: Vec<std::path::PathBuf>,
    #[serde(default)]
    pub dependencies: std::collections::HashMap<String, DependencyData>,
    #[serde(flatten)]
    pub compiler_flags: Option<CompilerFlags>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub enum LibraryType {
    Static,
    Dynamic,
}

impl Default for LibraryType {
    fn default() -> Self {
        LibraryType::Static
    }
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    pub data: DependencyData,
}

impl Dependency {
    pub fn new(name: &str, data: &DependencyData) -> Self {
        Self {
            name: name.to_string(),
            data: data.to_owned(),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
pub struct DependencyData {
    pub path: std::path::PathBuf,
    #[serde(default)]
    pub origin: DependencySource,
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
pub enum DependencySource {
    System,
    Include,
}

impl Default for DependencySource {
    fn default() -> Self {
        DependencySource::Include
    }
}
