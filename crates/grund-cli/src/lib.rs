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
    format_references, init, list, propose_id, refs, run_integrations, show_with_scope,
    validate_config,
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
include!("cli.rs");
