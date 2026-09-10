use anyhow::Result;
mod cli;
mod diff_service;
mod git_repository;
use clap::Parser;
use cli::Args;
use diff_service::DiffService;
use std::path::Path;

fn main() -> Result<()> {
    let args = Args::parse();

    println!("source repo {}", args.source_repo);
    println!("target branch {}", args.target_branch);
    println!("source branch {}", args.source_branch);
    println!("dest repo {}", args.dest_repo);
    println!("dry run {}", args.dry_run);

    let files = DiffService::changed_files(
        Path::new(&args.source_repo),
        &args.target_branch,
        &args.source_branch,
    )?;
    for file in &files {
        println!("{}", file.display());
    }
    Ok(())
}
