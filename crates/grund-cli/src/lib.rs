// §RM-core-cli-split: the `grund` frontend crate owns top-level CLI dispatch.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use grund_core::{
    AGENT_SETUP_INSTRUCTIONS, ApiScanError, CheckOpts, CitationDisjunction, CitationLevel,
    CitationRules, CitationTarget, CompleteIdsOpts, Config, CoverCitation, CoverOpts,
    CoverTextEntry, Finding, FmtOpts, IdOpts, IdProposal, IdProposalOutcome,
    InitAgentEntrypointSelection, InitFsHome, InitNext, InitOpts, InitOutput, ListEntry, ListOpts,
    NamespaceMatch, RefHit, RefsOpts, Report, ShowFormat, ShowMode, ShowOpts,
    canonical_template_text, check_with_opts, complete_ids, cover, cover_text, effective_config,
    format_references, init, list, print_config_warnings, propose_id, refs, run_integrations,
    show_with_scope, validate_config,
};

const SUBCOMMANDS: &[&str] = &[
    "check",
    "show",
    "list",
    "refs",
    "cover",
    "fmt",
    "id",
    "init",
    "config",
    "agent-setup-instructions",
    "completions",
    "integrations",
];

include!("cli_help.rs");
// Top-level dispatch, shared output helpers, and `main_entry`. One file per
// command follows, in `SUBCOMMANDS` order — the frontend crate is assembled by
// `include!` just as `grund-core` is, so a command's file is a flat slice of
// the same crate and needs no `mod`/`use` wiring (§AR-core-module-layout.3).
include!("cli.rs");
include!("cli_check.rs");
include!("cli_show.rs");
include!("cli_list.rs");
include!("cli_refs.rs");
include!("cli_cover.rs");
include!("cli_fmt.rs");
include!("cli_id.rs");
include!("cli_init.rs");
include!("cli_config.rs");
include!("cli_complete.rs");
