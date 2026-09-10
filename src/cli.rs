use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long)]
    pub source_repo: String,

    #[arg(long)]
    pub target_branch: String,

    #[arg(long)]
    pub source_branch: String,

    #[arg(long)]
    pub dest_repo: String,

    #[arg(long)]
    pub dry_run: bool,
}
