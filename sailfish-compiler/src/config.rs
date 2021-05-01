use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Config {
    pub delimiter: char,
    pub escape: bool,
    pub rm_whitespace: bool,
    pub template_dirs: Vec<PathBuf>,
    #[doc(hidden)]
    pub cache_dir: PathBuf,
    #[doc(hidden)]
    pub _non_exhaustive: (),
}

impl Default for Config {
    fn default() -> Self {
        Self {
            template_dirs: Vec::new(),
            delimiter: '%',
            escape: true,
            cache_dir: Path::new(env!("OUT_DIR")).join("cache"),
            rm_whitespace: false,
            _non_exhaustive: (),
        }
    }
}

#[cfg(feature = "config")]
mod imp {
    use serde::Deserialize;
    use std::fs;

    use super::*;
    use crate::error::*;

    impl Config {
        pub fn search_file_and_read(base: &Path) -> Result<Config, Error> {
            // search config file
            let mut path = PathBuf::new();
            let mut config = Config::default();

            for component in base.iter() {
                path.push(component);
                path.push("sailfish.toml");

                if path.is_file() {
                    let config_file =
                        ConfigFile::read_from_file(&*path).map_err(|mut e| {
                            e.source_file = Some(path.to_owned());
                            e
                        })?;

                    if let Some(template_dirs) = config_file.template_dirs {
                        for template_dir in template_dirs.into_iter().rev() {
                            let expanded =
                                expand_env_vars(template_dir).map_err(|mut e| {
                                    e.source_file = Some(path.to_owned());
                                    e
                                })?;

                            let template_dir = PathBuf::from(expanded);

                            if template_dir.is_absolute() {
                                config.template_dirs.push(template_dir);
                            } else {
                                config
                                    .template_dirs
                                    .push(path.parent().unwrap().join(template_dir));
                            }
                        }
                    }

                    if let Some(delimiter) = config_file.delimiter {
                        config.delimiter = delimiter;
                    }

                    if let Some(escape) = config_file.escape {
                        config.escape = escape;
                    }

                    if let Some(optimizations) = config_file.optimizations {
                        if let Some(rm_whitespace) = optimizations.rm_whitespace {
                            config.rm_whitespace = rm_whitespace;
                        }
                    }
                }

                path.pop();
            }

            Ok(config)
        }
    }

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    struct Optimizations {
        rm_whitespace: Option<bool>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(deny_unknown_fields)]
    struct ConfigFile {
        template_dirs: Option<Vec<String>>,
        delimiter: Option<char>,
        escape: Option<bool>,
        optimizations: Option<Optimizations>,
    }

    impl ConfigFile {
        fn read_from_file(path: &Path) -> Result<Self, Error> {
            let content = fs::read_to_string(path)
                .chain_err(|| format!("Failed to read configuration file {:?}", path))?;
            Self::from_string(&content)
        }

        fn from_string(content: &str) -> Result<Self, Error> {
            toml::from_str::<Self>(content).map_err(|e| error(e.to_string()))
        }
    }

    fn expand_env_vars<S: AsRef<str>>(input: S) -> Result<String, Error> {
        use std::env;

        let input = input.as_ref();
        let len = input.len();
        let mut iter = input.chars().enumerate();
        let mut result = String::new();

        let mut found = false;
        let mut env_var = String::new();

        while let Some((i, c)) = iter.next() {
            match c {
                '$' if found == false => {
                    if let Some((_, cc)) = iter.next() {
                        if cc == '{' {
                            found = true;
                        } else {
                            // We didn't find a trailing { after the $
                            // so we push the chars read onto the result
                            result.push(c);
                            result.push(cc);
                        }
                    }
                }
                '}' if found => {
                    let val = env::var(&env_var).map_err(|e| match e {
                        env::VarError::NotPresent => {
                            error(format!("Environment variable ({}) not set", env_var))
                        }
                        env::VarError::NotUnicode(_) => error(format!(
                            "Environment variable ({}) contents not valid unicode",
                            env_var
                        )),
                    })?;
                    result.push_str(&val);

                    env_var.clear();
                    found = false;
                }
                _ => {
                    if found {
                        env_var.push(c);

                        // Check if we're at the end with an unclosed environment variable:
                        // ${MYVAR instead of ${MYVAR}
                        // If so, push it back onto the string as some systems allows the $ { characters in paths.
                        if i == len - 1 {
                            result.push_str("${");
                            result.push_str(&env_var);
                        }
                    } else {
                        result.push(c);
                    }
                }
            }
        }

        Ok(result)
    }

    fn error<T: Into<String>>(msg: T) -> Error {
        make_error!(ErrorKind::ConfigError(msg.into()))
    }

    #[cfg(test)]
    mod tests {
        use crate::config::imp::expand_env_vars;
        use std::env;

        #[test]
        fn expands_env_vars() {
            env::set_var("TESTVAR", "/a/path");
            let input = "/path/to/${TESTVAR}Templates";
            let output = expand_env_vars(input).unwrap();
            assert_eq!(output, "/path/to//a/pathTemplates");
        }

        #[test]
        fn retains_case_sensitivity() {
            env::set_var("tEstVar", "/a/path");
            let input = "/path/${tEstVar}";
            let output = expand_env_vars(input).unwrap();
            assert_eq!(output, "/path//a/path");
        }

        #[test]
        fn retains_unclosed_env_var() {
            let input = "/path/to/${UNCLOSED";
            let output = expand_env_vars(input).unwrap();
            assert_eq!(output, input);
        }

        #[test]
        fn ingores_markers() {
            let input = "path/{$/$}/${/to/{";
            let output = expand_env_vars(input).unwrap();
            assert_eq!(output, input);
        }

        #[test]
        fn errors_on_unset_env_var() {
            let input = "/path/to/${UNSET}";
            let output = expand_env_vars(input);
            assert!(output.is_err());
        }
    }
}
