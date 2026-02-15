use std::path::PathBuf;

use {clap::Parser, clap_complete::Shell};

#[derive(Parser, Debug)]
#[clap(version, about, author)]
pub struct CliArgs {
  #[arg(long, short = 'c', default_value = "./config.json")]
  pub config: String,
  #[clap(subcommand)]
  pub command: Option<CliCommand>,
}

#[derive(Parser, Debug)]
pub enum CliCommand {
  Start {
    #[clap(flatten)]
    start: CliStart,
  },
  Completions {
    shell: Shell,
  },
}

#[derive(Parser, Debug, Default)]
pub struct CliStart {
  #[arg(long, short = 'p')]
  pub path: Option<PathBuf>,
}
