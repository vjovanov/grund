#!/usr/bin/env bash
set -euo pipefail

# try-integrations.sh — a manual testbed for the clickable-citation clients
# (§FS-integrations). It installs the integrations built from *this* working
# tree into a throwaway HOME, then launches the terminal or editor against it,
# in a repository full of real citations, with every open logged where you can
# see it.
#
# Why a sandbox HOME: `grund integrations <client> --write` writes to fixed
# targets under $HOME — the client dotfile, ~/.local/bin/grund-open, the user
# preference, and the global agent instruction files of six agents
# (§FS-integrations.4). Pointing HOME at a temporary directory keeps all of
# that off your real configuration, and every client then discovers its config
# by itself, exactly as it would for a real user.
#
#   scripts/try-integrations.sh show                print citations here, no setup
#   scripts/try-integrations.sh doctor              what can be tested here
#   scripts/try-integrations.sh resolve             headless resolver checks
#   scripts/try-integrations.sh wezterm             install + launch a client
#   scripts/try-integrations.sh wezterm --flatpak   the Flatpak-packaged build
#
# `show` touches nothing: it prints one citation of every shape into the
# terminal you are already in, so you can click them against whatever you have
# actually installed. Add --tui to print them with the mouse captured, the way
# an agent TUI does.
#
# Clients: wezterm, kitty, tmux, vscode, codium, iterm2 (macOS, manual).

usage() {
    awk 'NR > 3 && /^#/ { sub(/^# ?/, ""); print; next } NR > 3 { exit }' "$0"
}

# ---------------------------------------------------------------- options ---

REPO_ROOT=$(cd "$(dirname "$0")/.." && pwd)
FIXTURE=$REPO_ROOT
SANDBOX=${GRUND_TESTBED_DIR:-${TMPDIR:-/tmp}/grund-integrations-testbed}
BINARY=
BUILD=1
PROFILE=debug
FRESH=0
FLATPAK=0
TUI=0
EDITOR_CMD=
SUBDIR=

command=${1:-}
case $command in
    ''|-h|--help) usage; exit 0 ;;
esac
shift

while [ "$#" -gt 0 ]; do
    case $1 in
        --repo) FIXTURE=$(cd "$2" && pwd); shift 2 ;;
        --sandbox) SANDBOX=$2; shift 2 ;;
        --binary) BINARY=$2; BUILD=0; shift 2 ;;
        --no-build) BUILD=0; shift ;;
        --release) PROFILE=release; shift ;;
        --fresh) FRESH=1; shift ;;
        --flatpak) FLATPAK=1; shift ;;
        --tui) TUI=1; shift ;;
        --editor) EDITOR_CMD=$2; shift 2 ;;
        --cd) SUBDIR=$2; shift 2 ;;
        -h|--help) usage; exit 0 ;;
        *) echo "try-integrations: unknown option '$1'" >&2; exit 2 ;;
    esac
done

die() { printf 'try-integrations: %s\n' "$1" >&2; exit 1; }
say() { printf '%s\n' "$*"; }
step() { printf '\n== %s\n' "$*"; }

[ -f "$FIXTURE/.agents/grund.toml" ] ||
    die "no .agents/grund.toml in $FIXTURE — pass --repo <dir> pointing at a grund repository"

# The resolver climbs from the *clicked pane's* directory to the config root
# (§FS-integrations.3.1). Starting a subdirectory down is the honest test: a
# click from the repository root would pass even if the climb were broken.
if [ -z "$SUBDIR" ]; then
    for candidate in docs/functional-spec docs .; do
        [ -d "$FIXTURE/$candidate" ] && { SUBDIR=$candidate; break; }
    done
fi
WORKDIR=$FIXTURE/$SUBDIR

# ---------------------------------------------------------------- binary ----

build_binary() {
    # `show` and `doctor` only read; never make them wait on a build.
    if [ "$BUILD" = 1 ] && [ -z "$BINARY" ] && { [ "$command" = show ] || [ "$command" = doctor ]; }; then
        if [ -x "$REPO_ROOT/target/$PROFILE/grund" ]; then
            BINARY=$REPO_ROOT/target/$PROFILE/grund
        elif command -v grund >/dev/null 2>&1; then
            BINARY=$(command -v grund)
        fi
    fi
    if [ -n "$BINARY" ]; then
        [ -x "$BINARY" ] || die "not executable: $BINARY"
        GRUND_BIN=$(cd "$(dirname "$BINARY")" && pwd)/$(basename "$BINARY")
        return
    fi
    GRUND_BIN=$REPO_ROOT/target/$PROFILE/grund
    if [ "$BUILD" = 1 ]; then
        step "building grund ($PROFILE)"
        if [ "$PROFILE" = release ]; then
            (cd "$REPO_ROOT" && cargo build --release -p grund) >&2
        else
            (cd "$REPO_ROOT" && cargo build -p grund) >&2
        fi
    fi
    [ -x "$GRUND_BIN" ] ||
        die "no grund binary at $GRUND_BIN (drop --no-build, or pass --binary <path>)"
}

# --------------------------------------------------------------- sandbox ----

MARKER_FILE=.grund-testbed

prepare_sandbox() {
    if [ "$FRESH" = 1 ] && [ -e "$SANDBOX" ]; then
        # Only ever wipe a directory this script created.
        [ -f "$SANDBOX/$MARKER_FILE" ] ||
            die "$SANDBOX exists but is not a testbed sandbox — refusing to remove it"
        rm -rf "$SANDBOX"
    fi
    mkdir -p "$SANDBOX/.local/bin" "$SANDBOX/.cargo/bin" "$SANDBOX/.config" "$SANDBOX/bin"
    : >"$SANDBOX/$MARKER_FILE"
    : >>"$SANDBOX/opens.log"

    # `grund-open` and the WezTerm spawn both look for `grund` on PATH, and the
    # WezTerm spawn rebuilds PATH from $HOME/.local/bin and $HOME/.cargo/bin
    # (§FS-integrations.3.1) — which is the sandbox HOME here, so the binary
    # under test has to be reachable through both.
    ln -sf "$GRUND_BIN" "$SANDBOX/.local/bin/grund"
    ln -sf "$GRUND_BIN" "$SANDBOX/.cargo/bin/grund"

    SANDBOX_ENV=(
        "HOME=$SANDBOX"
        "XDG_CONFIG_HOME=$SANDBOX/.config"
        "XDG_DATA_HOME=$SANDBOX/.local/share"
        "PATH=$SANDBOX/.local/bin:$PATH"
        "GRUND_TESTBED_LOG=$SANDBOX/opens.log"
        "GRUND_TESTBED_EDITOR=$EDITOR_CMD"
        "GRUND_OPEN_CMD=$SANDBOX/bin/grund-open-log"
    )
}

install_client() {
    local client=$1
    step "installing $client into $SANDBOX"
    env "${SANDBOX_ENV[@]}" "$GRUND_BIN" integrations "$client" --write
}

write_helpers() {
    # Every open is recorded, then forwarded to --editor when one was given.
    # A click that resolves but opens nothing is the failure mode this whole
    # feature keeps hitting, so the log is the primary instrument: the spawn is
    # detached (WezTerm) or in a popup that closes (tmux), and its stdout is
    # otherwise lost.
    cat >"$SANDBOX/bin/grund-open-log" <<'EOF'
#!/bin/sh
printf '[%s] open %s\n' "$(date '+%H:%M:%S')" "$1" >>"${GRUND_TESTBED_LOG:-/dev/null}"
[ -n "${GRUND_TESTBED_EDITOR:-}" ] || exit 0
# shellcheck disable=SC2086
exec ${GRUND_TESTBED_EDITOR} "$1"
EOF

    cat >"$SANDBOX/bin/grund-open-echo" <<'EOF'
#!/bin/sh
printf '%s\n' "$1"
EOF

    # A program that has captured the mouse. WezTerm drops user mouse bindings
    # while one is in the foreground, which is why the gestures are registered
    # twice (§FS-integrations.3.1) — and every program that prints citations in
    # anger (agent TUIs, full-screen editors) is one of these.
    cat >"$SANDBOX/bin/tui" <<'EOF'
#!/bin/sh
printf '\033[?1000h\033[?1002h\033[?1006h'
trap 'printf "\033[?1006l\033[?1002l\033[?1000l\n"' EXIT INT TERM
printf 'Mouse reporting is ON — this pane behaves like an agent TUI.\n\n'
cat "${GRUND_TESTBED_SHEET:?}"
printf '\nClick a citation above, then press enter to leave mouse mode. '
read -r _ || true
EOF

    chmod +x "$SANDBOX/bin/grund-open-log" "$SANDBOX/bin/grund-open-echo" "$SANDBOX/bin/tui"
}

# -------------------------------------------------------------- citations ---

# Read the repository's own citation marker: `[reference] marker` is per-repo
# (§FS-config.3.1), and neither the client matchers nor the resolver hardcode §.
read_marker() {
    local m
    # No `| head -1`: an early-exiting reader SIGPIPEs the writer, and this
    # script runs under `set -o pipefail`.
    m=$(awk -F'"' '/^ *marker *=/ && !seen { print $2; seen = 1 }' "$FIXTURE/.agents/grund.toml")
    printf '%s' "${m:-§}"
}

grund_here() { (cd "$FIXTURE" && env "${SANDBOX_ENV[@]}" "$GRUND_BIN" "$@"); }

first_id() { grund_here list --kind "$1" 2>/dev/null | awk 'NR==1 {print $1}'; }

# Pick the citation forms the matchers and the resolver each have to survive.
# Everything is discovered from the fixture repository, so this works on any
# grund repo, not only this one.
collect_citations() {
    MARKER=$(read_marker)
    ID_PLAIN=$(first_id FS)
    [ -n "$ID_PLAIN" ] || ID_PLAIN=$(grund_here list 2>/dev/null | awk 'NR==1 {print $1}')
    [ -n "$ID_PLAIN" ] || die "no declarations found in $FIXTURE"

    ID_SECTION=
    for section in 1 2 3; do
        if grund_here "$ID_PLAIN.$section" --format json >/dev/null 2>&1; then
            ID_SECTION=$ID_PLAIN.$section
            break
        fi
    done

    ID_E2E=$(first_id E2E)
    ID_OTHER=$(first_id AR)
    [ -n "$ID_OTHER" ] || ID_OTHER=$ID_PLAIN
    ID_UNKNOWN=FS-no-such-declaration-here

    # A workspace-qualified `<alias>/<ID>` only resolves from the workspace
    # root above the member, which is what makes the resolver's climb continue
    # past a member config instead of stopping there.
    ID_QUALIFIED=$(grund_here list 2>/dev/null | awk '$1 ~ /\// && !seen { print $1; seen = 1 }')
    MEMBER_DIR=$(find "$FIXTURE" -mindepth 2 -maxdepth 6 -path '*/.agents/grund.toml' \
        -not -path '*/target/*' 2>/dev/null | awk 'NR == 1 { print }')
    [ -n "$MEMBER_DIR" ] && MEMBER_DIR=$(dirname "$(dirname "$MEMBER_DIR")")
}

sheet_body() {
    local client=$1 open_gesture=$2 peek_gesture=$3
    {
        printf 'grund citations — %s\n' "$client"
        printf 'repo %s, cwd %s\n\n' "$FIXTURE" "$SUBDIR"
        printf 'open:  %s\n' "$open_gesture"
        printf 'peek:  %s\n\n' "$peek_gesture"
        printf 'Click targets:\n\n'
        printf '  %-24s%s%s\n' 'plain citation' "$MARKER" "$ID_PLAIN"
        [ -n "$ID_SECTION" ] &&
            printf '  %-24s%s%s\n' 'section (its own line)' "$MARKER" "$ID_SECTION"
        printf '  %-24sas recorded in (%s%s) and nowhere else\n' 'inside prose' "$MARKER" "$ID_OTHER"
        [ -n "$ID_E2E" ] &&
            printf '  %-24s%s%s\n' 'E2E case directory' "$MARKER" "$ID_E2E"
        [ -n "$ID_QUALIFIED" ] &&
            printf '  %-24s%s%s\n' 'workspace-qualified' "$MARKER" "$ID_QUALIFIED"
        printf '\nMust NOT work:\n\n'
        printf '  %-24s%s\n' 'bare id, no marker' "$ID_PLAIN"
        printf '  %-24s%s%s\n' 'unknown id' "$MARKER" "$ID_UNKNOWN"
        if [ "${4:-sandbox}" = sandbox ]; then
            printf '\nCommands in this shell:\n'
            printf '  cites    reprint this sheet          opens   show the open log\n'
            printf '  tui      same sheet, mouse captured  detect  grund integrations\n'
            printf '\nOpens are logged live below (target: %s).\n' \
                "${EDITOR_CMD:-log only — pass --editor 'code --goto' to really open}"
        fi
    }
}

write_sheet() {
    SHEET=$SANDBOX/citations.txt
    sheet_body "$@" >"$SHEET"
}

# `show`: no sandbox, no install, no HOME override — the citations land in the
# terminal you are already in, resolved by whatever you have installed for real.
show_citations() {
    local open="ctrl-click (wezterm) · ctrl+shift+g (kitty) · copy-mode + prefix g (tmux) · click (vscode)"
    local peek="ctrl+shift-click (wezterm) · ctrl+shift+p (kitty) · prefix G (tmux) · hover (vscode)"
    if [ "$TUI" = 1 ]; then
        SHEET=$(mktemp "${TMPDIR:-/tmp}/grund-citations.XXXXXX")
        trap 'rm -f "$SHEET"' EXIT
        sheet_body "your installed clients" "$open" "$peek" plain >"$SHEET"
        GRUND_TESTBED_SHEET=$SHEET
        export GRUND_TESTBED_SHEET
        # Same mouse-capturing pretence the sandbox `tui` helper puts up, so the
        # gestures can be tried the way an agent TUI leaves the terminal.
        printf '\033[?1000h\033[?1002h\033[?1006h'
        trap 'printf "\033[?1006l\033[?1002l\033[?1000l\n"; rm -f "$SHEET"' EXIT INT TERM
        printf 'Mouse reporting is ON — this pane behaves like an agent TUI.\n\n'
        cat "$SHEET"
        printf '\nClick a citation above, then press enter to leave mouse mode. '
        read -r _ || true
    else
        sheet_body "your installed clients" "$open" "$peek" plain
    fi
}

write_rcfile() {
    RCFILE=$SANDBOX/testbedrc
    cat >"$RCFILE" <<EOF
export PS1='grund-testbed:\W\$ '
# Report the directory with OSC 7. WezTerm cannot find the clicked pane's
# directory without it (§FS-integrations.3.1) and no shell emits it by default —
# vte.sh, the usual emitter, skips every terminal that is not VTE. A testbed
# whose shell stayed silent would only ever test the failure path.
__grund_osc7() { printf '\033]7;file://%s%s\033\\\\' "\${HOSTNAME:-localhost}" "\$PWD"; }
PROMPT_COMMAND="__grund_osc7\${PROMPT_COMMAND:+; \$PROMPT_COMMAND}"
export GRUND_TESTBED_SHEET='$SHEET'
export PATH="$SANDBOX/bin:\$PATH"
cites() { cat "\$GRUND_TESTBED_SHEET"; }
opens() { cat "$SANDBOX/opens.log"; }
detect() { grund integrations; }
cites
# Follow the log in the background: a click's resolver runs detached, so this is
# where you see whether it resolved, and to what.
tail -n 0 -f "$SANDBOX/opens.log" &
disown 2>/dev/null || true
EOF
}

# ---------------------------------------------------------------- clients ---

need() { command -v "$1" >/dev/null 2>&1 || die "$1 not found on PATH"; }

# A `wezterm` on PATH that is really `flatpak run …` swallows the environment
# this script sets: the sandbox HOME never crosses into the sandbox, so the
# launch would quietly test the real configuration instead of the testbed's.
is_flatpak_wrapper() {
    local path
    path=$(command -v "$1" 2>/dev/null) || return 1
    grep -qsI 'flatpak run' "$path"
}

launch_wezterm() {
    local config=$SANDBOX/.config/wezterm/wezterm.lua
    [ -f "$config" ] || die "no wezterm config written at $config"
    if [ "$FLATPAK" = 1 ]; then
        need flatpak
        step "launching Flatpak WezTerm"
        # Measured, not assumed: the portal starts the host command from the
        # session environment, not the sandbox's, so nothing set here reaches it.
        say "note: the resolver is re-spawned on the host through flatpak-spawn"
        say "      (§FS-integrations.3.1), which does NOT carry this sandbox's"
        say "      environment across. The host runs your real ~/.local/bin/grund-open"
        say "      with your session's editor, and the open log below stays empty."
        say "      Watch for the editor window instead."
        exec flatpak run \
            --filesystem="$SANDBOX" --filesystem="$FIXTURE" \
            --env=HOME="$SANDBOX" \
            --env=XDG_CONFIG_HOME="$SANDBOX/.config" \
            --env=GRUND_OPEN_CMD="$SANDBOX/bin/grund-open-log" \
            --env=GRUND_TESTBED_LOG="$SANDBOX/opens.log" \
            --env=GRUND_TESTBED_EDITOR="$EDITOR_CMD" \
            org.wezfurlong.wezterm start --always-new-process --cwd "$WORKDIR" \
            -- bash --rcfile "$RCFILE" -i
    fi
    need wezterm
    is_flatpak_wrapper wezterm &&
        die "$(command -v wezterm) is a Flatpak wrapper — re-run with --flatpak, or this
                       launch reads your real config instead of the testbed's"
    step "launching WezTerm"
    exec env "${SANDBOX_ENV[@]}" wezterm --config-file "$config" \
        start --always-new-process --cwd "$WORKDIR" -- bash --rcfile "$RCFILE" -i
}

launch_kitty() {
    need kitty
    step "launching kitty"
    exec env "${SANDBOX_ENV[@]}" kitty \
        --config "$SANDBOX/.config/kitty/kitty.conf" \
        --directory "$WORKDIR" \
        bash --rcfile "$RCFILE" -i
}

launch_tmux() {
    need tmux
    step "launching tmux (socket grund-testbed, in this window)"
    local socket=grund-testbed
    env "${SANDBOX_ENV[@]}" tmux -L "$socket" kill-server 2>/dev/null || true
    # Preload the buffer so `prefix + g` works on the first try; copy-mode
    # selection is the second, more realistic test.
    env "${SANDBOX_ENV[@]}" tmux -L "$socket" -f "$SANDBOX/.tmux.conf" \
        new-session -d -c "$WORKDIR" bash --rcfile "$RCFILE" -i
    env "${SANDBOX_ENV[@]}" tmux -L "$socket" set-buffer "$MARKER$ID_PLAIN"
    exec env "${SANDBOX_ENV[@]}" tmux -L "$socket" attach
}

launch_editor() {
    local client=$1
    local bin=''
    local extensions_dir=''
    case $client in
        vscode) extensions_dir=$SANDBOX/.vscode/extensions ;;
        codium) extensions_dir=$SANDBOX/.vscode-oss/extensions ;;
    esac
    for candidate in $2; do
        command -v "$candidate" >/dev/null 2>&1 && { bin=$candidate; break; }
    done
    [ -n "$bin" ] || die "none of '$2' found on PATH"
    step "launching $bin"
    say "open the integrated terminal, cd into $SUBDIR, and print a citation:"
    say "  printf '%s\\n' '$MARKER$ID_PLAIN'"
    say "hover it for the declaration lead, click it to open (§FS-integrations.3.3)."
    exec env "${SANDBOX_ENV[@]}" "$bin" \
        --extensions-dir "$extensions_dir" \
        --user-data-dir "$SANDBOX/$client-user-data" \
        "$FIXTURE"
}

launch_iterm2() {
    step "iterm2 is a manual client (§FS-integrations.3.4)"
    say "grund never rewrites iTerm2's binary plist, so there is nothing to launch."
    say "The resolver and the user preference are installed in $SANDBOX;"
    say "the Smart Selection rule below is the part only you can apply."
    say ""
    env "${SANDBOX_ENV[@]}" "$GRUND_BIN" integrations iterm2
}

# ---------------------------------------------------------------- doctor ----

doctor() {
    step "binaries"
    for b in wezterm kitty tmux code codium code-insiders flatpak; do
        printf '  %-14s %s\n' "$b" "$(command -v "$b" || echo '—')"
    done
    if command -v tmux >/dev/null 2>&1; then
        local v
        v=$(tmux -V | awk '{print $2}')
        printf '  tmux %s — peek popup needs 3.2+ (%s)\n' "$v" \
            "$(awk -v v="${v%%[a-z]*}" 'BEGIN {print (v+0 >= 3.2) ? "ok" : "prefix+G will be inert"}')"
    fi
    for b in wezterm kitty; do
        is_flatpak_wrapper "$b" &&
            printf '  note: %s is a Flatpak wrapper — use --flatpak for it\n' "$b"
    done
    if command -v flatpak >/dev/null 2>&1; then
        flatpak list --app 2>/dev/null | grep -qi wezterm &&
            printf '  flatpak WezTerm present — test it with --flatpak\n'
    fi
    step "this repository"
    printf '  fixture   %s\n' "$FIXTURE"
    printf '  cwd       %s\n' "$WORKDIR"
    printf '  marker    %s\n' "$(read_marker)"
    printf '  binary    %s\n' "$GRUND_BIN"
    printf '  sandbox   %s\n' "$SANDBOX"
    step "your real environment (untouched by this script)"
    printf '  EDITOR          %s\n' "${EDITOR:-—}"
    printf '  GRUND_OPEN_CMD  %s\n' "${GRUND_OPEN_CMD:-—}"
    printf '  grund           %s\n' "$(command -v grund || echo '—')"
    printf '  grund-open      %s\n' "$(command -v grund-open || echo '—')"
}

# --------------------------------------------------------------- resolve ----

FAILURES=0

# Run the installed resolver exactly as a click does: from the pane's directory,
# with the sandbox HOME, printing the target instead of opening it.
resolve_one() {
    local dir=$1 token=$2
    (cd "$dir" && env "${SANDBOX_ENV[@]}" \
        GRUND_OPEN_CMD="$SANDBOX/bin/grund-open-echo" \
        "$SANDBOX/.local/bin/grund-open" "$token" 2>&1)
}

check() {
    local label=$1 dir=$2 token=$3 expect=$4 out rc=0
    out=$(resolve_one "$dir" "$token") || rc=$?
    local ok=1
    case $expect in
        resolves) [ "$rc" = 0 ] && [ -e "${out%%:*}" ] || ok=0 ;;
        directory) { [ "$rc" = 0 ] && [ -d "$out" ]; } || ok=0 ;;
        fails) [ "$rc" != 0 ] || ok=0 ;;
    esac
    if [ "$ok" = 1 ]; then
        printf '  PASS  %-24s %s\n' "$label" "${out#"$FIXTURE"/}"
    else
        printf '  FAIL  %-24s rc=%s %s\n' "$label" "$rc" "$out"
        FAILURES=$((FAILURES + 1))
    fi
}

resolve_checks() {
    step "resolver checks (from $SUBDIR)"
    check "plain citation" "$WORKDIR" "$MARKER$ID_PLAIN" resolves
    [ -n "$ID_SECTION" ] &&
        check "section citation" "$WORKDIR" "$MARKER$ID_SECTION" resolves
    check "bare id, no marker" "$WORKDIR" "$ID_PLAIN" resolves
    check "swept-in punctuation" "$WORKDIR" "($MARKER$ID_PLAIN" resolves
    check "from repository root" "$FIXTURE" "$MARKER$ID_PLAIN" resolves
    [ -n "$ID_E2E" ] &&
        check "E2E case directory" "$WORKDIR" "$MARKER$ID_E2E" directory
    if [ -n "$ID_QUALIFIED" ]; then
        check "workspace-qualified" "$WORKDIR" "$MARKER$ID_QUALIFIED" resolves
        # The demanding one: from inside a member, the member's own config
        # cannot resolve `<alias>/<ID>`, so the climb has to continue past it
        # to the workspace root instead of stopping at the nearest config.
        [ -n "$MEMBER_DIR" ] &&
            check "qualified from member" "$MEMBER_DIR" "$MARKER$ID_QUALIFIED" resolves
    fi
    check "unknown id" "$WORKDIR" "$MARKER$ID_UNKNOWN" fails
    check "outside any repo" "/" "$MARKER$ID_PLAIN" fails

    step "peek"
    local out rc=0
    out=$( (cd "$WORKDIR" && env "${SANDBOX_ENV[@]}" PAGER=cat \
        "$SANDBOX/.local/bin/grund-open" --peek "$MARKER$ID_PLAIN") 2>&1 ) || rc=$?
    if [ "$rc" = 0 ] && [ -n "$out" ]; then
        printf '  PASS  %-24s %s\n' "--peek" "$(printf '%s\n' "$out" | awk 'NR == 1 { print }')"
    else
        printf '  FAIL  %-24s rc=%s %s\n' "--peek" "$rc" "$out"
        FAILURES=$((FAILURES + 1))
    fi

    # The section citation must land on the section's own line, not on the
    # declaration heading — truncating the suffix is the bug this catches.
    if [ -n "$ID_SECTION" ]; then
        local plain_line section_line
        plain_line=$(resolve_one "$WORKDIR" "$MARKER$ID_PLAIN")
        section_line=$(resolve_one "$WORKDIR" "$MARKER$ID_SECTION")
        step "section suffix"
        if [ "$plain_line" != "$section_line" ]; then
            printf '  PASS  %-24s %s\n' "section != declaration" "${section_line##*/}"
        else
            printf '  FAIL  %-24s both resolve to %s\n' "section != declaration" "$section_line"
            FAILURES=$((FAILURES + 1))
        fi
    fi

    printf '\n'
    if [ "$FAILURES" = 0 ]; then
        say "all resolver checks passed"
    else
        say "$FAILURES check(s) failed"
        exit 1
    fi
}

# ------------------------------------------------------------------ main ----

build_binary

case $command in
    show)
        SANDBOX_ENV=("PATH=$PATH")
        collect_citations
        show_citations
        ;;
    doctor)
        doctor
        ;;
    resolve)
        prepare_sandbox
        write_helpers
        # Any client write installs the resolver; kitty is the cheapest, and
        # its dotfile is inert without kitty running.
        install_client kitty >/dev/null
        collect_citations
        resolve_checks
        ;;
    wezterm|kitty|tmux|vscode|codium|iterm2)
        prepare_sandbox
        write_helpers
        install_client "$command"
        collect_citations
        case $command in
            wezterm) write_sheet wezterm \
                "ctrl-click, or ctrl+shift+g then the label" \
                "ctrl+shift-click, or ctrl+shift+i then the label (split pane)" ;;
            kitty) write_sheet kitty "ctrl+shift+g, then the hint label" "ctrl+shift+p (overlay)" ;;
            tmux) write_sheet tmux "select in copy mode, then prefix + g" "prefix + G (popup, tmux 3.2+)" ;;
            *) write_sheet "$command" "click the link in the integrated terminal" "hover the link" ;;
        esac
        write_rcfile
        say ""
        say "sandbox HOME: $SANDBOX   (your real dotfiles are untouched)"
        say "open log:     $SANDBOX/opens.log"
        case $command in
            wezterm) launch_wezterm ;;
            kitty) launch_kitty ;;
            tmux) launch_tmux ;;
            vscode) launch_editor vscode "code code-insiders" ;;
            codium) launch_editor codium "codium vscodium" ;;
            iterm2) launch_iterm2 ;;
        esac
        ;;
    *)
        echo "try-integrations: unknown command '$command'" >&2
        echo "commands: show, doctor, resolve, wezterm, kitty, tmux, vscode, codium, iterm2" >&2
        exit 2
        ;;
esac
