use std::path::Path;

use ensembler::CmdLineRunner;
use crate::tera;

use crate::plugins::Plugin;
use crate::Result;

impl Plugin {
    pub async fn format(&self, input: &str, input_filename: &Path) -> Result<String> {
        let command = &self.format.command;
        let mut args = vec![];
        let mut ctx = tera::Context::default();
        ctx.insert("filename", input_filename.to_str().unwrap());
        for arg in &self.format.args {
            let arg = tera::render(arg, &ctx)?;
            args.push(arg.to_string());
        }
        let output = CmdLineRunner::new(command).args(args).stdin_string(input).execute().await?;
        Ok(output.stdout)
    }
}
