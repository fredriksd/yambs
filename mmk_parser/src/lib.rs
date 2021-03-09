//!
//#![warn(missing_debug_implementations, rust_2018_idioms_, missing_docs)]

use std::collections::HashMap;
use std::vec::Vec;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use error::MyMakeError;
use regex::Regex;

// Pretty assertions only for testing.
#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Mmk
{
    pub data: HashMap<String, Vec<String>>,
}

impl Mmk {
    pub fn new() -> Mmk
    {
        Mmk { data: HashMap::new() }
    }


    pub fn to_string(self: &Self, key: &str) -> String
    {
        let mut formatted_string = String::new();
        if self.data.contains_key(key) {
            for item in &self.data[key] {
                if item == "" {
                    break;
                }
                if key == "MMK_DEPEND" {
                    formatted_string.push_str("-I");
                }
                if key == "MMK_SYS_INCLUDE" {
                    formatted_string.push_str("-isystem ");
                }
                formatted_string.push_str(item);
                formatted_string.push_str(" ");
            }
        }
        formatted_string.trim_end().to_string()
    }


    pub fn valid_keyword(self: &Self, keyword: & str) -> Result<(), MyMakeError>
    {
        if keyword == "MMK_DEPEND"
        || keyword == "MMK_SOURCES"
        || keyword == "MMK_HEADERS"
        || keyword == "MMK_EXECUTABLE"
        || keyword == "MMK_SYS_INCLUDE" 
        || keyword == "MMK_CXXFLAGS_APPEND" 
        || keyword == "MMK_CPPFLAGS_APPEND" 
        || keyword == "MMK_LIBRARY_LABEL" {
            Ok(())
        }
        else {
            Err(MyMakeError::from(format!("{} is not a valid keyword.", keyword)))
        }
    }


    pub fn sources_to_objects(self: &Self) -> String {
        let sources = &self.to_string("MMK_SOURCES");
        let objects = sources.replace(".cpp", ".o");
        objects
    }


    fn parse_mmk_expression(&mut self, mmk_keyword: &str, data_iter: &mut std::str::Lines) -> Result<(), MyMakeError> {
        self.valid_keyword(mmk_keyword)?;
        let mut arg_vec: Vec<String> = Vec::new();
        let mut current_line = data_iter.next();
        while current_line != None {
            let line = current_line.unwrap().trim();
            if line != "" {
                let arg = current_line.unwrap().trim().to_string();
                arg_vec.push(arg);                
            }
            else {
                break;
            }
            current_line = data_iter.next();
        }
        self.data.insert(String::from(mmk_keyword), arg_vec);
        Ok(())
    }


    pub fn has_library_label(&self) -> bool {
        self.data.contains_key("MMK_LIBRARY_LABEL")
    }


    pub fn parse(&mut self, data: &String) -> Result<(), MyMakeError> {
        let no_comment_data = remove_comments(data);
        let mut lines = no_comment_data.lines();
        let mut current_line = lines.next();
        let mmk_rule = Regex::new(r"(MMK_\w+):[\r\n]*").unwrap();
        while current_line != None {
            if let Some(captured) = mmk_rule.captures(current_line.unwrap()) {
                let mmk_keyword = captured.get(1).unwrap().as_str();
                self.valid_keyword(mmk_keyword)?;
                self.parse_mmk_expression(mmk_keyword, &mut lines)?;
                current_line = lines.next();
            }
            else {
                current_line = lines.next();
            } 
        }
        Ok(())
    }


    pub fn source_file_path(&self, source: &String) -> Option<PathBuf> {
        let mut source_path = PathBuf::from(source);
        if source_path.pop() {
            return Some(source_path);
        }
        None
    }
}


pub fn validate_file_path(file_path_as_str: &str) -> Result<PathBuf, MyMakeError> {
    let file_path = PathBuf::from(file_path_as_str).canonicalize().unwrap();
    if !file_path.is_file() {
        return Err(MyMakeError::from(format!("Error: {:?} is not a valid path!", &file_path)));
    }
    Ok(file_path)
}


pub fn read_file(file_path: &Path) -> Result<String, io::Error>
{
    fs::read_to_string(&file_path)
}


pub fn remove_comments(data: &String) -> String {
    let mut lines = data.lines();
    let mut current_line = lines.next();
    let comment_expression = Regex::new(r"#.*").unwrap();
    let mut non_comment_data: String = data.clone();
    
    while current_line != None {
        non_comment_data = comment_expression.replace(&non_comment_data, "").to_string();
        current_line = lines.next();
    }
    non_comment_data
}


#[cfg(test)]
pub mod tests
{
    use super::*;
    #[test]
    fn test_mmk_file_reader()
    {
        let path = std::path::Path::new("/home/fredrik/bin/mymake/mmk_parser/src/test.mmk");
        let content = read_file(&path);        
        assert_eq!(content.unwrap(),("\
#This is a comment.
MMK_DEPEND:
   /home/fredrik/Documents/Tests/AStarPathFinder/PlanGenerator/test/

MMK_SOURCES:
   filename.cpp
   otherfilename.cpp

#This is a second comment.
MMK_EXECUTABLE:
   x\n"));
    }

    #[test]
    fn test_remove_comments()
    {
        let path = std::path::Path::new("/home/fredrik/bin/mymake/mmk_parser/src/test.mmk");
        let content = read_file(&path).unwrap();     
        assert_eq!(remove_comments(&content),String::from("
MMK_DEPEND:
   /home/fredrik/Documents/Tests/AStarPathFinder/PlanGenerator/test/

MMK_SOURCES:
   filename.cpp
   otherfilename.cpp


MMK_EXECUTABLE:
   x\n"));
    }
    
    #[test]
    fn test_parse_mmk_sources() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES:\n\
                                                filename.cpp\n\
                                                otherfilename.cpp\n");

        mmk_content.parse( &content)?;
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp", "otherfilename.cpp"]);
        Ok(())
    }

    #[test]
    fn test_parse_mmk_source() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES:\n\
                                                filename.cpp");
        mmk_content.parse(&content)?;
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp"]);
        Ok(())
    }


    #[test]
    fn test_parse_mmk_dependencies() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_DEPEND:\n\
                                                /some/path/to/depend/on \n\
                                                /another/path/to/depend/on\n");
        mmk_content.parse(&content)?;
        assert_eq!(mmk_content.data["MMK_DEPEND"], ["/some/path/to/depend/on", "/another/path/to/depend/on"]);
        Ok(())
    }

    #[test]
    fn test_multiple_keywords() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES:
                                                filename.cpp
                                                otherfilename.cpp
                                            
                                            MMK_DEPEND:
                                                /some/path/to/depend/on
                                                /another/path/
                                                         
                                            MMK_EXECUTABLE:
                                                main");
        mmk_content.parse(&content)?;
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp", "otherfilename.cpp"]);
        assert_eq!(mmk_content.data["MMK_DEPEND"], ["/some/path/to/depend/on", "/another/path/"]);
        assert_eq!(mmk_content.data["MMK_EXECUTABLE"], ["main"]);
        Ok(())
    }
    #[test]
    fn test_parse_mmk_no_valid_keyword() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_DEPENDS:\n\
                                                /some/path/to/depend/on \n\
                                                /another/path/to/depend/on\n");
        let result = mmk_content.parse(&content);                
        assert!(result.is_err());
        assert_eq!(&String::from("MMK_DEPENDS is not a valid keyword."), result.unwrap_err().to_string());
        Ok(())
    }
}

