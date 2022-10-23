use structopt::StructOpt;

use crate::cli;
use crate::cli::configurations;
use crate::errors::{CommandLineError, FsError};

// TODO: Need to add tests for C++ validation and sanitizer validation
// TODO: Add default values that correctly correspond for 'configuration' when not all options are
// specified.
// TODO: Perhaps, BuildManagerConfigurations should be defaulted to have a predefined set of configurations
// TODO: and remove those which are replaced by command line opted input.
// TODO: At a later stage, should jobs be added to build configurations or should it be abstracted
// TODO: to its own struct?

#[derive(StructOpt, Debug)]
#[structopt(
    version = "0.1.0",
    name = "YAMBS",
    about = "\
             GNU Make build system overlay for C++ projects. Yambs generates makefiles and builds the project with the \n\
             specifications written in the respective YAMBS files."
)]
pub struct CommandLine {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(Debug)]
pub struct ManifestDirectory(std::path::PathBuf);

impl ManifestDirectory {
    pub fn as_path(&self) -> &std::path::Path {
        self.0.as_path()
    }
}

impl std::default::Default for ManifestDirectory {
    fn default() -> Self {
        Self(std::env::current_dir().unwrap())
    }
}

impl std::string::ToString for ManifestDirectory {
    fn to_string(&self) -> String {
        self.0.display().to_string()
    }
}

impl std::str::FromStr for ManifestDirectory {
    type Err = CommandLineError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let canonicalized_path =
            cli::canonicalize_path(&std::path::PathBuf::from(s)).map_err(FsError::Canonicalize)?;
        Ok(Self {
            0: canonicalized_path,
        })
    }
}

#[derive(StructOpt, Debug)]
pub enum Subcommand {
    /// Build project specified by manifest YAMBS file.
    Build(BuildOpts),
    /// Print previous invocation line used and exit.
    Remake(RemakeOpts),
}

#[derive(StructOpt, Debug)]
#[structopt(setting(structopt::clap::AppSettings::TrailingVarArg))]
pub struct BuildOpts {
    /// Input manifest file for YAMBS. By default, Yambs searches for yambs.toml manifest in current directory.
    #[structopt(default_value, hide_default_value(true), long = "manifest-directory")]
    pub manifest_dir: ManifestDirectory,
    /// Set runtime configurations (build configurations, C++ standard, sanitizers, etc)
    #[structopt(flatten)]
    pub configuration: ConfigurationOpts,
    /// Set parallelization of builds for Make.
    #[structopt(short = "j", long = "jobs", default_value = "10")]
    pub jobs: u8,
    /// Set build directory. Generated output by Yambs will be put here. Defaults to current working directory.
    #[structopt(
        long,
        short = "b",
        default_value,
        hide_default_value(true),
        parse(try_from_str)
    )]
    pub build_directory: cli::BuildDirectory,
    /// Create dottie graph of build tree and exit.
    #[structopt(long = "dottie-graph")]
    pub create_dottie_graph: bool,
    /// Toggles verbose output.
    #[structopt(short = "v", long = "verbose")]
    pub verbose: bool,
    #[structopt(hidden = true)]
    pub make_args: Vec<String>,
}

#[derive(StructOpt, Debug, Clone)]
pub struct ConfigurationOpts {
    /// Build configuration to use
    #[structopt(default_value, long = "build-type")]
    pub build_type: configurations::BuildType,
    /// C++ standard to be passed to compiler
    #[structopt(default_value,
                long = "std",
                parse(try_from_str = configurations::CXXStandard::parse))]
    pub cxx_standard: configurations::CXXStandard,
    /// Enable sanitizers
    #[structopt(long = "sanitizer")]
    pub sanitizer: Option<configurations::Sanitizer>,
}

#[derive(StructOpt, Debug)]
pub struct RemakeOpts {
    /// Build directory to read invocation from.
    #[structopt(parse(try_from_str))]
    pub build_directory: cli::BuildDirectory,
}

#[cfg(test)]
mod tests {
    use super::*;
    use structopt::StructOpt;

    #[test]
    fn arguments_passed_after_double_hyphen_are_parsed_raw() {
        let build_args = ["--build-type", "debug", "--", "-j", "10", "-DNDEBUG=1"];
        let build_opts = BuildOpts::from_iter(std::iter::once("build").chain(build_args));
        assert_eq!(build_opts.make_args, vec!["-j", "10", "-DNDEBUG=1"]);
    }
}
