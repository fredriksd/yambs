use super::*;
use tempdir::TempDir;

#[test]
fn get_source_directory_from_path_test() {
    let dir = TempDir::new("example").unwrap();
    let source_dir = dir.path().join("src");
    create_dir(&source_dir).unwrap();
    assert_eq!(get_source_directory_from_path(dir.path()), source_dir);
}

#[test]
fn get_source_directory_from_path_no_source_directory_defaults_to_original_path_test() {
    let dir = TempDir::new("example").unwrap();
    assert_eq!(get_source_directory_from_path(dir.path()), dir.path());
}

#[test]
fn get_include_directory_from_path_test() {
    let dir = TempDir::new("example").unwrap();
    let include_dir = dir.path().join("include");
    create_dir(&include_dir).unwrap();
    let actual = get_include_directory_from_path(dir.path());
    assert!(actual.is_ok());
    assert_eq!(actual.unwrap(), include_dir);
}

#[test]
fn get_include_directory_from_path_search_one_directory_up_test() {
    let dir = TempDir::new("example").unwrap();
    let include_dir = dir.path().join("include");
    create_dir(&include_dir).unwrap();
    let actual = get_include_directory_from_path(dir.path().join("src"));
    assert!(actual.is_ok());
    assert_eq!(actual.unwrap(), include_dir);
}

#[test]
fn get_include_directory_from_path_fails_test() {
    let dir = TempDir::new("example").unwrap();
    let result = get_include_directory_from_path(dir.path());
    assert!(result.is_err());
}

#[test]
fn is_source_directory_src_test() {
    let dir = TempDir::new("example").unwrap();
    let source_dir = dir.path().join("src");
    create_dir(&source_dir).unwrap();
    assert_eq!(is_source_directory(source_dir), true);
}

#[test]
fn is_source_directory_source_test() {
    let dir = TempDir::new("example").unwrap();
    let source_dir = dir.path().join("source");
    create_dir(&source_dir).unwrap();
    assert_eq!(is_source_directory(source_dir), true);
}

#[test]
fn is_source_directory_false_test() {
    let source_dir = PathBuf::from("/some/path/without/source/directory");
    assert_eq!(is_source_directory(source_dir), false);
}

#[test]
fn is_test_directory_true_test() {
    let dir = TempDir::new("example").unwrap();
    let test_dir = dir.path().join("test");
    create_dir(&test_dir).unwrap();
    assert_eq!(is_test_directory(test_dir), true);
}

#[test]
fn is_test_directory_false_test() {
    let test_dir = PathBuf::from("/some/path/without/test/directory");
    assert_eq!(is_test_directory(test_dir), false);
}

#[test]
fn get_head_directory_gets_head_test() {
    let dir = PathBuf::from("some/path/to/strip/head");
    let expected = PathBuf::from("head");
    assert_eq!(get_head_directory(&dir), &expected);
}
