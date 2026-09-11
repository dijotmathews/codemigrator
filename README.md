# codemigrator

A CLI tool for migrating a set of changed files from one git repository/branch
onto a fresh branch in another repository.

It diffs `--source-branch` against `--target-branch` in the source repository,
copies the changed files (as they exist on `--source-branch`) into a new
branch in the destination repository, and commits them there — with a
`--dry-run` mode to preview everything first.

## Features

- **Branch diffing** — computes the changed files between `--target-branch`
  and `--source-branch` in the source repository, using three-dot
  (`target...source`) semantics so unrelated changes made on the target
  branch after the two diverged are excluded.
- **Extension filtering** — only files with `.java`, `.properties`, or
  `.json` extensions are considered.
- **git-status-style change summary** — before doing anything, prints the
  changed files labeled like `git status` (`new file:`, `modified:`,
  `deleted:`).
- **Keeps the destination branch current** — fast-forwards `--target-branch`
  in the destination repository from `origin` before diffing/copying, whether
  or not that branch is currently checked out. Accepts either `dev` or
  `origin/dev` as the branch name.
- **New branch per migration** — interactively prompts for a branch name and
  creates/checks it out in the destination repository.
- **Copies file content, not just working-tree state** — files are read
  directly from the `--source-branch` git blob, so the source repository
  doesn't need that branch checked out. Files deleted on `--source-branch`
  are removed from the destination if present there.
- **Commits the migration** — interactively prompts for a commit message and
  commits exactly the files that were copied/deleted (nothing else in the
  destination working tree is staged).
- **Dry-run mode** — `--dry-run` prints every action (`would copy`, `would
  delete`, `would create branch`) without touching either repository, and
  skips the branch/commit prompts' side effects.

## Requirements

- Rust (edition 2024 toolchain)
- The `git` binary available on `PATH` (used for fetching, branching, and
  committing in the destination repository)
- Both `--source-repo` and `--dest-repo` must already be local git clones;
  `--dest-repo` needs an `origin` remote configured for the pull step to
  succeed.

## Build

```sh
cargo build --release
```

The binary is produced at `target/release/codemigrator`.

## Usage

```sh
codemigrator \
  --source-repo <path-to-source-repo> \
  --target-branch <branch> \
  --source-branch <branch> \
  --dest-repo <path-to-destination-repo> \
  [--dry-run]
```

| Flag              | Description                                                                 |
|--------------------|------------------------------------------------------------------------------|
| `--source-repo`    | Path to the local git repository containing the changes to migrate.          |
| `--target-branch`  | The base branch to diff against (e.g. `main` or `origin/main`). Also the branch fast-forwarded from `origin` in the destination repo. |
| `--source-branch`  | The branch containing the changes to copy out.                               |
| `--dest-repo`      | Path to the local git repository the files should be copied into.            |
| `--dry-run`        | Preview every step without modifying either repository.                      |

### Example

```sh
codemigrator \
  --source-repo ~/dev/pg-p3-components \
  --target-branch origin/dev \
  --source-branch RJCOLGASC \
  --dest-repo ~/dev/intern-pg-p3-components
```

This will:

1. Print the two repositories and branches involved.
2. Fast-forward `dev` in `intern-pg-p3-components` from `origin`.
3. Diff `origin/dev...RJCOLGASC` in `pg-p3-components` and print the matching
   changed files (`.java`/`.properties`/`.json`) in a `git status`-style list.
4. Prompt: `Enter branch name to create in destination repository:` — creates
   and checks out that branch in `intern-pg-p3-components`.
5. Copy each changed file's content from `RJCOLGASC` into the destination
   repo (deleting files there that were deleted on `RJCOLGASC`).
6. Prompt: `Enter commit message:` — stages and commits exactly the copied
   files on the new branch.

Run with `--dry-run` first to review the plan — file copies, deletions, and
branch creation are all skipped, and no commit prompt appears.

## Testing

```sh
cargo test
```

Tests spin up real, throwaway git repositories (via `git` on `PATH`) in
temporary directories to exercise diffing, pulling, branching, copying, and
committing against actual git behavior rather than mocks.
