use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SourceFiles(std::vec::Vec<SourceFile>);

impl SourceFiles {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from_paths(sources: &[std::path::PathBuf]) -> Result<Self, AssociatedFileError> {
        Ok(Self(
            sources
                .iter()
                .map(|source| SourceFile::new(source))
                .collect::<Result<Vec<SourceFile>, AssociatedFileError>>()?,
        ))
    }

    pub fn push(&mut self, file: SourceFile) {
        self.0.push(file)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, SourceFile> {
        self.0.iter()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl std::convert::From<Vec<SourceFile>> for SourceFiles {
    fn from(value: Vec<SourceFile>) -> Self {
        Self(value)
    }
}

impl std::iter::IntoIterator for SourceFiles {
    type Item = <std::vec::Vec<SourceFile> as IntoIterator>::Item;
    type IntoIter = <std::vec::Vec<SourceFile> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> std::iter::IntoIterator for &'a SourceFiles {
    type Item = <&'a std::vec::Vec<SourceFile> as IntoIterator>::Item;
    type IntoIter = <&'a std::vec::Vec<SourceFile> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum AssociatedFileError {
    #[error("Could not specify file type: {0}")]
    CouldNotSpecifyFileType(String),
    #[error("Source file {0:?} does not exist")]
    FileNotExisting(std::path::PathBuf),
    #[error("Source file {0} has no extension")]
    NoFileExtension(PathBuf),
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, serde::Serialize, serde::Deserialize)]
pub struct SourceFile {
    file_type: FileType,
    file: std::path::PathBuf,
}

impl SourceFile {
    pub fn new(file: &std::path::Path) -> Result<Self, AssociatedFileError> {
        if !file.exists() {
            return Err(AssociatedFileError::FileNotExisting(file.to_path_buf()));
        }
        let file_type = match file.extension().and_then(|extension| extension.to_str()) {
            Some("cpp") | Some("cc") | Some("c") => FileType::Source,
            Some("h") | Some("hpp") => FileType::Header,
            Some(ft) => {
                return Err(AssociatedFileError::CouldNotSpecifyFileType(ft.to_string()));
            }
            None => {
                return Err(AssociatedFileError::NoFileExtension(file.to_path_buf()));
            }
        };
        log::debug!("Found source file {}", file.display());

        Ok(Self {
            file_type,
            file: file.to_path_buf(),
        })
    }

    pub fn file(&self) -> std::path::PathBuf {
        self.file.clone()
    }

    pub fn is_source(&self) -> bool {
        self.file_type == FileType::Source
    }

    pub fn is_header(&self) -> bool {
        self.file_type == FileType::Header
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, serde::Serialize, serde::Deserialize)]
pub enum FileType {
    Source,
    Header,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_file_is_source_file_type() {
        let tempdir = tempdir::TempDir::new("test").unwrap();
        let file = tempdir.path().join("file.cpp");
        std::fs::File::create(&file).unwrap();
        let expected = SourceFile {
            file_type: FileType::Source,
            file: file.clone(),
        };
        let actual = SourceFile::new(&file).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn header_file_is_header_file_type() {
        let tempdir = tempdir::TempDir::new("test").unwrap();
        let file = tempdir.path().join("file.h");
        std::fs::File::create(&file).unwrap();
        let expected = SourceFile {
            file_type: FileType::Header,
            file: file.clone(),
        };
        let actual = SourceFile::new(&file).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn fails_to_recognize_file_type() {
        let tempdir = tempdir::TempDir::new("test").unwrap();
        let file = tempdir.path().join("file.py");
        std::fs::File::create(&file).unwrap();
        let actual = SourceFile::new(&file);
        assert_eq!(
            actual.unwrap_err(),
            AssociatedFileError::CouldNotSpecifyFileType(String::from("py"))
        );
    }
}
