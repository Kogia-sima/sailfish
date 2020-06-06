use quote::ToTokens;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use crate::config::Config;
use crate::error::*;
use crate::optimizer::Optimizer;
use crate::parser::Parser;
use crate::resolver::Resolver;
use crate::translator::{Translator, TranslatedSource};
use crate::util::rustfmt_block;

#[derive(Default)]
pub struct Compiler {
    config: Config
}

impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: Config) -> Self {
        Self { config }
    }

    fn translate_file_contents(&self, input: &Path) -> Result<TranslatedSource, Error> {
        let parser = Parser::new().delimiter(self.config.delimiter);
        let translator = Translator::new().escape(self.config.escape);
        let content = fs::read_to_string(input)
            .chain_err(|| format!("Failed to open template file: {:?}", input))?;

        let stream = parser.parse(&*content);
        translator.translate(stream)
    }

    pub fn compile_file(&self, template_dir: &Path, input: &Path, output: &Path) -> Result<(), Error> {
        // TODO: introduce cache system

        let input = input.canonicalize()?;

        let include_handler = Arc::new(|arg: &str| -> Result<_, Error> {
            let input_file = if arg.starts_with("/") {
                // absolute imclude
                template_dir.join(arg)
            } else {
                // relative include
                input.parent().unwrap().join(arg)
            };
            Ok(self.translate_file_contents(&*input_file)?.ast)
        });

        let resolver = Resolver::new().include_handler(include_handler);
        let optimizer = Optimizer::new();

        let compile_file = |input: &Path, output: &Path| -> Result<(), Error> {
            let mut tsource = self.translate_file_contents(input)?;

            resolver.resolve(&mut tsource.ast)?;
            optimizer.optimize(&mut tsource.ast);

            if let Some(parent) = output.parent() {
                fs::create_dir_all(parent)?;
            }
            if output.exists() {
                fs::remove_file(output)?;
            }

            let string = tsource.ast.into_token_stream().to_string();
            fs::write(output, rustfmt_block(&*string).unwrap_or(string))?;
            Ok(())
        };

        compile_file(&*input, &*output)
            .chain_err(|| "Failed to compile template.")
            .map_err(|mut e| {
                e.source = fs::read_to_string(&*input).ok();
                e.source_file = Some(input.to_owned());
                e
            })?;

        Ok(())
    }
}
