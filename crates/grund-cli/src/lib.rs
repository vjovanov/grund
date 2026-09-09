//! Top-level dispatch, shared output helpers, and `main_entry`. One file per
//! command follows, in `SUBCOMMANDS` order — the frontend crate is assembled by
//! `include!` just as `grund-core` is, so a command's file is a flat slice of
//! the same crate and needs no `mod`/`use` wiring (§AR-core-module-layout.3).

// §AR-bindings.3: the `grund` frontend crate owns top-level CLI dispatch.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use grund_core::{
    AGENT_SETUP_INSTRUCTIONS, ApiScanError, BatchShowQuery, CheckOpts, CitationDisjunction,
    CitationLevel, CitationRules, CitationTarget, CompleteIdsOpts, Config, CoverCitation,
    CoverOpts, FetchFailureKind, Finding, FindingSite, FmtOpts, FmtScanAbort, IdOpts, IdProposal,
    IdProposalOutcome, InitAgentEntrypointSelection, InitFsHome, InitNext, InitOpts, InitOutput,
    ListEntry, ListOpts, ListSizeEntry, ListSizeOpts, NamespaceMatch, PointSizeUnit, RefHit,
    RefsOpts, RefsQueryFailure, Report, ShowFormat, ShowMode, ShowOpts, ShowQueryError,
    REFS_QUERY_FAILURE_WARNING, canonical_template_text, check_with_opts, complete_ids, cover,
    effective_config, fetch_snapshot, format_references, init, list, list_sizes,
    names_member_id_candidate, print_config_warnings, propose_id, refs, refs_outcome,
    refs_query_failure_is_exit_one,
    render_finding_sites_json, run_integrations, show_batch_with_scope, show_with_scope,
    validate_config,
};
use grund_core::{CHECK_FINDING_CODES, CheckFindingSelection};

const SUBCOMMANDS: &[&str] = &[
    "check",
    "show",
    "list",
    "refs",
    "cover",
    "fmt",
    "fetch",
    "id",
    "init",
    "config",
    "agent-setup-instructions",
    "completions",
    "integrations",
];

include!("cli_help.rs");
include!("cli_help_check.rs");
include!("cli_help_show.rs");
include!("cli.rs");
include!("cli_check.rs");
include!("cli_show.rs");
include!("cli_show_batch.rs");
include!("cli_list.rs");
include!("cli_refs.rs");
include!("cli_cover.rs");
include!("cli_fmt.rs");
include!("cli_fetch.rs");
include!("cli_id.rs");
include!("cli_init.rs");
include!("cli_config.rs");
include!("cli_complete.rs");
