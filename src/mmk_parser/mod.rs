//!
//#![warn(missing_debug_implementations, rust_2018_idioms_, missing_docs)]

//TODO: Burde ha muligheten til å kunne bruke path som bruker relativ-path-direktiver (../)

use std::collections::HashMap;
use std::vec::Vec;

use regex::Regex;

mod keyword;
mod mmk_constants;

use crate::errors::{FsError, MyMakeError, ParseError};
use crate::utility;
pub use keyword::Keyword;
pub use mmk_constants::{Constant, Constants};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Mmk {
    data: HashMap<String, Vec<Keyword>>,
    constants: Constants,
    file: std::path::PathBuf,
}

impl Mmk {
    pub fn new(path: &std::path::Path) -> Mmk {
        let source_path = path.to_path_buf();

        Mmk {
            data: HashMap::new(),
            constants: Constants::new(path, &source_path),
            file: source_path,
        }
    }

    pub fn file(&self) -> std::path::PathBuf {
        self.file.to_owned()
    }

    pub fn data(&self) -> &HashMap<String, Vec<Keyword>> {
        &self.data
    }

    #[allow(unused)]
    pub fn data_mut(&mut self) -> &mut HashMap<String, Vec<Keyword>> {
        &mut self.data
    }

    pub fn has_executables(&self) -> bool {
        self.data.contains_key("MMK_EXECUTABLE")
    }

    pub fn has_dependencies(&self) -> bool {
        self.data.contains_key("MMK_REQUIRE")
    }

    pub fn get_args(&self, key: &str) -> Option<&Vec<Keyword>> {
        if self.valid_keyword(key).ok().is_none() {
            None
        } else if self.data.contains_key(key) {
            Some(&self.data[key])
        } else {
            None
        }
    }

    pub fn to_string(&self, key: &str) -> String {
        let mut formatted_string = String::new();
        if self.data.contains_key(key) {
            for item in &self.data[key] {
                if item.argument() == "" {
                    break;
                }

                if key == "MMK_SYS_INCLUDE" {
                    formatted_string.push_str("-isystem ");
                }
                formatted_string.push_str(item.argument());
                formatted_string.push(' ');
            }
        }
        formatted_string.trim_end().to_string()
    }

    pub fn get_include_directories(&self) -> Result<String, MyMakeError> {
        if self.data.contains_key("MMK_REQUIRE") {
            let mut formatted_string = String::new();
            for keyword in &self.data["MMK_REQUIRE"] {
                if keyword.option() == "SYSTEM" {
                    formatted_string.push_str("-isystem");
                    formatted_string.push(' ');
                } else {
                    formatted_string.push_str("-I");
                }
                let dep_path = utility::get_include_directory_from_path(
                    &std::path::PathBuf::from(keyword.argument()),
                )?;
                formatted_string.push_str(dep_path.to_str().unwrap());
                formatted_string.push(' ');
            }
            return Ok(formatted_string.trim_end().to_string());
        }
        Ok(String::from(""))
    }

    pub fn valid_keyword(&self, keyword: &str) -> Result<(), ParseError> {
        let stripped_keyword = keyword.trim_end_matches(':');
        if stripped_keyword == "MMK_REQUIRE"
            || stripped_keyword == "MMK_SOURCES"
            || stripped_keyword == "MMK_HEADERS"
            || stripped_keyword == "MMK_EXECUTABLE"
            || stripped_keyword == "MMK_SYS_INCLUDE"
            || stripped_keyword == "MMK_CXXFLAGS_APPEND"
            || stripped_keyword == "MMK_CPPFLAGS_APPEND"
            || stripped_keyword == "MMK_LIBRARY_LABEL"
        {
            Ok(())
        } else {
            Err(ParseError::InvalidKeyword {
                file: self.file.to_path_buf(),
                keyword: stripped_keyword.to_string(),
            })
        }
    }

    fn parse_mmk_expression(
        &mut self,
        mmk_keyword: &str,
        data_iter: &mut std::str::Lines,
    ) -> Result<(), ParseError> {
        self.valid_keyword(mmk_keyword)?;
        let mut arg_vec: Vec<Keyword> = Vec::new();
        let mut current_line = data_iter.next();
        while current_line != None {
            let line = current_line.unwrap().trim();
            if !line.is_empty() && self.valid_keyword(line).is_err() {
                let keyword = self.parse_and_create_keyword(line);

                arg_vec.push(keyword);
            } else if line.is_empty() {
                break;
            } else {
                return Err(ParseError::InvalidSpacing {
                    file: self.file.to_path_buf(),
                });
            }
            current_line = data_iter.next();
        }
        self.data.insert(String::from(mmk_keyword), arg_vec);
        Ok(())
    }

    fn parse_and_create_keyword(&self, line: &str) -> Keyword {
        let line_split: Vec<&str> = line.split(' ').collect();
        let keyword: Keyword;
        if line_split.len() == 1 {
            let arg = line_split[0];
            keyword = Keyword::from(&self.replace_constant_with_value(&arg.to_string()))
        } else {
            let option = line_split[1];
            let arg = line_split[0];
            keyword = Keyword::from(&self.replace_constant_with_value(&arg.to_string()))
                .with_option(option);
        }
        keyword
    }

    pub fn has_library_label(&self) -> bool {
        self.data.contains_key("MMK_LIBRARY_LABEL")
    }

    pub fn has_system_include(&self) -> bool {
        self.data.contains_key("MMK_SYS_INCLUDE")
    }

    pub fn parse(&mut self, data: &str) -> Result<(), ParseError> {
        let no_comment_data = remove_comments(&data);
        let mut lines = no_comment_data.lines();
        let mut current_line = lines.next();
        let mmk_rule = Regex::new(r"(MMK_\w+):[\r\n]*").unwrap();
        while current_line != None {
            if let Some(captured) = mmk_rule.captures(current_line.unwrap()) {
                let mmk_keyword = captured.get(1).unwrap().as_str();
                self.parse_mmk_expression(mmk_keyword, &mut lines)?;
                current_line = lines.next();
            } else {
                current_line = lines.next();
            }
        }
        Ok(())
    }

    fn replace_constant_with_value(&self, mmk_keyword_value: &str) -> String {
        if let Some(constant_string) = self.constants.get_constant(&mmk_keyword_value.to_string()) {
            let item = self
                .constants
                .get_item(Constant::new(&constant_string))
                .unwrap();
            let constant_reconstructed = format!("${{{}}}", constant_string);
            mmk_keyword_value.replace(&constant_reconstructed, &item)
        } else {
            mmk_keyword_value.to_string()
        }
    }

    pub fn source_file_path(&self, source: &str) -> Option<std::path::PathBuf> {
        let mut source_path = std::path::PathBuf::from(source);
        if source_path.pop() {
            return Some(source_path);
        }
        None
    }
}

pub fn validate_file_path(file_path_as_str: &str) -> Result<std::path::PathBuf, FsError> {
    let file_path = std::path::PathBuf::from(file_path_as_str)
        .canonicalize()
        .map_err(FsError::Canonicalize)?;

    if !file_path.is_file() {
        return Err(FsError::FileDoesNotExist(file_path));
    }
    Ok(file_path)
}

pub fn validate_file_name(path: &std::path::Path) -> Result<(), ParseError> {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    match file_name {
        "lib.mmk" | "run.mmk" => (),
        _ => {
            return Err(ParseError::InvalidFilename(file_name.to_string()));
        }
    };
    Ok(())
}

pub fn remove_comments(data: &str) -> String {
    let mut lines = data.lines();
    let mut current_line = lines.next();
    let comment_expression = Regex::new(r"#.*").unwrap();
    let mut non_comment_data = data.to_string();

    while current_line != None {
        non_comment_data = comment_expression
            .replace(&non_comment_data, "")
            .to_string();
        current_line = lines.next();
    }
    non_comment_data
}

#[cfg(test)]
#[path = "./mod_test.rs"]
mod lib_test;
