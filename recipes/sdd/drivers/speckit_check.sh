#!/usr/bin/env bash
# speckit_check.sh — deterministic validation of the design sequence's
# committed spec artifacts. No network, no LLM: structural checks plus
# an optional fold-in of the local `specify` CLI's non-interactive
# check. Reads its prompt file ($1) ONLY to find the result_path line;
# everything else comes from the filesystem (never from prior step
# results). Writes {"result": "designed"|"fail", "notes": ...} to the
# result path — "designed" only when every check passes.
set -u

prompt_file="${1:-}"
if [ -z "$prompt_file" ] || [ ! -f "$prompt_file" ]; then
    echo "speckit_check: prompt file argument missing or unreadable" >&2
    exit 2
fi

# The result contract renders the path as an indented line of its own:
# an absolute path ending in .json. Take the last match (the contract
# section sits at the bottom of the prompt).
result_path=""
while IFS= read -r line; do
    trimmed="${line#"${line%%[![:space:]]*}"}"
    trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
    case "$trimmed" in
        /*.json) result_path="$trimmed" ;;
    esac
done < "$prompt_file"
if [ -z "$result_path" ]; then
    echo "speckit_check: no result_path line found in the prompt file" >&2
    exit 2
fi
mkdir -p "$(dirname "$result_path")" 2>/dev/null || true

findings=""
add_finding() {
    if [ -z "$findings" ]; then findings="$1"; else findings="$findings; $1"; fi
}

# Newest specs/<slug>/ directory by modification time.
newest=""
for d in specs/*/; do
    [ -d "$d" ] || continue
    if [ -z "$newest" ] || [ "$d" -nt "$newest" ]; then newest="$d"; fi
done

ok=1
slug=""
if [ -z "$newest" ]; then
    ok=0
    add_finding "no specs/<slug>/ directory found"
else
    slug="$(basename "$newest")"
    spec_dir="specs/$slug"
    change_dir="openspec/changes/$slug"

    for f in spec.md plan.md tasks.md; do
        if [ ! -s "$spec_dir/$f" ]; then
            ok=0
            add_finding "$spec_dir/$f missing or empty"
        fi
    done
    if [ -s "$spec_dir/spec.md" ] \
        && ! grep -Eiq '^#{1,6}[[:space:]].*acceptance' "$spec_dir/spec.md"; then
        ok=0
        add_finding "$spec_dir/spec.md has no acceptance section heading"
    fi
    if [ -s "$spec_dir/tasks.md" ] \
        && ! grep -Eq '^[[:space:]]*[-*][[:space:]]\[([ xX])\]' "$spec_dir/tasks.md"; then
        ok=0
        add_finding "$spec_dir/tasks.md has no checkbox task item"
    fi
    if [ ! -s "$change_dir/proposal.md" ]; then
        ok=0
        add_finding "$change_dir/proposal.md missing or empty"
    elif ! grep -Eiq '^#{1,6}[[:space:]]*why' "$change_dir/proposal.md"; then
        ok=0
        add_finding "$change_dir/proposal.md has no why section heading"
    fi
fi

# spec-kit CLI fold-in: specify 0.8.7 offers `check` (non-interactive,
# local tool probe — no artifact validator exists in this version).
# Guarded: only when the CLI and the subcommand are present; absence is
# not a failure.
if command -v specify >/dev/null 2>&1; then
    if specify check --help >/dev/null 2>&1 </dev/null; then
        if specify check >/dev/null 2>&1 </dev/null; then
            add_finding "specify check: exit 0"
        else
            ok=0
            add_finding "specify check failed (nonzero exit)"
        fi
    else
        add_finding "specify present but no non-interactive check subcommand; structural checks only"
    fi
else
    add_finding "specify CLI not found; structural checks only"
fi

if [ "$ok" -eq 1 ]; then
    result="designed"
    notes="spec artifacts for '$slug' pass every check ($findings)"
else
    result="fail"
    notes="$findings"
fi
# Sanitize for the JSON string: the pieces are fixed strings plus the
# slug, but a hostile directory name must not break out of the quotes.
notes="$(printf '%s' "$notes" | tr -d '"\\' | tr '\n' ' ')"

printf '{"result": "%s", "notes": "%s"}\n' "$result" "$notes" > "$result_path"
exit 0
