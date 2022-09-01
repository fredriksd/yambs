mod constants;

// FIXME: Write tests!
pub fn parse(toml_path: &std::path::Path) -> Result<Recipe, ParseTomlError> {
    // let toml_fh = std::fs::File::open(toml_path).map_err(ParseTomlError::FailedToOpen)?;
    let toml_content =
        String::from_utf8(std::fs::read(toml_path).map_err(ParseTomlError::FailedToRead)?).unwrap();
    parse_toml(&toml_content)
}

fn parse_toml(toml: &str) -> Result<Recipe, ParseTomlError> {
    toml::from_str(toml).map_err(ParseTomlError::FailedToParse)
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct Recipe {
    sources: Vec<String>,
    #[serde(flatten)]
    program_type: ProgramType,
    requires: Option<Vec<String>>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
enum ProgramType {
    #[serde(rename = "executable")]
    Executable(String),
    #[serde(rename = "library")]
    Library(String),
}

#[derive(thiserror::Error, Debug)]
pub enum ParseTomlError {
    #[error("Failed to parse TOML recipe file.")]
    FailedToParse(#[source] toml::de::Error),
    #[error("Failed to read TOML recipe file.")]
    FailedToRead(#[source] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    const TOML_RECIPE: &str = r#"
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    executable = 'x'
    "#;

    const TOML_WITH_REQUIRE_RECIPE: &str = r#"
    requires = ["SomeProject", "SomeSecondProject"]
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    executable = 'x'
    "#;

    #[test]
    fn parse_produces_recipe_file_from_toml() {
        {
            let recipe = parse_toml(TOML_RECIPE).unwrap();
            let expected = Recipe {
                sources: vec![
                    "x.cpp".to_string(),
                    "y.cpp".to_string(),
                    "z.cpp".to_string(),
                ],
                program_type: ProgramType::Executable("x".to_string()),
                requires: None,
            };
            assert_eq!(recipe, expected);
        }
        {
            let recipe = parse_toml(TOML_WITH_REQUIRE_RECIPE).unwrap();
            let expected = Recipe {
                sources: vec![
                    "x.cpp".to_string(),
                    "y.cpp".to_string(),
                    "z.cpp".to_string(),
                ],
                program_type: ProgramType::Executable("x".to_string()),
                requires: Some(vec![
                    "SomeProject".to_string(),
                    "SomeSecondProject".to_string(),
                ]),
            };
            assert_eq!(recipe, expected);
        }
    }
}
