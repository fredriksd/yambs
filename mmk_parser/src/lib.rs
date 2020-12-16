//!
//#![warn(missing_debug_implementations, rust_2018_idioms_, missing_docs)]

use std::collections::HashMap;
use std::vec::Vec;
use std::fs;
use std::io;
use std::path::Path;
use error::MyMakeError;
use regex::Regex;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Mmk
{
    pub data: HashMap<String, Vec<String>>,
}

impl Mmk
{
    pub fn new() -> Mmk
    {
        Mmk { data: HashMap::new() }
    }

    pub fn parse_file(self: &mut Self, data: &String) -> &mut Mmk
    {
        let no_comment_data = remove_comments(data);
        parse_mmk(self, &no_comment_data, "MMK_SOURCES");
        parse_mmk(self, &no_comment_data, "MMK_HEADERS");
        parse_mmk(self, &no_comment_data, "MMK_EXECUTABLE");
        parse_mmk(self, &no_comment_data, "MMK_DEPEND")        
    }

    pub fn to_string(self: &Self, key: &str) -> String
    {
        let mut formatted_string = String::new();
        if self.data.contains_key(key)
        {
            for item in &self.data[key]
            {
                if *item == String::from("")
                {
                    break;
                }
                if key == "MMK_DEPEND"
                {
                    formatted_string.push_str("-I");
                }
                formatted_string.push_str(&item[..].trim());
                formatted_string.push_str(" ");
            }            
        }
        formatted_string.trim_end().to_string()
    }

    pub fn valid_keyword(self: &Self, keyword: &str) -> bool
    {
        keyword    == "MMK_DEPEND"
        || keyword == "MMK_SOURCES" 
        || keyword == "MMK_HEADERS"
        || keyword == "MMK_EXECUTABLE"
    }

    pub fn sources_to_objects(self: &Self) -> String {
        let sources = &self.to_string("MMK_SOURCES");
        let objects = sources.replace(".cpp", ".o");
        objects
    }
}

pub fn validate_file_path(file_path_as_str: &str) -> Result<std::path::PathBuf, MyMakeError> {
    let file_path = std::path::PathBuf::from(file_path_as_str);
    if !file_path.is_file() {
        return Err(MyMakeError::from(format!("Error: {:?} is not a valid path!", &file_path)));
    }
    Ok(file_path)
}

pub fn read_file(file_path: &Path) -> Result<String, io::Error>
{
    fs::read_to_string(&file_path)
}

fn clip_string(data: &String, keyword:&str) -> String
{
    let keyword_index: usize = match data.find(&keyword)
    {
        Some(match_index) => match_index,
        None => return String::from(""),
    };
    data[keyword_index..].to_string()
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

pub fn parse_mmk<'a>(mmk_container: &'a mut Mmk, data: &String, keyword: &str) -> &'a mut Mmk
{
    if mmk_container.valid_keyword(keyword)
    {
        let filtered_data: String = clip_string(&data, &keyword).replace(" ", "")
                                                .to_string();

        if filtered_data == ""
        {
            mmk_container.data.insert(keyword.to_string(), vec![filtered_data]);
            return mmk_container;
        }
        let split_data: Vec<&str> = filtered_data.trim_start()
                                                    .split_terminator("=")
                                                    .collect();

        let mut mmk_right_side: Vec<String> = split_data[1].split_terminator("\\").map(|s| 
            {
                s.trim_end_matches("MMK_DEPEND")
                .trim_end_matches("MMK_SOURCES")
                .trim_end_matches("MMK_HEADERS")
                .trim_end_matches("MMK_EXECUTABLE")
                .trim_matches(&['\n', '\r'][..])
                .to_string()
            }
        ).collect();
        mmk_right_side.retain(|x| x.is_empty() == false);
        mmk_container.data.insert(keyword.to_string(), mmk_right_side);
    }
    mmk_container
}


#[cfg(test)]
pub mod tests
{
    use super::*;
    use pretty_assertions::assert_eq;
    #[test]
    fn test_mmk_file_reader()
    {
        let path = std::path::Path::new("/home/fredrik/bin/mymake/mmk_parser/src/test.mmk");
        let content = read_file(&path);        
        assert_eq!(content.unwrap(),"\
            #This is a comment.\n\
            MMK_DEPEND = /home/fredrik/Documents/Tests/AStarPathFinder/PlanGenerator/test/\n\
            \n\
            MMK_SOURCES = filename.cpp \\
              otherfilename.cpp\n\
            \n\
            #This is a second comment.\n\
            MMK_EXECUTABLE = x\n\
            ");
    }

    #[test]
    fn test_remove_comments()
    {
        let path = std::path::Path::new("/home/fredrik/bin/mymake/mmk_parser/src/test.mmk");
        let content = read_file(&path).unwrap();     
        assert_eq!(remove_comments(&content),"\
            \n\
            MMK_DEPEND = /home/fredrik/Documents/Tests/AStarPathFinder/PlanGenerator/test/\n\
            \n\
            MMK_SOURCES = filename.cpp \\
              otherfilename.cpp\n\
            \n\
            \n\
            MMK_EXECUTABLE = x\n\
            ");
    }
    
    #[test]
    fn test_parse_mmk_sources()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
                                                          otherfilename.cpp\n");

        parse_mmk( &mut mmk_content, &content, "MMK_SOURCES");
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp", "otherfilename.cpp"]);
    }

    #[test]
    fn test_parse_mmk_source()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\");
        parse_mmk(&mut mmk_content, &content, "MMK_SOURCES");
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp"]);
    }


    #[test]
    fn test_parse_mmk_source_newline_after_end()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
        ");
        parse_mmk(&mut mmk_content, &content, "MMK_SOURCES");
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp"]);
    }

    #[test]
    fn test_parse_mmk_dependencies()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/to/depend/on\n");
        parse_mmk(&mut mmk_content, &content, "MMK_DEPEND");
        assert_eq!(mmk_content.data["MMK_DEPEND"], ["/some/path/to/depend/on", "/another/path/to/depend/on"]);
    }

    #[test]
    fn test_multiple_keywords()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
                                                          otherfilename.cpp\n
                                            
                                            MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/\n
                                                         
                                            MMK_EXECUTABLE = main");

        parse_mmk(&mut mmk_content, &content, "MMK_SOURCES");
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp", "otherfilename.cpp"]);
        parse_mmk(&mut mmk_content, &content, "MMK_DEPEND");
        assert_eq!(mmk_content.data["MMK_DEPEND"], ["/some/path/to/depend/on", "/another/path/"]);
        parse_mmk(&mut mmk_content, &content, "MMK_EXECUTABLE");
        assert_eq!(mmk_content.data["MMK_EXECUTABLE"], ["main"]);
    }
    #[test]
    fn test_parse_mmk_no_keyword()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/to/depend/on\n");
        parse_mmk(&mut mmk_content, &content, "MMK_DEP");
        assert!(mmk_content.data.is_empty() == true);
    }
}

