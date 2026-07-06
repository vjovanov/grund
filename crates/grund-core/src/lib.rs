use anyhow::{Context, Result, anyhow};
use ignore::WalkBuilder;
use once_cell::sync::Lazy;
use rayon::prelude::*;
use regex::Regex;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;
use unicode_normalization::UnicodeNormalization;

// §AR-bindings.1: `grund-core` is the shared implementation crate used by the
// published `grund` CLI and, next, the optional LSP server. The category files
// are still included flat to keep this first package split behavior-preserving.
include!("grammar.rs");
include!("model.rs");
include!("config.rs");
include!("scanner.rs");
include!("checker.rs");
include!("checker_cmd.rs");
include!("workspace_context.rs");
include!("output.rs");
include!("show.rs");
include!("show_render.rs");
include!("fmt.rs");
include!("fmt_links.rs");
include!("id.rs");
include!("refs.rs");
include!("cover.rs");
include!("list.rs");
include!("completions.rs");
include!("integrations.rs");
include!("init_templates.rs");
include!("init.rs");
include!("api.rs");
include!("compat_cli.rs");
include!("tests.rs");
