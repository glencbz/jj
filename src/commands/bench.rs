// Copyright 2023 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::fmt::Debug;
use std::io;
use std::time::Instant;

use clap::Subcommand;
use criterion::Criterion;
use jujutsu_lib::index::HexPrefix;
use jujutsu_lib::repo::Repo;

use crate::cli_util::{CommandError, CommandHelper};
use crate::ui::Ui;

/// Commands for benchmarking internal operations
#[derive(Subcommand, Clone, Debug)]
#[command(hide = true)]
pub enum BenchCommands {
    #[command(name = "commonancestors")]
    CommonAncestors(BenchCommonAncestorsArgs),
    #[command(name = "isancestor")]
    IsAncestor(BenchIsAncestorArgs),
    #[command(name = "resolveprefix")]
    ResolvePrefix(BenchResolvePrefixArgs),
    #[command(name = "revset")]
    Revset(BenchRevsetArgs),
    #[command(name = "revsets")]
    Revsets(BenchRevsetsArgs),
}

/// Find the common ancestor(s) of a set of commits
#[derive(clap::Args, Clone, Debug)]
pub struct BenchCommonAncestorsArgs {
    revision1: String,
    revision2: String,
}

/// Checks if the first commit is an ancestor of the second commit
#[derive(clap::Args, Clone, Debug)]
pub struct BenchIsAncestorArgs {
    ancestor: String,
    descendant: String,
}

/// Walk the revisions in the revset
#[derive(clap::Args, Clone, Debug)]
pub struct BenchRevsetArgs {
    revisions: String,
}

/// Benchmark multiple revsets specified in a file
#[derive(clap::Args, Clone, Debug)]
pub struct BenchRevsetsArgs {
    file: String,
}

/// Resolve a commit ID prefix
#[derive(clap::Args, Clone, Debug)]
pub struct BenchResolvePrefixArgs {
    prefix: String,
}

fn run_bench<R, O>(ui: &mut Ui, id: &str, mut routine: R) -> io::Result<()>
where
    R: (FnMut() -> O) + Copy,
    O: Debug,
{
    let mut criterion = Criterion::default();
    let before = Instant::now();
    let result = routine();
    let after = Instant::now();
    writeln!(
        ui,
        "First run took {:?} and produced: {:?}",
        after.duration_since(before),
        result
    )?;
    criterion.bench_function(id, |bencher: &mut criterion::Bencher| {
        bencher.iter(routine);
    });
    Ok(())
}

pub(crate) fn cmd_bench(
    ui: &mut Ui,
    command: &CommandHelper,
    subcommand: &BenchCommands,
) -> Result<(), CommandError> {
    match subcommand {
        BenchCommands::CommonAncestors(command_matches) => {
            let workspace_command = command.workspace_helper(ui)?;
            let commit1 = workspace_command.resolve_single_rev(&command_matches.revision1)?;
            let commit2 = workspace_command.resolve_single_rev(&command_matches.revision2)?;
            let index = workspace_command.repo().index();
            let routine =
                || index.common_ancestors(&[commit1.id().clone()], &[commit2.id().clone()]);
            run_bench(
                ui,
                &format!(
                    "commonancestors-{}-{}",
                    &command_matches.revision1, &command_matches.revision2
                ),
                routine,
            )?;
        }
        BenchCommands::IsAncestor(command_matches) => {
            let workspace_command = command.workspace_helper(ui)?;
            let ancestor_commit =
                workspace_command.resolve_single_rev(&command_matches.ancestor)?;
            let descendant_commit =
                workspace_command.resolve_single_rev(&command_matches.descendant)?;
            let index = workspace_command.repo().index();
            let routine = || index.is_ancestor(ancestor_commit.id(), descendant_commit.id());
            run_bench(
                ui,
                &format!(
                    "isancestor-{}-{}",
                    &command_matches.ancestor, &command_matches.descendant
                ),
                routine,
            )?;
        }
        BenchCommands::ResolvePrefix(command_matches) => {
            let workspace_command = command.workspace_helper(ui)?;
            let prefix = HexPrefix::new(&command_matches.prefix).unwrap();
            let index = workspace_command.repo().index();
            let routine = || index.resolve_prefix(&prefix);
            run_bench(ui, &format!("resolveprefix-{}", prefix.hex()), routine)?;
        }
        BenchCommands::Revset(command_matches) => {
            let workspace_command = command.workspace_helper(ui)?;
            let expression = workspace_command.parse_revset(&command_matches.revisions)?;
            // Time both evaluation and iteration. Note that we don't clear caches (such as
            // commit objects in `Store`) between each run (`criterion` doesn't
            // seem to support that).
            let routine = || {
                let count = workspace_command
                    .evaluate_revset(expression.clone())
                    .unwrap()
                    .iter()
                    .count();
                format!("{count} commits")
            };
            run_bench(
                ui,
                &format!("revset-{}", &command_matches.revisions),
                routine,
            )?;
        }
        BenchCommands::Revsets(command_matches) => {
            let workspace_command = command.workspace_helper(ui)?;
            let file_path = command.cwd().join(&command_matches.file);
            let revsets = std::fs::read_to_string(&file_path)?;
            let mut criterion = Criterion::default();
            let mut group = criterion.benchmark_group("revsets");
            for revset in revsets.lines() {
                let revset = revset.trim();
                if revset.starts_with('#') || revset.is_empty() {
                    continue;
                }
                writeln!(ui, "----------Testing revset: {revset}----------\n")?;
                let expression = workspace_command.parse_revset(revset)?;
                // Time both evaluation and iteration. Note that we don't clear caches (such as
                // commit objects in `Store`) between each run (`criterion`
                // doesn't seem to support that).
                let routine = || {
                    workspace_command
                        .evaluate_revset(expression.clone())
                        .unwrap()
                        .iter()
                        .count()
                };
                let before = Instant::now();
                let result = routine();
                let after = Instant::now();
                writeln!(
                    ui,
                    "First run took {:?} and produced {result} commits",
                    after.duration_since(before),
                )?;

                group.bench_function(&format!("revset {}", &revset), |bencher| {
                    bencher.iter(routine);
                });
            }
            // Neither of these seem to report anything...
            group.finish();
            criterion.final_summary();
        }
    }
    Ok(())
}