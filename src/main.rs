use anyhow::Result;
mod cli;
mod copy_service;
mod diff_service;
mod git_repository;
#[cfg(test)]
mod test_util;
use clap::Parser;
use cli::Args;
use copy_service::CopyService;
use diff_service::DiffService;
use git_repository::GitRepository;
use std::io::{self, Write};
use std::path::Path;

fn prompt(label: &str) -> Result<String> {
    print!("{label}");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

fn prompt_branch_name() -> Result<String> {
    let branch_name = prompt("Enter branch name to create in destination repository: ")?;

    if branch_name.is_empty() {
        anyhow::bail!("branch name must not be empty");
    }

    Ok(branch_name)
}

fn prompt_commit_message() -> Result<String> {
    let message = prompt("Enter commit message: ")?;

    if message.is_empty() {
        anyhow::bail!("commit message must not be empty");
    }

    Ok(message)
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Source repository:      {}", args.source_repo);
    println!("Destination repository: {}", args.dest_repo);
    println!(
        "Migrating changes from '{}' onto '{}'",
        args.source_branch, args.target_branch
    );
    if args.dry_run {
        println!("Dry run: no files will be copied and no branch will be created");
    }

    println!("pulling latest {} from origin in repo {}" , args.target_branch, args.dest_repo);
    GitRepository::pull_branch(Path::new(&args.dest_repo), &args.target_branch)?;

    let files = DiffService::changed_files(
        Path::new(&args.source_repo),
        &args.target_branch,
        &args.source_branch,
    )?;

    println!("Changes to be copied:");
    for file in &files {
        println!("\t{file}");
    }
    println!();

    let branch_name = prompt_branch_name()?;
    if args.dry_run {
        println!(
            "would create branch {} in destination repository",
            branch_name
        );
    } else {
        GitRepository::create_branch(Path::new(&args.dest_repo), &branch_name)?;
        println!("created branch {} in destination repository", branch_name);
    }

    let touched_paths = CopyService::apply(
        Path::new(&args.source_repo),
        &args.source_branch,
        Path::new(&args.dest_repo),
        &files,
        args.dry_run,
    )?;

    if args.dry_run {
        println!("Dry run: skipping commit");
    } else if touched_paths.is_empty() {
        println!("No files were copied; nothing to commit");
    } else {
        let commit_message = prompt_commit_message()?;
        GitRepository::commit_changes(Path::new(&args.dest_repo), &touched_paths, &commit_message)?;
        println!(
            "committed {} file(s) to branch {} in destination repository",
            touched_paths.len(),
            branch_name
        );
    }

    Ok(())
}
