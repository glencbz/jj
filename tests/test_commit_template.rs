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

use common::TestEnvironment;
use regex::Regex;

pub mod common;

#[test]
fn test_log_author_timestamp() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_success(test_env.env_root(), &["init", "repo", "--git"]);
    let repo_path = test_env.env_root().join("repo");

    test_env.jj_cmd_success(&repo_path, &["describe", "-m", "first"]);
    test_env.jj_cmd_success(&repo_path, &["new", "-m", "second"]);

    let stdout = test_env.jj_cmd_success(&repo_path, &["log", "-T", "author.timestamp()"]);
    insta::assert_snapshot!(stdout, @r###"
    @ 2001-02-03 04:05:09.000 +07:00
    o 2001-02-03 04:05:07.000 +07:00
    o 1970-01-01 00:00:00.000 +00:00
    "###);
}

#[test]
fn test_log_author_timestamp_ago() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_success(test_env.env_root(), &["init", "repo", "--git"]);
    let repo_path = test_env.env_root().join("repo");

    test_env.jj_cmd_success(&repo_path, &["describe", "-m", "first"]);
    test_env.jj_cmd_success(&repo_path, &["new", "-m", "second"]);

    let stdout = test_env.jj_cmd_success(&repo_path, &["log", "-T", "author.timestamp().ago()"]);
    let line_re = Regex::new(r"@|o [0-9]+ years ago").unwrap();
    assert!(
        stdout.lines().all(|x| line_re.is_match(x)),
        "expected every line to match regex"
    );
}

#[test]
fn test_log_default_colors() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_success(test_env.env_root(), &["init", "repo", "--git"]);
    let repo_path = test_env.env_root().join("repo");

    test_env.jj_cmd_success(&repo_path, &["describe", "-m", "description 1"]);
    test_env.jj_cmd_success(&repo_path, &["branch", "create", "my-branch"]);
    let stdout = test_env.jj_cmd_success(&repo_path, &["log", "--color=always"]);
    insta::assert_snapshot!(stdout, @r###"
    @ [1;35m9a45c67d3e96[0m [1;33mtest.user@example.com[0m [1;36m2001-02-03 04:05:08.000 +07:00[0m [1;35mmy-branch[0m [1;34m45a3aa29e907[0m
    | [1;37mdescription 1[0m
    o [35m000000000000[0m [33m[0m [36m1970-01-01 00:00:00.000 +00:00[0m [34m000000000000[0m
      (no description set)
    "###);
}