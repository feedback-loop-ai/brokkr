#!/usr/bin/env python3
"""Provider resolution, confined API adapters, and durable Forge handoffs.

The script is intentionally standard-library only. It is shared by the Claude
and Codex Forge adapters; it never imports or executes Claude Workflow files.
"""

from __future__ import annotations

import argparse
import base64
import fcntl
import hashlib
import itertools
import json
import mimetypes
import os
from pathlib import Path, PurePosixPath
import re
import shlex
import signal
import shutil
import subprocess
import sys
import time
import tempfile
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from typing import Any, Iterable


PROVIDER_ALIASES = {
    "alibaba": "qwen",
    "anthropic": "claude",
    "chatgpt": "codex",
    "dashscope": "qwen",
    "modelstudio": "qwen",
    "openai": "codex",
}
PROFILE_BY_PROVIDERS = {
    ("codex",): "codex-only",
    ("claude",): "claude-only",
    ("codex", "qwen"): "codex-qwen",
    ("claude", "qwen"): "claude-qwen",
    ("codex", "claude"): "dual",
    ("codex", "claude", "qwen"): "multimodal",
    ("codex", "claude", "deepseek"): "full",
    ("codex", "claude", "deepseek", "qwen"): "maximal",
}
BASE_PROFILE_BY_RESOLVED = {
    "codex-only": "codex-only",
    "claude-only": "claude-only",
    "codex-qwen": "codex-only",
    "claude-qwen": "claude-only",
    "dual": "dual",
    "multimodal": "dual",
    "full": "full",
    "maximal": "full",
}
CHARTERS = ("simplifier", "systems-architect", "operator")
MULTIMODAL_CHARTER = "experience-architect"
REQUIRED_HANDOFF_FIELDS = {
    "schema",
    "run_id",
    "invocation_id",
    "phase",
    "role",
    "provider",
    "model_requested",
    "model_actual",
    "effort_requested",
    "effort_actual",
    "runner",
    "billing_mode",
    "session_fingerprint",
    "prompt_schema",
    "repository_scope",
    "parents",
    "body_sha256",
    "content_sha256",
    "status",
    "usage",
    "cost",
    "repository_heads",
    "started_at",
    "completed_at",
}


class ForgeControlError(RuntimeError):
    pass


def positive_int(raw: str) -> int:
    value = int(raw)
    if value <= 0:
        raise argparse.ArgumentTypeError("must be greater than zero")
    return value


def json_type_matches(value: Any, expected: str) -> bool:
    """Implement the JSON types used by the checked-in handoff schema."""
    if expected == "null":
        return value is None
    if expected == "object":
        return isinstance(value, dict)
    if expected == "array":
        return isinstance(value, list)
    if expected == "string":
        return isinstance(value, str)
    if expected == "boolean":
        return isinstance(value, bool)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    return False


SUPPORTED_SCHEMA_KEYWORDS = {
    "$defs",
    "$id",
    "$ref",
    "$schema",
    "$comment",
    "additionalProperties",
    "allOf",
    "const",
    "default",
    "description",
    "enum",
    "examples",
    "items",
    "maxItems",
    "minItems",
    "minLength",
    "minimum",
    "pattern",
    "properties",
    "required",
    "title",
    "type",
    "uniqueItems",
}


def resolve_schema_ref(root_schema: dict[str, Any], reference: str) -> dict[str, Any]:
    """Resolve a local JSON Pointer reference from a trusted Forge schema."""
    if not reference.startswith("#/"):
        raise ForgeControlError(f"unsupported non-local JSON Schema reference: {reference}")
    current: Any = root_schema
    for raw in reference[2:].split("/"):
        token = raw.replace("~1", "/").replace("~0", "~")
        if not isinstance(current, dict) or token not in current:
            raise ForgeControlError(f"unresolvable JSON Schema reference: {reference}")
        current = current[token]
    if not isinstance(current, dict):
        raise ForgeControlError(f"JSON Schema reference is not an object: {reference}")
    return current


def schema_definition_errors(
    schema: dict[str, Any],
    root_schema: dict[str, Any] | None = None,
    location: str = "$schema",
    visited: set[int] | None = None,
) -> list[str]:
    """Preflight every reachable subschema before instance-dependent validation."""
    root_schema = schema if root_schema is None else root_schema
    visited = set() if visited is None else visited
    marker = id(schema)
    if marker in visited:
        return []
    visited.add(marker)
    errors: list[str] = []
    unsupported = sorted(set(schema) - SUPPORTED_SCHEMA_KEYWORDS)
    if unsupported:
        errors.append(f"{location}: unsupported JSON Schema keyword(s): {', '.join(unsupported)}")

    reference = schema.get("$ref")
    if "$ref" in schema:
        if not isinstance(reference, str):
            errors.append(f"{location}.$ref: expected a string")
        else:
            try:
                target = resolve_schema_ref(root_schema, reference)
            except ForgeControlError as exc:
                errors.append(f"{location}.$ref: {exc}")
            else:
                errors.extend(
                    schema_definition_errors(
                        target,
                        root_schema,
                        f"{location}.$ref({reference})",
                        visited,
                    )
                )

    for keyword in ("$defs", "properties"):
        children = schema.get(keyword)
        if keyword not in schema:
            continue
        if not isinstance(children, dict):
            errors.append(f"{location}.{keyword}: expected an object of schemas")
            continue
        for name, child in children.items():
            child_location = f"{location}.{keyword}.{name}"
            if not isinstance(child, dict):
                errors.append(f"{child_location}: expected a schema object")
                continue
            errors.extend(
                schema_definition_errors(child, root_schema, child_location, visited)
            )

    item_schema = schema.get("items")
    if "items" in schema:
        if not isinstance(item_schema, dict):
            errors.append(f"{location}.items: expected a schema object")
        else:
            errors.extend(
                schema_definition_errors(
                    item_schema, root_schema, f"{location}.items", visited
                )
            )

    all_of = schema.get("allOf")
    if "allOf" in schema:
        if not isinstance(all_of, list):
            errors.append(f"{location}.allOf: expected an array of schemas")
        else:
            for index, child in enumerate(all_of):
                child_location = f"{location}.allOf[{index}]"
                if not isinstance(child, dict):
                    errors.append(f"{child_location}: expected a schema object")
                    continue
                errors.extend(
                    schema_definition_errors(child, root_schema, child_location, visited)
                )

    additional = schema.get("additionalProperties")
    if "additionalProperties" in schema:
        if isinstance(additional, dict):
            errors.extend(
                schema_definition_errors(
                    additional,
                    root_schema,
                    f"{location}.additionalProperties",
                    visited,
                )
            )
        elif not isinstance(additional, bool):
            errors.append(
                f"{location}.additionalProperties: expected a boolean or schema object"
            )
    return errors


def schema_errors(
    value: Any,
    schema: dict[str, Any],
    location: str = "$",
    root_schema: dict[str, Any] | None = None,
    active_refs: set[tuple[int, str]] | None = None,
) -> list[str]:
    """Validate every JSON Schema keyword supported by Forge, failing closed."""
    initial_call = root_schema is None
    root_schema = schema if root_schema is None else root_schema
    active_refs = set() if active_refs is None else active_refs
    errors = schema_definition_errors(schema, root_schema) if initial_call else []
    reference = schema.get("$ref")
    if "$ref" in schema and not isinstance(reference, str):
        errors.append(f"{location}: $ref must be a string")
    elif isinstance(reference, str):
        marker = (id(value), reference)
        if marker in active_refs:
            errors.append(f"{location}: circular JSON Schema reference: {reference}")
        else:
            active_refs.add(marker)
            try:
                target = resolve_schema_ref(root_schema, reference)
                errors.extend(
                    schema_errors(value, target, location, root_schema, active_refs)
                )
            except ForgeControlError as exc:
                errors.append(f"{location}: {exc}")
            finally:
                active_refs.remove(marker)
    all_of = schema.get("allOf")
    if "allOf" in schema and not isinstance(all_of, list):
        errors.append(f"{location}: allOf must be an array")
    elif isinstance(all_of, list):
        for index, subschema in enumerate(all_of):
            if not isinstance(subschema, dict):
                errors.append(f"{location}.allOf[{index}]: expected a schema object")
                continue
            errors.extend(schema_errors(value, subschema, location, root_schema, active_refs))
    expected = schema.get("type")
    if expected is not None:
        expected_types = [expected] if isinstance(expected, str) else expected
        if not isinstance(expected_types, list) or not all(
            isinstance(item, str) for item in expected_types
        ):
            errors.append(f"{location}: type must be a string or an array of strings")
        elif not any(json_type_matches(value, item) for item in expected_types):
            errors.append(f"{location}: expected {' or '.join(expected_types)}")
            return errors
    if "const" in schema and value != schema["const"]:
        errors.append(f"{location}: must equal {schema['const']!r}")
    if "enum" in schema:
        if not isinstance(schema["enum"], list) or not schema["enum"]:
            errors.append(f"{location}: enum must be a non-empty array")
        elif value not in schema["enum"]:
            errors.append(f"{location}: value is outside the allowed enum")
    if isinstance(value, str):
        min_length = schema.get("minLength", 0)
        if not isinstance(min_length, int) or isinstance(min_length, bool) or min_length < 0:
            errors.append(f"{location}: minLength must be a non-negative integer")
        elif len(value) < min_length:
            errors.append(f"{location}: string is shorter than minLength")
        if "pattern" in schema:
            pattern = schema["pattern"]
            if not isinstance(pattern, str) or len(pattern) > 256 or any(
                token in pattern for token in ("(", ")", "|", "\\1", "\\2")
            ):
                errors.append(f"{location}: unsafe or unsupported regex pattern")
            else:
                try:
                    matches = re.search(pattern, value) is not None
                except re.error:
                    matches = False
                if not matches:
                    errors.append(f"{location}: string does not match the required pattern")
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        if "minimum" in schema:
            minimum = schema["minimum"]
            if not isinstance(minimum, (int, float)) or isinstance(minimum, bool):
                errors.append(f"{location}: minimum must be a number")
            elif value < minimum:
                errors.append(f"{location}: value is below minimum")
    if isinstance(value, list):
        min_items = schema.get("minItems", 0)
        if not isinstance(min_items, int) or isinstance(min_items, bool) or min_items < 0:
            errors.append(f"{location}: minItems must be a non-negative integer")
        elif len(value) < min_items:
            errors.append(f"{location}: array is shorter than minItems")
        if "maxItems" in schema:
            max_items = schema["maxItems"]
            if not isinstance(max_items, int) or isinstance(max_items, bool) or max_items < 0:
                errors.append(f"{location}: maxItems must be a non-negative integer")
            elif len(value) > max_items:
                errors.append(f"{location}: array is longer than maxItems")
        unique_items = schema.get("uniqueItems", False)
        if not isinstance(unique_items, bool):
            errors.append(f"{location}: uniqueItems must be a boolean")
        elif unique_items:
            canonical = [json.dumps(item, sort_keys=True, separators=(",", ":")) for item in value]
            if len(canonical) != len(set(canonical)):
                errors.append(f"{location}: array items must be unique")
        item_schema = schema.get("items")
        if "items" in schema and not isinstance(item_schema, dict):
            errors.append(f"{location}: items must be a schema object")
        elif isinstance(item_schema, dict):
            for index, item in enumerate(value):
                errors.extend(
                    schema_errors(item, item_schema, f"{location}[{index}]", root_schema, active_refs)
                )
    if isinstance(value, dict):
        required_properties = schema.get("required", [])
        if not isinstance(required_properties, list) or not all(
            isinstance(item, str) for item in required_properties
        ):
            errors.append(f"{location}: required must be an array of strings")
            required_properties = []
        for required in required_properties:
            if required not in value:
                errors.append(f"{location}: missing required property {required}")
        properties = schema.get("properties", {})
        if not isinstance(properties, dict):
            errors.append(f"{location}: properties must be an object")
            properties = {}
        additional = schema.get("additionalProperties", True)
        if not isinstance(additional, (bool, dict)):
            errors.append(f"{location}: additionalProperties must be a boolean or schema object")
            additional = False
        for name, item in value.items():
            if name in properties:
                errors.extend(
                    schema_errors(
                        item, properties[name], f"{location}.{name}", root_schema, active_refs
                    )
                )
            elif additional is False:
                errors.append(f"{location}: unexpected property {name}")
            elif isinstance(additional, dict):
                errors.extend(
                    schema_errors(
                        item,
                        additional,
                        f"{location}.{name}",
                        root_schema,
                        active_refs,
                    )
                )
    return errors


def now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def find_workspace(start: Path | None = None) -> Path:
    current = (start or Path.cwd()).resolve()
    if current.is_file():
        current = current.parent
    for candidate in (current, *current.parents):
        if (candidate / "repos.yaml").is_file() and (candidate / ".claude").is_dir():
            return candidate
    raise ForgeControlError("no workspace ancestor contains both repos.yaml and .claude/")


def load_policy(root: Path) -> dict[str, Any]:
    path = root / ".agents/forge/providers.json"
    try:
        return json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise ForgeControlError(f"cannot load provider policy {path}: {exc}") from exc


def parse_scalar(raw: str) -> Any:
    value = raw.split(" #", 1)[0].strip()
    if value in {"true", "false"}:
        return value == "true"
    if value in {"null", "~"}:
        return None
    if len(value) >= 2 and value[0] == value[-1] and value[0] in {"'", '"'}:
        return value[1:-1]
    return value


def load_repositories(root: Path) -> dict[str, dict[str, Any]]:
    """Read only first-level repo fields from the workspace YAML manifest."""
    repos: dict[str, dict[str, Any]] = {}
    current: dict[str, Any] | None = None
    in_repos = False
    for line in (root / "repos.yaml").read_text().splitlines():
        if line == "repos:":
            in_repos = True
            continue
        if not in_repos:
            continue
        match = re.match(r"^  - name:\s*(.+?)\s*$", line)
        if match:
            current = {"name": parse_scalar(match.group(1))}
            repos[str(current["name"])] = current
            continue
        field = re.match(r"^    ([a-zA-Z0-9_-]+):\s*(.*?)\s*$", line)
        if current is not None and field:
            current[field.group(1)] = parse_scalar(field.group(2))
    return repos


def normalize_provider(value: str) -> str:
    normalized = value.strip().lower()
    return PROVIDER_ALIASES.get(normalized, normalized)


def normalize_provider_list(raw: str, order: Iterable[str]) -> tuple[str, ...]:
    if raw in BASE_PROFILE_BY_RESOLVED:
        reverse = {
            "codex-only": ("codex",),
            "claude-only": ("claude",),
            "codex-qwen": ("codex", "qwen"),
            "claude-qwen": ("claude", "qwen"),
            "dual": ("codex", "claude"),
            "multimodal": ("codex", "claude", "qwen"),
            "full": ("codex", "claude", "deepseek"),
            "maximal": ("codex", "claude", "deepseek", "qwen"),
        }
        return reverse[raw]
    requested = {normalize_provider(x) for x in raw.split(",") if x.strip()}
    unknown = requested - set(order)
    if unknown:
        raise ForgeControlError(f"unknown provider(s): {', '.join(sorted(unknown))}")
    return tuple(x for x in order if x in requested)


def command_json(command: list[str]) -> tuple[int, dict[str, Any] | None, str]:
    try:
        result = subprocess.run(command, text=True, capture_output=True, timeout=15, check=False)
    except (OSError, subprocess.TimeoutExpired) as exc:
        return 1, None, str(exc)
    text = result.stdout.strip() or result.stderr.strip()
    try:
        data = json.loads(text) if text else None
    except json.JSONDecodeError:
        data = None
    return result.returncode, data, text


def detect_availability(policy: dict[str, Any], override: str | None = None) -> dict[str, dict[str, Any]]:
    if override is not None:
        selected = {normalize_provider(x) for x in override.split(",") if x.strip()}
        return {
            name: {
                "available": name in selected,
                "auth_method": "test-override" if name in selected else None,
                "billing_mode": "unknown",
            }
            for name in policy["provider_order"]
        }

    status: dict[str, dict[str, Any]] = {}
    codex_ok = False
    codex_method = None
    if shutil.which("codex"):
        try:
            probe = subprocess.run(
                ["codex", "login", "status"], text=True, capture_output=True, timeout=15, check=False
            )
            codex_ok = probe.returncode == 0
            text = (probe.stdout + probe.stderr).lower()
            codex_method = "chatgpt" if "chatgpt" in text else "api" if "api" in text else "authenticated"
        except (OSError, subprocess.TimeoutExpired):
            pass
    status["codex"] = {
        "available": codex_ok,
        "auth_method": codex_method,
        "billing_mode": "subscription" if codex_method == "chatgpt" else "api" if codex_ok else None,
    }

    claude_ok = False
    claude_method = None
    if shutil.which("claude"):
        code, data, _ = command_json(["claude", "auth", "status", "--json"])
        claude_ok = code == 0 and bool(data and data.get("loggedIn"))
        if claude_ok:
            claude_method = str(data.get("authMethod") or "authenticated")
    status["claude"] = {
        "available": claude_ok,
        "auth_method": claude_method,
        "billing_mode": "subscription" if claude_method == "claude.ai" else "api" if claude_ok else None,
    }

    deepseek_env = policy["providers"]["deepseek"]["auth_env"]
    deepseek_ok = (
        bool(os.environ.get(deepseek_env))
        and bool(shutil.which("claude"))
        and bool(shutil.which("bwrap"))
    )
    status["deepseek"] = {
        "available": deepseek_ok,
        "auth_method": "api-key" if deepseek_ok else None,
        "billing_mode": "api" if deepseek_ok else None,
    }
    qwen_env = policy["providers"]["qwen"]["auth_env"]
    qwen_ok = bool(os.environ.get(qwen_env))
    if qwen_ok:
        try:
            modelstudio_base_url(policy["providers"]["qwen"], dict(os.environ))
        except ForgeControlError:
            qwen_ok = False
    status["qwen"] = {
        "available": qwen_ok,
        "auth_method": "api-key" if qwen_ok else None,
        "billing_mode": "api" if qwen_ok else None,
    }
    return status


def composed_profile(policy: dict[str, Any], profile_name: str) -> dict[str, Any]:
    """Overlay optional read-only Qwen routes without duplicating base profiles."""
    base_name = BASE_PROFILE_BY_RESOLVED[profile_name]
    profile = json.loads(json.dumps(policy["profiles"][base_name]))
    providers = list(profile["providers"])
    if profile_name in {"codex-qwen", "claude-qwen", "multimodal", "maximal"}:
        providers.append("qwen")
        profile["council"].append(dict(policy["providers"]["qwen"]["council_route"]))
        profile["routes"]["visual-verifier"].insert(
            0, dict(policy["providers"]["qwen"]["visual_route"])
        )
    profile["providers"] = providers
    return profile


def resolve_profile(
    policy: dict[str, Any], requested: str, availability: dict[str, dict[str, Any]]
) -> dict[str, Any]:
    order = tuple(policy["provider_order"])
    available = tuple(name for name in order if availability[name]["available"])
    if requested == "auto":
        qwen = ("qwen",) if "qwen" in available else ()
        if all(name in available for name in ("codex", "claude", "deepseek")):
            providers = ("codex", "claude", "deepseek") + qwen
        elif all(name in available for name in ("codex", "claude")):
            providers = ("codex", "claude") + qwen
        elif "codex" in available:
            providers = ("codex",) + qwen
        elif "claude" in available:
            providers = ("claude",) + qwen
        else:
            raise ForgeControlError("no trusted Forge provider is authenticated")
    else:
        providers = normalize_provider_list(requested, order)
        if providers not in PROFILE_BY_PROVIDERS:
            raise ForgeControlError(
                "unsupported provider set; Qwen may augment any trusted profile, while DeepSeek "
                "requires codex,claude; neither API provider is a control plane by itself"
            )
        missing = [name for name in providers if not availability[name]["available"]]
        if missing:
            raise ForgeControlError(
                f"explicit provider selection is unavailable: {', '.join(missing)}; no silent downgrade performed"
            )

    profile_name = PROFILE_BY_PROVIDERS[providers]
    ignored = [name for name in available if name not in providers]
    return {
        "requested": requested,
        "resolved": profile_name,
        "providers": list(providers),
        "ignored_available_providers": ignored,
        "availability": {name: availability[name] for name in order},
        "profile": composed_profile(policy, profile_name),
    }


def repo_names(values: Iterable[str]) -> list[str]:
    out: list[str] = []
    for value in values:
        out.extend(part.strip() for part in value.split(",") if part.strip())
    return list(dict.fromkeys(out))


def confined_provider_eligibility(
    policy: dict[str, Any],
    repos: dict[str, dict[str, Any]],
    provider: str,
    role: str,
    selected_repos: list[str],
    context_public: bool,
) -> dict[str, Any]:
    rules = policy["providers"][provider]
    display = "DeepSeek" if provider == "deepseek" else "Qwen"
    reasons: list[str] = []
    if role in rules.get("denied_roles", []):
        reasons.append(f"role '{role}' is explicitly denied for {display}")
    elif role not in rules["allowed_roles"]:
        reasons.append(f"role '{role}' is not {display}-eligible")
    if rules.get("requires_public_context") and not context_public:
        reasons.append("task context was not classified public")
    if not selected_repos:
        reasons.append(f"{display} requires an explicit repository boundary")
    details: list[dict[str, Any]] = []
    for name in selected_repos:
        repo = repos.get(name)
        if repo is None:
            reason = "repository is absent from repos.yaml"
        elif repo.get("archived") is True:
            reason = "repository is archived"
        elif name in rules["denied_repositories"]:
            reason = "repository is explicitly denied"
        elif repo.get("visibility", "unknown") not in rules["allowed_visibility"]:
            reason = f"visibility is {repo.get('visibility', 'unknown')}, not public"
        else:
            reason = None
        details.append({
            "repository": name,
            "visibility": repo.get("visibility", "unknown") if repo else "unknown",
            "eligible": reason is None,
            "reason": reason,
        })
        if reason:
            reasons.append(f"{name}: {reason}")
    return {
        "provider": provider,
        "role": role,
        "eligible": not reasons,
        "context_public": context_public,
        "repositories": details,
        "reasons": reasons,
    }


def deepseek_eligibility(
    policy: dict[str, Any],
    repos: dict[str, dict[str, Any]],
    role: str,
    selected_repos: list[str],
    context_public: bool,
) -> dict[str, Any]:
    return confined_provider_eligibility(
        policy, repos, "deepseek", role, selected_repos, context_public
    )


def qwen_eligibility(
    policy: dict[str, Any],
    repos: dict[str, dict[str, Any]],
    role: str,
    selected_repos: list[str],
    context_public: bool,
) -> dict[str, Any]:
    return confined_provider_eligibility(
        policy, repos, "qwen", role, selected_repos, context_public
    )


def feature_permutation_index(feature_id: str, seat_count: int = 3) -> int:
    permutation_count = 1
    for value in range(2, seat_count + 1):
        permutation_count *= value
    match = re.match(r"^(\d+)", feature_id)
    if match:
        return int(match.group(1)) % permutation_count
    return int(hashlib.sha256(feature_id.encode()).hexdigest()[:8], 16) % permutation_count


def council_route(
    policy: dict[str, Any],
    resolved: dict[str, Any],
    repositories: dict[str, dict[str, Any]],
    feature_id: str,
    selected_repos: list[str],
    context_public: bool,
) -> dict[str, Any]:
    profile_name = resolved["resolved"]
    profile = resolved["profile"]
    downgrade_reasons: list[str] = []
    seats = []
    for seat in profile["council"]:
        provider = seat["provider"]
        if provider in {"deepseek", "qwen"}:
            eligibility = confined_provider_eligibility(
                policy, repositories, provider, "council-seat", selected_repos, context_public
            )
            if not eligibility["eligible"]:
                downgrade_reasons.extend(
                    f"{provider}: {reason}" for reason in eligibility["reasons"]
                )
                continue
        seats.append(seat)
    if len(seats) < 3:
        trusted_profile = "dual" if all(
            provider in resolved["providers"] for provider in ("codex", "claude")
        ) else "codex-only" if "codex" in resolved["providers"] else "claude-only"
        seats = policy["profiles"][trusted_profile]["council"]
    downgrade = None
    if downgrade_reasons:
        downgrade = {
            "from": profile_name,
            "to": f"{len(seats)}-seat-trusted-council",
            "reasons": downgrade_reasons,
        }
    charters = CHARTERS + ((MULTIMODAL_CHARTER,) if len(seats) == 4 else ())
    permutation_index = feature_permutation_index(feature_id, len(seats))
    permutation = list(itertools.permutations(range(len(seats))))[permutation_index]
    assignments = []
    for charter_index, model_index in enumerate(permutation):
        route = dict(seats[model_index])
        route["charter"] = charters[charter_index]
        route["seat_id"] = charters[charter_index]
        assignments.append(route)
    return {
        "feature_id": feature_id,
        "provider_profile": resolved["resolved"],
        "permutation": permutation_index,
        "assignments": assignments,
        "chief": profile["routes"]["chief"][0],
        "council_downgrade": downgrade,
    }


def choose_route(
    policy: dict[str, Any],
    resolved: dict[str, Any],
    repositories: dict[str, dict[str, Any]],
    role: str,
    selected_repos: list[str],
    context_public: bool,
    excluded: set[str],
) -> dict[str, Any]:
    routes = resolved["profile"]["routes"].get(role)
    if not routes:
        raise ForgeControlError(f"profile {resolved['resolved']} has no route for role {role}")
    denials = []
    for route in routes:
        provider = route["provider"]
        if provider in excluded:
            denials.append({"provider": provider, "reason": "explicitly excluded"})
            continue
        if provider not in resolved["providers"]:
            denials.append({"provider": provider, "reason": "not in resolved profile"})
            continue
        if provider in {"deepseek", "qwen"}:
            eligibility = confined_provider_eligibility(
                policy, repositories, provider, role, selected_repos, context_public
            )
            if not eligibility["eligible"]:
                denials.append({"provider": provider, "reason": "; ".join(eligibility["reasons"])})
                continue
        return {
            "profile": resolved["resolved"],
            "role": role,
            "route": route,
            "denials": denials,
        }
    raise ForgeControlError(f"no eligible route for {role}: {json.dumps(denials)}")


def provider_runner_model(policy: dict[str, Any], provider: str, model: str) -> str:
    for profile in policy["profiles"].values():
        routes = list(profile["council"])
        for candidates in profile["routes"].values():
            routes.extend(candidates)
        for route in routes:
            if route["provider"] == provider and route["model"] == model:
                return str(route.get("runner_model") or model)
    return model


def visibility_scope(repositories: dict[str, dict[str, Any]], selected: list[str]) -> str:
    if not selected:
        return "none"
    values = {repositories.get(name, {}).get("visibility", "unknown") for name in selected}
    return next(iter(values)) if len(values) == 1 else "mixed"


def relative_display(root: Path, path: Path) -> str:
    try:
        return str(path.resolve().relative_to(root))
    except ValueError:
        return str(path.resolve())


def session_state_path(root: Path, run_id: str) -> Path:
    safe = re.sub(r"[^a-zA-Z0-9._-]+", "-", run_id)
    return root / ".forge/state" / safe / "sessions.json"


def atomic_replace(path: Path, data: bytes, mode: int = 0o600) -> None:
    """Durably replace a file after fully writing a same-directory temporary."""
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        os.fchmod(descriptor, mode)
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        os.replace(temporary, path)
        directory = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        temporary.unlink(missing_ok=True)


def publish_append_only(path: Path, data: bytes) -> None:
    """Atomically publish immutable content without a check-then-write race."""
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary_name = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    temporary = Path(temporary_name)
    try:
        os.fchmod(descriptor, 0o644)
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(data)
            stream.flush()
            os.fsync(stream.fileno())
        try:
            os.link(temporary, path)
        except FileExistsError:
            if path.read_bytes() != data:
                raise ForgeControlError(f"refusing to overwrite append-only handoff: {path}")
        directory = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(directory)
        finally:
            os.close(directory)
    finally:
        temporary.unlink(missing_ok=True)


def remember_session(root: Path, run_id: str, invocation_id: str, provider: str, session_id: str) -> None:
    path = session_state_path(root, run_id)
    path.parent.mkdir(parents=True, exist_ok=True)
    lock_path = path.with_suffix(".lock")
    with lock_path.open("a+b") as lock:
        os.chmod(lock_path, 0o600)
        fcntl.flock(lock.fileno(), fcntl.LOCK_EX)
        data: dict[str, Any] = {}
        if path.exists():
            try:
                loaded = json.loads(path.read_text())
            except json.JSONDecodeError as exc:
                raise ForgeControlError(f"invalid session state {path}: {exc}") from exc
            if not isinstance(loaded, dict):
                raise ForgeControlError(f"invalid session state {path}: expected an object")
            data = loaded
        data[invocation_id] = {"provider": provider, "session_id": session_id, "updated_at": now()}
        atomic_replace(path, (json.dumps(data, indent=2, sort_keys=True) + "\n").encode())


def normalize_body(path: Path) -> bytes:
    body = path.read_bytes()
    return body if body.endswith(b"\n") else body + b"\n"


def normalize_usage(value: Any) -> dict[str, Any]:
    """Normalize provider-specific token counters into the durable contract."""
    reported = value if isinstance(value, dict) else {}

    def counter(*names: str) -> int | None:
        for name in names:
            amount = reported.get(name)
            if isinstance(amount, int) and not isinstance(amount, bool) and amount >= 0:
                return amount
        return None

    prompt_details = reported.get("prompt_tokens_details")
    if not isinstance(prompt_details, dict):
        prompt_details = reported.get("input_tokens_details")
    prompt_details = prompt_details if isinstance(prompt_details, dict) else {}
    cache_read = counter(
        "cache_read_tokens", "cache_read_input_tokens", "cached_input_tokens"
    )
    if cache_read is None:
        nested = prompt_details.get("cached_tokens")
        cache_read = nested if isinstance(nested, int) and nested >= 0 else None
    return {
        "input_tokens": counter("input_tokens", "prompt_tokens"),
        "output_tokens": counter("output_tokens", "completion_tokens"),
        "cache_read_tokens": cache_read,
        "cache_write_tokens": counter("cache_write_tokens", "cache_creation_input_tokens"),
    }


def handoff_content_hash(metadata: dict[str, Any], body: bytes) -> str:
    """Hash canonical logical frontmatter plus the normalized handoff body."""
    canonical = {key: value for key, value in metadata.items() if key != "content_sha256"}
    encoded = json.dumps(
        canonical, sort_keys=True, separators=(",", ":"), ensure_ascii=False
    ).encode()
    return sha256_bytes(encoded + b"\n" + body)


def load_handoff_schema(root: Path) -> dict[str, Any]:
    candidates = (
        root / ".agents/forge/schemas/handoff.schema.json",
        Path(__file__).resolve().parent.parent / ".agents/forge/schemas/handoff.schema.json",
    )
    for path in candidates:
        if path.is_file():
            try:
                return json.loads(path.read_text())
            except json.JSONDecodeError as exc:
                raise ForgeControlError(f"invalid handoff schema {path}: {exc}") from exc
    raise ForgeControlError("cannot locate handoff.schema.json")


def load_validation_policy(root: Path) -> dict[str, Any]:
    """Load provider policy from a workspace or this script's checkout for tests."""
    candidates = (
        root / ".agents/forge/providers.json",
        Path(__file__).resolve().parent.parent / ".agents/forge/providers.json",
    )
    for path in candidates:
        if path.is_file():
            try:
                return json.loads(path.read_text())
            except json.JSONDecodeError as exc:
                raise ForgeControlError(f"invalid provider policy {path}: {exc}") from exc
    raise ForgeControlError("cannot locate providers.json for handoff validation")


def normalized_repository_scope(metadata: dict[str, Any]) -> tuple[dict[str, Any], list[str]]:
    """Return a crash-safe scope while leaving schema validation to report bad input."""
    raw_scope = metadata.get("repository_scope", {})
    scope = raw_scope if isinstance(raw_scope, dict) else {}
    raw_selected = scope.get("repositories", [])
    selected = (
        raw_selected
        if isinstance(raw_selected, list)
        and all(isinstance(name, str) and bool(name.strip()) for name in raw_selected)
        else []
    )
    return scope, selected


def confined_ancestor_errors(metadata: dict[str, Any], boundary_provider: str) -> list[str]:
    """Require every ancestor consumed by a confined provider to be explicitly public."""
    scope, selected = normalized_repository_scope(metadata)
    if scope.get("visibility") == "public" and selected:
        return []
    display = "DeepSeek" if boundary_provider == "deepseek" else "Qwen"
    return [
        f"{display} confined handoff requires every ancestor to have a non-empty "
        "public repository scope"
    ]


def confined_handoff_errors(
    root: Path,
    repositories: dict[str, dict[str, Any]],
    metadata: dict[str, Any],
) -> list[str]:
    """Apply public-scope and role policy before confined-provider publication."""
    provider = metadata.get("provider")
    if provider not in {"deepseek", "qwen"}:
        return []
    display = "DeepSeek" if provider == "deepseek" else "Qwen"
    scope, selected = normalized_repository_scope(metadata)
    errors: list[str] = []
    if scope.get("visibility") != "public" or not selected:
        errors.append(f"{display} handoff must have a non-empty public repository scope")
    eligibility = confined_provider_eligibility(
        load_validation_policy(root),
        repositories,
        str(provider),
        str(metadata.get("role")),
        selected,
        scope.get("visibility") == "public",
    )
    if not eligibility["eligible"]:
        errors.append(
            f"{display} handoff violates current provider policy: "
            + "; ".join(eligibility["reasons"])
        )
    return errors


def build_handoff(root: Path, repositories: dict[str, dict[str, Any]], ns: argparse.Namespace) -> Path:
    output = Path(ns.output).resolve()
    body = normalize_body(Path(ns.body_file).resolve())
    selected = repo_names(ns.repo)
    parents = []
    for raw in ns.parent:
        parent = Path(raw).resolve()
        if not parent.is_file():
            raise ForgeControlError(f"parent handoff does not exist: {parent}")
        report = verify_handoff(
            root,
            parent,
            confined_boundary=(
                str(ns.provider) if ns.provider in {"deepseek", "qwen"} else None
            ),
        )
        if not report["valid"]:
            raise ForgeControlError(
                f"parent handoff is invalid {parent}: {'; '.join(report['errors'])}"
            )
        parents.append({"path": relative_display(root, parent), "sha256": sha256_file(parent)})
    started = ns.started_at or now()
    completed = ns.completed_at or now()
    session_seed = ns.session_id or f"none:{ns.run_id}:{ns.invocation_id}"
    metadata: dict[str, Any] = {
        "schema": "forge.handoff/v1",
        "run_id": ns.run_id,
        "invocation_id": ns.invocation_id,
        "phase": ns.phase,
        "role": ns.role,
        "provider": ns.provider,
        "model_requested": ns.model_requested,
        "model_actual": ns.model_actual or ns.model_requested,
        "effort_requested": ns.effort,
        "effort_actual": ns.effort_actual,
        "runner": ns.runner,
        "billing_mode": ns.billing_mode,
        "session_fingerprint": f"sha256:{sha256_bytes(session_seed.encode())}",
        "prompt_schema": ns.prompt_schema,
        "repository_scope": {
            "visibility": visibility_scope(repositories, selected),
            "repositories": selected,
        },
        "parents": parents,
        "body_sha256": sha256_bytes(body),
        "status": ns.status,
        "usage": normalize_usage(
            json.loads(Path(ns.usage_file).read_text()) if ns.usage_file else {}
        ),
        "cost": (
            json.loads(Path(ns.cost_file).read_text())
            if ns.cost_file
            else {
                "currency": "USD",
                "amount": 0 if ns.billing_mode == "none" else None,
                "basis": (
                    "deterministic"
                    if ns.billing_mode == "none"
                    else "subscription-no-marginal-price"
                    if ns.billing_mode == "subscription"
                    else "provider-did-not-report"
                ),
                "estimated": False,
            }
        ),
        "repository_heads": dict(item.split("=", 1) for item in ns.repo_head),
        "started_at": started,
        "completed_at": completed,
    }
    metadata["content_sha256"] = handoff_content_hash(metadata, body)
    rendered = b"---\n" + json.dumps(metadata, indent=2, sort_keys=True).encode() + b"\n---\n" + body
    schema = load_handoff_schema(root)
    errors = schema_errors(metadata, schema)
    errors.extend(confined_handoff_errors(root, repositories, metadata))
    if errors:
        raise ForgeControlError(f"refusing to publish invalid handoff: {'; '.join(errors)}")
    publish_append_only(output, rendered)
    if ns.session_id:
        remember_session(root, ns.run_id, ns.invocation_id, ns.provider, ns.session_id)
    return output


def parse_handoff(path: Path) -> tuple[dict[str, Any], bytes]:
    data = path.read_bytes()
    if not data.startswith(b"---\n"):
        raise ForgeControlError(f"{path}: missing JSON frontmatter")
    marker = data.find(b"\n---\n", 4)
    if marker < 0:
        raise ForgeControlError(f"{path}: unterminated JSON frontmatter")
    try:
        metadata = json.loads(data[4:marker].decode())
    except json.JSONDecodeError as exc:
        raise ForgeControlError(f"{path}: invalid JSON frontmatter: {exc}") from exc
    return metadata, data[marker + 5 :]


def verify_handoff(
    root: Path,
    path: Path,
    visiting: set[Path] | None = None,
    cache: dict[tuple[Path, str | None], dict[str, Any]] | None = None,
    confined_boundary: str | None = None,
) -> dict[str, Any]:
    """Validate one complete handoff and every ancestor with cycle detection."""
    resolved_path = path.resolve()
    visiting = set() if visiting is None else visiting
    cache = {} if cache is None else cache
    cache_key = (resolved_path, confined_boundary)
    if cache_key in cache:
        return cache[cache_key]
    if resolved_path in visiting:
        return {
            "path": relative_display(root, resolved_path),
            "valid": False,
            "errors": [f"parent cycle detected at {relative_display(root, resolved_path)}"],
            "metadata": {},
        }
    visiting.add(resolved_path)
    try:
        metadata, body = parse_handoff(resolved_path)
        errors = schema_errors(metadata, load_handoff_schema(root))
        missing = sorted(REQUIRED_HANDOFF_FIELDS - metadata.keys())
        errors.extend(f"missing field: {field}" for field in missing)
        if metadata.get("body_sha256") != sha256_bytes(body):
            errors.append("body_sha256 mismatch")
        if metadata.get("content_sha256") != handoff_content_hash(metadata, body):
            errors.append("content_sha256 mismatch")
        if confined_boundary is not None:
            errors.extend(confined_ancestor_errors(metadata, confined_boundary))
        if (root / "repos.yaml").is_file():
            verification_repositories = load_repositories(root)
        else:
            scope, selected = normalized_repository_scope(metadata)
            verification_repositories = {
                str(name): {"name": str(name), "visibility": scope.get("visibility", "unknown")}
                for name in selected
            }
        errors.extend(confined_handoff_errors(root, verification_repositories, metadata))
        provider = metadata.get("provider")
        next_confined_boundary = confined_boundary
        if next_confined_boundary is None and provider in {"deepseek", "qwen"}:
            next_confined_boundary = str(provider)
        parents = metadata.get("parents", [])
        if isinstance(parents, list):
            for parent in parents:
                if not isinstance(parent, dict):
                    continue
                parent_path = Path(str(parent.get("path", "")))
                if not parent_path.is_absolute():
                    parent_path = root / parent_path
                parent_path = parent_path.resolve()
                if not parent_path.is_file():
                    errors.append(f"missing parent: {parent.get('path')}")
                    continue
                if sha256_file(parent_path) != parent.get("sha256"):
                    errors.append(f"parent hash mismatch: {parent.get('path')}")
                parent_report = verify_handoff(
                    root,
                    parent_path,
                    visiting,
                    cache,
                    next_confined_boundary,
                )
                if not parent_report["valid"]:
                    errors.extend(
                        f"invalid parent {parent.get('path')}: {error}"
                        for error in parent_report["errors"]
                    )
        report = {
            "path": relative_display(root, resolved_path),
            "valid": not errors,
            "errors": errors,
            "metadata": metadata,
        }
        cache[cache_key] = report
        return report
    finally:
        visiting.remove(resolved_path)


def blind_handoff(root: Path, path: Path) -> dict[str, Any]:
    report = verify_handoff(root, path)
    if not report["valid"]:
        raise ForgeControlError(f"cannot blind invalid handoff {path}: {'; '.join(report['errors'])}")
    metadata, body = parse_handoff(path)
    return {
        "schema": "forge.blind-handoff/v1",
        "role": metadata["role"],
        "body_sha256": metadata["body_sha256"],
        "content": body.decode(),
    }


def numeric_usage(value: Any, prefix: str = "") -> dict[str, float]:
    totals: dict[str, float] = {}
    if isinstance(value, dict):
        for key, child in value.items():
            child_prefix = f"{prefix}.{key}" if prefix else str(key)
            for name, amount in numeric_usage(child, child_prefix).items():
                totals[name] = totals.get(name, 0) + amount
    elif isinstance(value, (int, float)) and not isinstance(value, bool):
        totals[prefix] = float(value)
    return totals


SEVERITY_ORDER = ("none", "info", "low", "medium", "high", "critical")
#: Fallback base for signature checks when a feature declares none.
DEFAULT_BASE_BRANCH = "develop"
#: Condition vocabulary the transition table may use. Anything else is a
#: malformed rule and must fail loudly rather than silently never matching.
KNOWN_CONDITION_KEYS = frozenset(
    {
        "consecutive_failures_gte",
        "max_residual_severity_above",
        "skip_verify",
        "fixes_applied",
        "has_security_residual",
        "high_risk_uncovered",
        "drift_detected",
        "dirty_worktrees",
    }
)


#: Outward-facing, effectively irreversible actions the phase machine governs.
#: One definition for every runtime: a duplicated regex drifted, and the weakest
#: runtime then set the effective policy.
#: (executable, required subcommand tokens) for each governed action.
GOVERNED_ACTIONS = (
    ("git", ("push",)),
    ("gh", ("pr", "create")),
)

#: Shell operators that begin a NEW command within one string.
COMMAND_SEPARATORS = {";", "&&", "||", "|", "&", "(", ")", "{", "}"}


def split_commands(argv: list[str]) -> list[list[str]]:
    """Split a tokenised line into the individual commands it runs."""
    commands: list[list[str]] = [[]]
    for token in argv:
        if token in COMMAND_SEPARATORS:
            commands.append([])
        else:
            commands[-1].append(token)
    return [command for command in commands if command]


def is_governed(command: str) -> dict[str, Any]:
    """Whether a shell command performs a governed action.

    Deliberately over-matches: a false deny is visible and annoying, a false
    allow is a silent bypass of a safety control. Command-string matching is a
    convenience layer only -- the authoritative gate is the git pre-push hook
    installed in every feature worktree, which sees real refs no matter how the
    push was spelled.
    """
    try:
        argv = shlex.split(command, comments=False)
    except ValueError:
        # Unbalanced quotes: we cannot know what this runs, so treat it as
        # governed. Fail loud, per the over-match posture above.
        return {"command": command, "governed": True, "patterns": ["unparseable"]}

    matched: list[str] = []
    for parts in split_commands(argv):
        if not parts:
            continue
        # `git` may be reached as /usr/bin/git; the basename is what matters.
        executable = PurePosixPath(parts[0]).name
        for name, required in GOVERNED_ACTIONS:
            if executable != name:
                continue
            # Every required token must appear as its OWN argv element. This is
            # what regex-on-stripped-text could not do: `git "push"` tokenises
            # to ["git", "push"] and is caught, while `git commit -m "mention
            # push"` keeps the message as one element and is not.
            rest = parts[1:]
            if all(token in rest for token in required):
                matched.append(f"{name} {' '.join(required)}")
    return {"command": command, "governed": bool(matched), "patterns": sorted(set(matched))}


def load_phase_machine(root: Path) -> dict[str, Any]:
    path = root / ".agents/forge/phase-machine.json"
    try:
        return json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise ForgeControlError(f"cannot load phase machine {path}: {exc}") from exc


def spec_forge_dir(spec_dir: Path) -> Path:
    return spec_dir.resolve() / "forge"


def resolve_spec_dir(root: Path, raw: str, feature_id: str | None, *, must_exist: bool = True) -> Path:
    """Bind phase state to a feature's canonical directory under the workspace.

    State was previously keyed by a caller-supplied path, so an open security
    hold could be sidestepped by advancing a second directory under the same
    feature id, or by symlinking specs/<id> at a journal the caller authored.
    Identity, not the argument, decides where state lives.

    Test/tmp directories outside the workspace are still allowed (the suite and
    ad-hoc probes rely on it), but a path *inside* the workspace must be the
    real specs/<feature-id>, and no component may be a symlink.
    """
    candidate = Path(raw)
    resolved = candidate.resolve()
    specs_root = (root / "specs").resolve()

    # Check for symlinks on the *literal* path first: a symlink under specs/
    # pointing outside the workspace resolves away from specs/ entirely, so an
    # "is it inside?" test alone would wave it through.
    probe = candidate if candidate.is_absolute() else (root / candidate)
    literal = probe if probe.is_absolute() else probe.absolute()
    if str(literal).startswith(str(specs_root)):
        for part in (literal, *literal.parents):
            if part == specs_root:
                break
            if part.is_symlink():
                raise ForgeControlError(
                    f"refusing symlinked spec path: {relative_display(root, part)}"
                )

    inside = resolved == specs_root or specs_root in resolved.parents
    claimed = feature_id or resolved.name

    if inside:
        expected = (specs_root / claimed).resolve()
        if resolved != expected:
            raise ForgeControlError(
                f"spec directory {relative_display(root, resolved)} does not match feature "
                f"'{claimed}'; phase state must live in specs/{claimed}"
            )
    if must_exist and not resolved.is_dir():
        raise ForgeControlError(f"spec directory does not exist: {resolved}")
    return resolved


def foreign_feature_blockers(root: Path, feature_id: str, spec_dir: Path) -> list[dict[str, Any]]:
    """Open hard blockers recorded for this feature in any *other* journal.

    Defence in depth for sec-forge-2: even if a second ledger is somehow
    created for the same feature, its holds remain visible.
    """
    found: list[dict[str, Any]] = []
    specs_root = root / "specs"
    if not specs_root.is_dir():
        return found
    for journal in sorted(specs_root.glob("*/forge/events.jsonl")):
        other = journal.parent.parent.resolve()
        if other == spec_dir.resolve():
            continue
        try:
            events = read_events(other)
        except ForgeControlError:
            continue
        if not any(event.get("feature_id") == feature_id for event in events):
            continue
        for blocker in journal_blockers(events):
            blocker = dict(blocker)
            blocker["journal"] = relative_display(root, other)
            found.append(blocker)
    return found


def chain_hash(previous: str, payload: dict[str, Any]) -> str:
    """Link one journal entry to its predecessor."""
    body = json.dumps({k: v for k, v in payload.items() if k != "prev"}, sort_keys=True)
    return sha256_bytes((previous + body).encode())


# The full set `git rev-parse --local-env-vars` reports: every variable git
# itself considers repo-local, not just the two (GIT_DIR/GIT_WORK_TREE) that
# first surfaced the bug. GIT_CONFIG can redirect config resolution to another
# repo's file; GIT_COMMON_DIR and GIT_ALTERNATE_OBJECT_DIRECTORIES can point
# object/ref lookups at a different repository's store. Any of these inherited
# from the pushing repo's hook environment would let that repo's state leak
# into (or override) an observation `-C` was told to make elsewhere.
_GIT_LOCAL_ENV_VARS = (
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_CONFIG",
    "GIT_CONFIG_PARAMETERS",
    "GIT_CONFIG_COUNT",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
)


def git_env() -> dict[str, str]:
    """Environment with git's per-repo variables removed.

    git exports GIT_DIR/GIT_WORK_TREE to hooks, and an exported GIT_DIR
    OVERRIDES `-C`. Inheriting it makes every observation here read the pushing
    repo instead of the path we named — surfacing as a missing anchor ref or
    phantom drift, but only when invoked from the pre-push hook, which is the
    one place the gate actually runs. Every local Git subprocess in this file
    must pass `env=git_env()`; a variable left off this list is a variable
    that can still redirect an observation.
    """
    return {k: v for k, v in os.environ.items() if k not in _GIT_LOCAL_ENV_VARS}


def git_out(root: Path, args: list[str], stdin: bytes | None = None) -> str | None:
    """Run a git plumbing command in the workspace repo; None on any failure."""
    try:
        result = subprocess.run(
            ["git", "-C", str(root), *args],
            input=stdin,
            capture_output=True,
            timeout=30,
            check=False,
            env=git_env(),
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    return result.stdout.decode().strip()


def forge_ref(feature_id: str) -> str:
    """Injective, git-legal ref name for a feature.

    The sanitising regex alone was many-to-one -- '028/x' and '028-x' both
    mapped to refs/forge/028-x, so two features shared one chain and each
    invalidated the other. Ids containing '..' or ending '.lock' also produced
    names git rejects, which silently disabled anchoring. A short digest of the
    raw id restores injectivity; the trailing separator keeps '.lock' from ever
    terminating the name.
    """
    safe = re.sub(r"[^a-zA-Z0-9._-]+", "-", feature_id).strip("-.") or "feature"
    safe = re.sub(r"\.\.+", ".", safe)
    digest = sha256_bytes(feature_id.encode())[:8]
    return f"refs/forge/{safe}-{digest}"


def superseded_ref(feature_id: str, when: str) -> str:
    """Where a repair parks the chain it is replacing.

    Two defects lived in the old fixed `refs/forge-superseded/<raw-id>`: it used
    the RAW id, sidestepping the injective sanitising that forge_ref exists to
    do (so ids git rejects silently lost the evidence instead of parking it),
    and a second repair overwrote the first -- destroying exactly the chain an
    investigator would want, since repeated repairs are the suspicious case.

    The timestamp is for HUMANS reading `git for-each-ref`: it has 1-second
    resolution, so two repairs in one second collide. The caller's suffix walk
    is what actually guarantees no chain is overwritten -- do not remove it on
    the assumption that this name is already unique.
    """
    base = forge_ref(feature_id).split("/", 2)[-1]
    stamp = re.sub(r"[^0-9]", "", when)[:14] or "0"
    return f"refs/forge-superseded/{base}-{stamp}"


def journal_feature_id(spec_dir: Path, fallback: str | None = None) -> str:
    """The feature id this journal belongs to, taken from the journal itself.

    phase-clear and phase-attest have no --feature-id, so deriving it from the
    directory name split one feature across two refs: the phase transitions
    anchored under the real id, the clearances under the directory name.
    """
    for event in read_events(spec_dir, verify=False):
        if event.get("feature_id"):
            return str(event["feature_id"])
    return fallback or spec_dir.name


def anchor_transition(
    root: Path,
    spec_dir: Path,
    feature_id: str,
    event: dict[str, Any],
    repos: list[str],
    *,
    entry_index: int | None = None,
) -> dict[str, Any] | None:
    """Commit one phase transition to refs/forge/<feature-id>.

    A git commit object is content-addressed, so a chain of them is a far
    stronger tamper anchor than journal-head.json -- an ordinary writable file
    sitting next to the journal it was meant to protect.

    Built entirely with plumbing (hash-object / mktree / commit-tree /
    update-ref): the index, the working tree, and the checked-out branch are
    never touched, so this cannot disturb work in progress. Dates and identity
    are pinned, so replaying the same run reproduces the same SHAs.

    Best-effort by contract: the JSON journal remains authoritative for gating,
    and a failure here must never wedge an unattended run.
    """
    journal = spec_forge_dir(spec_dir) / "events.jsonl"
    if not journal.is_file():
        return None

    payload = {
        "schema": "forge.transition-anchor/v1",
        "feature_id": feature_id,
        "transition": {
            "from": event.get("from"),
            "result": event.get("result"),
            "rule": event.get("rule"),
            "next": event.get("next"),
            "severity": event.get("severity"),
            "kind": event.get("kind"),
        },
        # The journal's own hash: forging or truncating events.jsonl no longer
        # matches what this immutable commit recorded.
        "journal_sha256": sha256_file(journal),
        # The 1-based position of the entry this commit anchors. During a
        # rebuild the journal is already complete, so counting it would stamp
        # every commit with the final length -- indistinguishable from padding.
        "journal_entries": (
            entry_index
            if entry_index is not None
            else len(read_events(spec_dir, verify=False))
        ),
        # Code lives in sibling repos, so a single commit cannot contain it --
        # but it can pin it. Taken from the event the journal already recorded,
        # NOT re-observed: re-reading the worktrees here made the anchor
        # disagree with the entry it claims to anchor, and broke replay.
        "repo_heads": event.get("heads") or observe_heads(root, feature_id, repos),
        "at": event.get("at"),
    }
    body = (json.dumps(payload, indent=2, sort_keys=True) + "\n").encode()

    blob = git_out(root, ["hash-object", "-w", "--stdin"], stdin=body)
    state_file = state_path(spec_dir)
    state_blob = (
        git_out(root, ["hash-object", "-w", "--stdin"], stdin=state_file.read_bytes())
        if state_file.is_file()
        else None
    )
    journal_blob = git_out(root, ["hash-object", "-w", "--stdin"], stdin=journal.read_bytes())
    if not blob or not journal_blob:
        return None

    entries = [f"100644 blob {blob}\ttransition.json", f"100644 blob {journal_blob}\tevents.jsonl"]
    if state_blob:
        entries.append(f"100644 blob {state_blob}\tstate.json")
    tree = git_out(root, ["mktree"], stdin=("\n".join(sorted(entries)) + "\n").encode())
    if not tree:
        return None

    ref = forge_ref(feature_id)
    parent = git_out(root, ["rev-parse", "--verify", "--quiet", ref])
    stamp = str(event.get("at") or now())
    identity = {
        "GIT_AUTHOR_NAME": "forge-phase-machine",
        "GIT_AUTHOR_EMAIL": "forge@alkem.io",
        "GIT_COMMITTER_NAME": "forge-phase-machine",
        "GIT_COMMITTER_EMAIL": "forge@alkem.io",
        "GIT_AUTHOR_DATE": stamp,
        "GIT_COMMITTER_DATE": stamp,
    }
    if event.get("kind"):
        subject = f"forge({feature_id}): {event['kind']}"
    else:
        subject = (
            f"forge({feature_id}): {event.get('from')} -> {event.get('next')} "
            f"[{event.get('rule')}]"
        )
    args = ["commit-tree", tree, "-m", subject]
    if parent:
        args += ["-p", parent]
    # Signing is what would make the anchor a control rather than a log. It is
    # opt-in AND best-effort: `commit-tree -S` blocks on pinentry, which cannot
    # prompt from this non-interactive subprocess, so an unattended run would
    # hang until the timeout and then anchor nothing at all. Signing therefore
    # only applies when the caller has arranged a non-interactive agent (a
    # cached passphrase or a loopback pinentry); on failure we fall back to an
    # unsigned anchor, which is strictly better than no anchor.
    sign = os.environ.get("FORGE_SIGN_ANCHORS") == "1"

    # Set when signing failed by running out of budget rather than by pinentry
    # being unavailable. The distinction decides whether the failure may be
    # cached: a timeout means nobody touched the key in time, which is
    # transient and per-transition. Caching it let FORGE_SIGN_TIMEOUT=5 -- an
    # ungoverned env var -- force one timeout and silently disable signing for
    # the next 900s, i.e. downgrade the run through a value an agent controls.
    timed_out = False

    def build(signed: bool) -> subprocess.CompletedProcess[bytes] | None:
        nonlocal timed_out
        try:
            return subprocess.run(
                ["git", "-C", str(root), *args, *(["-S"] if signed else [])],
                capture_output=True,
                timeout=signing_timeout() if signed else 30,
                check=False,
                env={**git_env(), **identity},
            )
        except subprocess.TimeoutExpired:
            if signed:
                timed_out = True
            return None
        except OSError:
            return None

    # The probe result is cached OUTSIDE the spec dir, keyed by the ambient gpg
    # environment. An agent-writable sentinel next to the journal could be
    # pre-planted to silently downgrade a run the operator explicitly asked to
    # sign -- the same absent-evidence-as-settled-negative pattern this feature
    # has now hit four times.
    probe = signing_probe_path(root)
    # The probe is agent-writable (it must live outside the spec dir, but no
    # location in the workspace is beyond an agent with shell access). An
    # operator physically present at a tty who asked for signing outranks it:
    # otherwise a pre-planted sentinel silently downgrades exactly the run the
    # human is there to authorise.
    # DIRECTION-OF-EFFECT RULE for isatty(), the primitive two operator gates
    # were wrongly built on: isatty() may only ever make a run STRICTER, never
    # grant anything. Here a forged True (an agent under a pty) merely causes
    # signing to be ATTEMPTED despite a fresh negative probe -- it costs the
    # run a bounded timeout and falls back to the same unsigned anchor. No
    # value of `attended` can make an unsigned chain read as signed; that rests
    # on verification against the committed pin. Contrast phase_attest and
    # phase_reanchor, where a forged True skipped an authorisation check
    # entirely. If a future change makes this value grant something, it is the
    # same bug again and must be removed.
    attended = sys.stdin.isatty() or sys.stderr.isatty()
    if sign and not attended and signing_probe_fresh(root):
        sign = False
    if sign:
        # Tell the operator a signature is wanted, and where the prompt will
        # appear. An agent-run process has no tty, so a curses pinentry renders
        # into whatever pty gpg-agent last saw -- often a different pane. Left
        # unsaid, the run just stalls until the budget expires and falls back,
        # which is why signing "never worked" before this was understood.
        hint = ""
        if not sys.stderr.isatty():
            hint = " (no tty here: the PIN prompt appears in the terminal that started gpg-agent)"
        print(
            f"[forge] waiting up to {signing_timeout()}s for a signature — "
            f"touch your key{hint}. FORGE_SIGN_TIMEOUT changes the budget; "
            "unset FORGE_SIGN_ANCHORS to skip signing.",
            file=sys.stderr,
            flush=True,
        )
    made = build(sign)
    if sign and (made is None or made.returncode != 0):
        # Signing unavailable non-interactively (pinentry cannot prompt from a
        # subprocess). Record it out-of-tree and fall back: an unsigned anchor
        # beats no anchor, and anchor_signing_mode() reports the truth.
        if not timed_out:
            try:
                probe.parent.mkdir(parents=True, exist_ok=True)
                probe.write_text(f"pinentry unavailable at {now()}\n")
                probe.chmod(0o600)
            except OSError:
                pass
        made = build(False)
    if made is None or made.returncode != 0:
        return None
    commit = made.stdout.decode().strip()
    # Compare-and-swap on the parent we actually read. A plain `update-ref`
    # lets a stale writer overwrite a concurrent one, orphaning its commit and
    # desynchronising chain depth from journal length -- which then wedges the
    # ship gate. Passing the old value makes git reject the race instead.
    swap = ["update-ref", ref, commit, parent] if parent else ["update-ref", ref, commit, ""]
    if git_out(root, swap) is None:
        return None
    if git_out(root, ["rev-parse", "--verify", "--quiet", ref]) != commit:
        return None
    return {"ref": ref, "commit": commit, "parent": parent}


def verify_chain_contents(
    root: Path, spec_dir: Path, head: str, events: list[dict[str, Any]], gaps: int
) -> list[str]:
    """Check each anchor commit against the journal entry it claims to record.

    Comparing only the COUNT of commits was the load-bearing defect of round 5:
    six copies of one anchor passed as a six-transition chain, so a forged
    journal plus duplicate anchors shipped a security-held feature with zero
    reasons. Depth equality must be a CONSEQUENCE of per-entry verification,
    never a substitute for it.
    """
    problems: list[str] = []
    listing = git_out(root, ["rev-list", "--reverse", head])
    if listing is None:
        return ["anchor chain could not be read"]
    commits = [line.strip() for line in listing.splitlines() if line.strip()]

    if len(commits) + gaps != len(events):
        return [
            f"anchor chain has {len(commits)} commits for {len(events)} journal "
            f"entries ({gaps} recorded anchoring failure(s)) -- the ref was "
            "rewritten or restarted"
        ]

    # Each anchor records the journal AS IT STOOD when written, so entry N's
    # anchor must claim exactly N entries. Duplicates and reordering break this.
    seen: set[int] = set()
    for position, commit in enumerate(commits, start=1):
        body = git_out(root, ["show", f"{commit}:transition.json"])
        if not body:
            problems.append(f"anchor {commit[:12]} carries no transition record")
            continue
        try:
            payload = json.loads(body)
        except json.JSONDecodeError:
            problems.append(f"anchor {commit[:12]} carries an unreadable record")
            continue
        claimed = payload.get("journal_entries")
        if not isinstance(claimed, int):
            problems.append(f"anchor {commit[:12]} records no journal length")
            continue
        if claimed in seen:
            problems.append(
                f"anchor {commit[:12]} duplicates journal length {claimed} -- "
                "the chain was padded with repeated commits"
            )
            continue
        seen.add(claimed)
        if not 1 <= claimed <= len(events):
            problems.append(
                f"anchor {commit[:12]} records journal length {claimed}, outside "
                f"the journal's {len(events)} entries"
            )
            continue
        # Compare against the entry that anchor claims to be the newest.
        entry = events[claimed - 1]
        recorded = payload.get("transition") or {}
        if entry.get("kind"):
            expected = {"kind": entry.get("kind")}
            actual = {"kind": recorded.get("kind")}
        else:
            fields = ("from", "result", "rule", "next")
            expected = {field: entry.get(field) for field in fields}
            actual = {field: recorded.get(field) for field in fields}
        if expected != actual:
            problems.append(
                f"anchor {commit[:12]} records {actual} but journal entry "
                f"{claimed} is {expected} -- the chain does not match the journal"
            )
        if payload.get("at") != entry.get("at"):
            problems.append(
                f"anchor {commit[:12]} timestamp disagrees with journal entry {claimed}"
            )
        # With no recorded gaps every entry is anchored, so a commit's position
        # in the chain must equal the entry it records. Gaps shift later
        # positions, so only monotonicity is checkable then.
        if not gaps and position != claimed:
            problems.append(
                f"anchor {commit[:12]} is at position {position} but records "
                f"journal entry {claimed} -- the chain is out of order"
            )
    # Gaps are NOT a blocking problem. Anchoring is best-effort by contract
    # (spec.md, record_anchor_gap's docstring, and ship_gate's own comment all
    # say so), and blocking here contradicted all three: one transient git
    # failure -- a full disk, a concurrent gc -- permanently wedged a finished
    # feature. Worse, the only escape was phase-reanchor, whose operator gate
    # is an agent-passable flag, so the block actively trained callers into
    # running the one command that can launder a forged journal.
    #
    # The honest signal is the `anchor_gaps` counter ship_gate already returns.
    return problems


def verify_transition_anchor(root: Path, spec_dir: Path, feature_id: str) -> list[str]:
    """Compare the journal against the immutable ref commit that recorded it."""
    journal_path = spec_forge_dir(spec_dir) / "events.jsonl"
    ref = forge_ref(feature_id)
    head = git_out(root, ["rev-parse", "--verify", "--quiet", ref])
    if not head:
        # FAIL CLOSED. Returning [] here meant deleting the ref discarded the
        # whole check: a forged-but-internally-consistent journal shipped a
        # security-held feature. An absent anchor for an existing journal is
        # tampering (or an anchoring failure), never a pass.
        if journal_path.is_file():
            return [
                f"no transition anchor at {ref} for a journal with "
                f"{len(read_events(spec_dir, verify=False))} entries -- the ref was deleted "
                "or anchoring never ran; this run cannot be evidenced"
            ]
        return []
    recorded = git_out(root, ["show", f"{head}:transition.json"])
    if not recorded:
        return [f"{ref} exists but carries no transition record"]
    try:
        payload = json.loads(recorded)
    except json.JSONDecodeError:
        return [f"{ref} carries an unreadable transition record"]

    if not journal_path.is_file():
        return [f"journal is missing but {ref} records {payload.get('journal_entries')} entries"]

    problems: list[str] = []
    if payload.get("journal_sha256") != sha256_file(journal_path):
        problems.append(
            f"journal does not match the commit anchor {head[:12]} -- it was edited or truncated"
        )

    # The head alone is not enough: a chain can be shortened, or restarted with
    # no parent, while its newest commit still matches a doctored journal.
    journal_events = read_events(spec_dir, verify=False)
    entries = len(journal_events)
    if payload.get("journal_entries") != entries:
        problems.append(
            f"anchor {head[:12]} records {payload.get('journal_entries')} journal entries, "
            f"found {entries}"
        )
    # Anchoring is best-effort, so the chain can legitimately fall behind the
    # journal when a write fails. Those gaps are RECORDED, so a shortfall that
    # exactly matches them is a known write failure, not tampering. Without
    # this, one transient failure wedged the feature forever -- and the message
    # blamed tampering, whose only rational response is to bypass the gate.
    gaps = anchor_gaps(spec_dir)
    problems.extend(verify_chain_contents(root, spec_dir, head, journal_events, gaps))

    # Signature state is deliberately NOT a problem here. Signing is opt-in per
    # run, so an unsigned chain must ship; and a partially-signed one (the key
    # went away mid-run) is no worse than none. Treating either as blocking
    # wedged runs and prescribed a repair that did not exist. The honest
    # reporting lives in anchor_signing_mode(), which ship_gate surfaces as
    # anchor_mode so a ledger never overstates the evidence it holds.
    return problems


def expected_signing_keys(root: Path) -> set[str]:
    """Fingerprints allowed to sign anchors.

    `%G?` == "G" only means *a* key in the local trustdb signed it -- and an
    agent can generate a passphrase-less key and mark it trusted in seconds
    (verified). Without pinning, self-signed forgeries would report
    `anchor_mode: signed`, which is worse than unsigned because it reads as
    authoritative.

    The allowlist is read from the COMMITTED object, not the working tree: a
    tracked file is still agent-writable on disk, and reading it from disk let
    an agent append its own fingerprint and obtain `anchor_mode: signed`
    without ever committing. Reading `HEAD:<path>` means the pin changes only
    through a commit, which a reviewer sees.

    Entries must be full 40-hex fingerprints. A short or malformed entry would
    otherwise widen the pin into a wildcard via suffix matching, so anything
    else is a misconfiguration and is rejected outright rather than silently
    ignored.
    """
    relative = ".agents/forge/signing-keys.txt"
    body = git_out(root, ["show", f"HEAD:{relative}"])
    if body is None:
        # Not committed (or no HEAD): no pin can be claimed. Never fall back to
        # the working-tree copy -- that is the file an agent can edit.
        return set()
    keys: set[str] = set()
    for line in body.splitlines():
        entry = line.split("#", 1)[0].strip().upper()
        if not entry:
            continue
        if not re.fullmatch(r"[0-9A-F]{40}", entry):
            raise ForgeControlError(
                f"{relative}: '{entry[:16]}' is not a 40-hex fingerprint; a "
                "malformed allowlist would widen the key pin"
            )
        keys.add(entry)
    return keys


def anchor_signing_mode(root: Path, feature_id: str) -> str:
    """`signed` | `partially-signed` | `unsigned` -- reported, never blocking.

    `partially-signed` means SOME anchors carry a signature from a PINNED key:
    the honest picture of a key that went away mid-run. It deliberately does
    not count unpinned signatures. Testing bare `%G?` here re-opened the hole
    the pin exists to close -- an agent's self-made key reported identically to
    the operator's hardware key, so a forged chain read as partly trusted.
    """
    if anchor_signed(root, feature_id):
        return "signed"
    head = git_out(root, ["rev-parse", "--verify", "--quiet", forge_ref(feature_id)])
    allowed = expected_signing_keys(root)
    if head and allowed:
        rows = git_out(root, ["log", "--format=%G?%x09%GF%x09%GP", head]) or ""
        for row in rows.splitlines():
            if signature_is_pinned(row, allowed):
                return "partially-signed"
    return "unsigned"


def anchor_signed(root: Path, feature_id: str) -> bool:
    """Whether every anchor commit is signed by an EXPECTED key.

    The ledger must distinguish an anchor that constrains a hostile agent from
    one that only records honest history; conflating them is how this feature
    overstated its guarantees twice. A signature by an unpinned key does not
    count -- that would only prove the signer had *some* key.
    """
    head = git_out(root, ["rev-parse", "--verify", "--quiet", forge_ref(feature_id)])
    if not head:
        return False
    allowed = expected_signing_keys(root)
    if not allowed:
        return False  # nothing to pin against: cannot claim a trusted anchor
    rows = git_out(root, ["log", "--format=%G?%x09%GF%x09%GP", head])
    if not rows:
        return False
    for row in rows.splitlines():
        if not row.strip():
            continue
        if not signature_is_pinned(row, allowed):
            return False
    return True


def signature_is_pinned(row: str, allowed: set[str]) -> bool:
    """Whether one `%G?\t%GF\t%GP` row is a good signature by a pinned key.

    Matches the SIGNING KEY or its PRIMARY. A hardware key signs with its
    signing subkey (`sub: s`), never the primary, so pinning `%GF` alone
    rejected every genuine signature this feature exists to accept -- found
    only by signing for real, after six review rounds of reading the code.
    Pinning the primary is the right granularity: an operator rotating to a new
    signing subkey is the same operator, and the primary is what an allowlist
    reviewer can actually verify.

    Entries are validated 40-hex, so match FULL fingerprints. An earlier
    bidirectional endswith() turned a short entry into a wildcard: an allowlist
    of "7" accepted any key ending in 7.
    """
    parts = row.split("\t")
    if not parts or parts[0].strip() != "G":
        return False
    # %GP can be empty (older git, or a signature git cannot trace to a
    # primary); an empty field must never match an empty-ish allowlist entry,
    # which is why entries are 40-hex-validated at parse time.
    return any(part.strip().upper() in allowed for part in parts[1:] if part.strip())


#: How long to wait for an anchor signature. A hardware key needs a *human* to
#: reach over and touch it, so the budget is a person's reaction time, not a
#: process timeout. Long enough to be usable when the operator is present;
#: bounded so an unattended run still finishes.
SIGNING_TIMEOUT_DEFAULT = 60
#: A declined/missed touch is a transient miss, not proof the key is absent, so
#: the "don't retry" cache expires. Only a genuinely unavailable key should keep
#: costing the full budget once per window.
SIGNING_PROBE_TTL = 900


def signing_timeout() -> int:
    """Seconds to wait for a signature, from `FORGE_SIGN_TIMEOUT`."""
    raw = os.environ.get("FORGE_SIGN_TIMEOUT")
    if not raw:
        return SIGNING_TIMEOUT_DEFAULT
    try:
        value = int(raw)
    except ValueError:
        return SIGNING_TIMEOUT_DEFAULT
    # Floor: below a few seconds no human could ever touch in time, and a
    # too-small value would silently mean "never sign".
    return max(5, min(value, 600))


def signing_probe_fresh(root: Path) -> bool:
    """Whether a recent probe said signing is unavailable here.

    Expires: a caching bug once meant one missed touch disabled signing for
    every later transition, so an operator who stepped away for a single
    prompt silently lost signing for the whole run.
    """
    probe = signing_probe_path(root)
    if not probe.is_file():
        return False
    try:
        age = time.time() - probe.stat().st_mtime
    except OSError:
        return False
    if age > SIGNING_PROBE_TTL:
        try:
            probe.unlink()
        except OSError:
            pass
        return False
    return True


def signing_probe_path(root: Path) -> Path:
    """Where the 'signing blocks here' probe is cached.

    Deliberately in the ignored local state, not the spec dir: a sentinel
    sitting beside the journal is agent-writable, so pre-planting one silently
    downgraded a run the operator had explicitly opted into signing for. Keyed
    by GNUPGHOME so a different key environment re-probes.
    """
    key = sha256_bytes((os.environ.get("GNUPGHOME") or "~/.gnupg").encode())[:12]
    return root / ".forge/state" / f"signing-probe-{key}"


def anchor_gaps_path(spec_dir: Path) -> Path:
    return spec_forge_dir(spec_dir) / "anchor-gaps.jsonl"


def record_anchor_gap(spec_dir: Path, event: dict[str, Any]) -> None:
    """Note a transition whose anchor write failed.

    Anchoring is best-effort, but the gate compares chain depth against journal
    length -- so an unrecorded failure looked exactly like a rewritten chain and
    wedged the feature permanently. Recording the gap keeps the comparison
    honest and makes repair possible.

    KNOWN LIMIT (sec-028-r5-4): this file is neither hash-chained nor anchored,
    so an agent with shell access can plant entries. It cannot forge a PASS --
    the head anchor still pins the journal hash and entry count, so appended
    events are caught regardless of gaps. What a planted gap CAN do is excuse
    missing mid-chain anchors, i.e. destroy evidence and mute the ordering
    check. ship_gate therefore reports `anchor_gaps`, and any non-zero value
    means the chain is incomplete and a re-anchor is owed. Consistent with the
    accepted forensic-only posture: this is evidence, not prevention.
    """
    path = anchor_gaps_path(spec_dir)
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        with path.open("a", encoding="utf-8") as handle:
            handle.write(
                json.dumps(
                    {"at": event.get("at") or now(), "prev": event.get("prev")},
                    sort_keys=True,
                )
                + "\n"
            )
    except OSError:
        pass  # best-effort by contract; the gate still fails closed


def anchor_gaps(spec_dir: Path) -> int:
    path = anchor_gaps_path(spec_dir)
    if not path.is_file():
        return 0
    try:
        return sum(1 for line in path.read_text().splitlines() if line.strip())
    except OSError:
        return 0


def anchor_repairs(spec_dir: Path) -> int:
    """How many times this feature's chain has been rebuilt."""
    path = spec_forge_dir(spec_dir) / "reanchors.jsonl"
    if not path.is_file():
        return 0
    try:
        return sum(1 for line in path.read_text().splitlines() if line.strip())
    except OSError:
        return 0


def rebuild_conflicts(
    root: Path, head: str, events: list[dict[str, Any]]
) -> list[str]:
    """Journal entries that contradict an anchor the existing chain holds.

    A repair restores anchors a write failure lost; it must never change one
    that succeeded. Comparing the two is what makes `phase-reanchor` unable to
    launder a doctored journal -- the flag-based operator gate could not, since
    an agent simply passes the flag.
    """
    conflicts: list[str] = []
    listing = git_out(root, ["rev-list", "--reverse", head])
    if listing is None:
        # Cannot read the chain we are about to replace: refuse rather than
        # assume it holds nothing worth contradicting.
        return ["the existing chain could not be read"]
    for commit in [line.strip() for line in listing.splitlines() if line.strip()]:
        body = git_out(root, ["show", f"{commit}:transition.json"])
        if not body:
            continue
        try:
            payload = json.loads(body)
        except json.JSONDecodeError:
            continue
        index = payload.get("journal_entries")
        if not isinstance(index, int) or not 1 <= index <= len(events):
            continue
        entry = events[index - 1]
        recorded = payload.get("transition") or {}
        for field in ("from", "result", "rule", "next"):
            if field in recorded and recorded[field] != entry.get(field):
                conflicts.append(
                    f"anchor {commit[:12]} recorded {field}="
                    f"'{recorded[field]}' but the journal now says "
                    f"'{entry.get(field)}' at entry {index}"
                )
                break
    return conflicts


def phase_reanchor(root: Path, ns: argparse.Namespace) -> dict[str, Any]:
    """Rebuild a feature's anchor chain from its journal after a write failure.

    Without this, a single transient anchor failure left a completed feature
    permanently unshippable and the only escape was to bypass the gate --
    which trains operators out of the control. The superseded chain is kept
    under refs/forge-superseded/<id>-<utc> rather than discarded, so the
    pre-repair evidence survives even when a later repair runs.

    This writes NO journal entry: the journal is the input to the rebuild, so
    appending to it here would change the very thing each commit must hash.
    The repair is evidenced by the superseded ref, not by the journal.
    """
    spec_dir = resolve_spec_dir(root, ns.spec_dir, getattr(ns, "feature_id", None))
    events = read_events(spec_dir)
    if not events:
        raise ForgeControlError("no journal to re-anchor")
    feature = journal_feature_id(spec_dir, getattr(ns, "feature_id", None))
    ref = forge_ref(feature)

    # OPERATOR-ONLY. Re-anchoring rebuilds the evidence chain FROM the journal,
    # so an agent that doctored the journal could use it to mint a matching
    # chain and ship -- verified: appending unanchored forged entries then
    # re-anchoring flipped the gate to allowed.
    #
    # There is DELIBERATELY no interactive path. The previous gate accepted
    # `sys.stdin.isatty()` as proof of a human, but isatty() is an ambient
    # property of the CALLER, not an authentication primitive: an agent that
    # spawns this under a pty gets isatty() == True and answers its own
    # prompt. Verified -- a pty.spawn child reports isatty True, so the
    # "interactive confirmation" authenticated nobody. This is the same shape
    # as every earlier defect here: a control resting on an artifact the
    # untrusted caller controls.
    #
    # What remains is honest rather than strong: --assume-operator plus a
    # NAMED operator, recorded in the run's own artifacts. Like phase-attest
    # (sec-forge-15) this is an attributable assertion, not proof -- but it is
    # not a gate that quietly believes it is more.
    if not getattr(ns, "assume_operator", False):
        raise ForgeControlError(
            "phase-reanchor rewrites the evidence chain and needs an operator: "
            "pass --assume-operator --operator '<name>' from a system that "
            "already authenticates you. There is no interactive confirmation: "
            "a tty proves nothing about who is calling"
        )
    operator = (getattr(ns, "operator", None) or "").strip()
    if not operator:
        raise ForgeControlError(
            "phase-reanchor needs --operator '<name>': a rebuilt chain is "
            "weaker evidence than an organic one, so the repair must be "
            "attributable to someone"
        )

    previous = git_out(root, ["rev-parse", "--verify", "--quiet", ref])

    # THE REPAIR MAY NOT REWRITE HISTORY, ONLY RESTORE IT. Requiring an
    # operator flag stopped nothing: an agent passes the flag, so doctoring the
    # journal and re-anchoring still flipped a security-hold to shippable.
    # The existing chain is the evidence needed to prevent that -- every anchor
    # it holds records what the journal ACTUALLY said at that point. A rebuild
    # must therefore agree with every anchor that survives; it may only fill in
    # entries the chain never recorded.
    if previous:
        conflicts = rebuild_conflicts(root, previous, events)
        if conflicts:
            raise ForgeControlError(
                "refusing to re-anchor: the journal contradicts anchors the "
                "existing chain already recorded, so this is a rewrite, not a "
                "repair. " + "; ".join(conflicts[:3])
            )

    parked = None
    if previous:
        stamped = now()
        parked = superseded_ref(feature, stamped)
        # Never clobber: if an earlier repair already parked a chain at this
        # second, walk to a free name rather than overwrite its evidence.
        suffix = 0
        while git_out(root, ["rev-parse", "--verify", "--quiet", parked]):
            suffix += 1
            parked = f"{superseded_ref(feature, stamped)}-{suffix}"
        # git_out returns "" on a successful update-ref, so test for None --
        # a truthiness check would read every success as a failure.
        if git_out(root, ["update-ref", parked, previous]) is None:
            raise ForgeControlError(
                f"could not preserve the existing chain at {parked}; refusing to "
                "rebuild, because discarding it would destroy the evidence a "
                "repair exists to explain"
            )
        git_out(root, ["update-ref", "-d", ref])

    rebuilt = 0
    for index, event in enumerate(events, start=1):
        if anchor_transition(
            root, spec_dir, feature, event, ns.repo or [], entry_index=index
        ) is None:
            raise ForgeControlError(
                f"re-anchor failed at entry {rebuilt + 1}; the previous chain is "
                f"preserved at {parked or '(none: there was no prior chain)'}"
            )
        rebuilt += 1

    gaps = anchor_gaps_path(spec_dir)
    consumed = anchor_gaps(spec_dir)
    if gaps.is_file():
        gaps.unlink()
    record = {
        "at": now(),
        "feature_id": feature,
        "operator": operator,
        "superseded": previous,
        "superseded_ref": parked,
        "rebuilt": rebuilt,
        "gaps_consumed": consumed,
    }
    # A repair leaves the chain materially weaker (every rebuilt commit records
    # a position derived from the completed journal), so the FACT of a repair
    # must survive. It cannot go in the journal -- the journal is the rebuild's
    # input, and appending would change what each commit must hash -- so it
    # goes beside it, and ship_gate surfaces the count.
    try:
        repairs = spec_forge_dir(spec_dir) / "reanchors.jsonl"
        repairs.parent.mkdir(parents=True, exist_ok=True)
        with repairs.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps(record, sort_keys=True) + "\n")
    except OSError:
        pass
    return {
        "ref": ref,
        "rebuilt": rebuilt,
        "superseded": previous,
        "superseded_ref": parked,
        "operator": operator,
        "note": "chain rebuilt from the journal; the journal itself was not modified",
    }


def append_event(
    spec_dir: Path,
    event: dict[str, Any],
    *,
    root: Path | None = None,
    repos: list[str] | None = None,
) -> dict[str, Any]:
    """Append one hash-chained record to the append-only journal.

    Without the chain, truncating events.jsonl reset every blocker and the
    deletion was undetectable -- which for an audit artifact means it evidences
    only the happy path.
    """
    events = read_events(spec_dir, verify=False)
    previous = events[-1].get("prev", "") if events else ""
    linked = dict(event)
    linked["prev"] = chain_hash(str(previous), event)
    journal = spec_forge_dir(spec_dir) / "events.jsonl"
    journal.parent.mkdir(parents=True, exist_ok=True)
    with journal.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(linked, sort_keys=True) + "\n")
        handle.flush()
        os.fsync(handle.fileno())
    # Anchor the head OUTSIDE the journal: an intra-file chain detects edits but
    # not truncation, because chopping the tail leaves a valid prefix.
    write_json_atomically(
        spec_forge_dir(spec_dir) / "journal-head.json",
        {"entries": len(events) + 1, "head": linked["prev"], "updated_at": linked.get("at", now())},
    )

    # Anchor HERE, not at the call sites. Anchoring used to be a separate call
    # that phase_sign forgot, which permanently desynchronised journal length
    # from chain depth and deadlocked every signed run at the ship gate. Binding
    # the two writes together makes that divergence unrepresentable.
    if root is not None:
        feature = journal_feature_id(spec_dir, str(event.get("feature_id") or ""))
        linked["anchor"] = anchor_transition(root, spec_dir, feature, linked, repos or [])
        if linked["anchor"] is None:
            # Record the shortfall so the gate can tell a write failure from a
            # rewritten chain, and so `phase-reanchor` knows repair is needed.
            record_anchor_gap(spec_dir, linked)
    return linked


def read_events(spec_dir: Path, *, verify: bool = True) -> list[dict[str, Any]]:
    path = spec_forge_dir(spec_dir) / "events.jsonl"
    if not path.is_file():
        return []
    events: list[dict[str, Any]] = []
    for index, line in enumerate(path.read_text().splitlines(), start=1):
        line = line.strip()
        if not line:
            continue
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError as exc:
            raise ForgeControlError(f"{path}:{index}: corrupt journal entry: {exc}") from exc
    if verify:
        previous = ""
        for index, event in enumerate(events, start=1):
            # The chain is TOTAL. An earlier build skipped entries lacking
            # `prev`, which made forging cheaper than tampering: appending
            # unchained records walked a security-held feature to `done` while
            # every verifier stayed silent.
            if "prev" not in event:
                raise ForgeControlError(
                    f"{path}:{index}: unchained journal entry -- the record was not "
                    "written by forge-control.py"
                )
            expected = chain_hash(previous, event)
            if event["prev"] != expected:
                raise ForgeControlError(
                    f"{path}:{index}: journal chain broken -- entries were removed or edited"
                )
            previous = event["prev"]

        # The anchor is mandatory once any entry exists: a missing one is
        # tampering, not a legacy state, since append_event always writes it.
        anchor_path = spec_forge_dir(spec_dir) / "journal-head.json"
        if events and not anchor_path.is_file():
            raise ForgeControlError(
                f"{path}: journal head anchor is missing -- the journal cannot be trusted"
            )
        if anchor_path.is_file():
            try:
                anchor = json.loads(anchor_path.read_text())
            except json.JSONDecodeError as exc:
                raise ForgeControlError(f"{anchor_path}: corrupt journal anchor: {exc}") from exc
            # Exact match, not >=: appending past the anchor is as much a
            # tamper as truncating below it.
            if anchor.get("entries") != len(events):
                raise ForgeControlError(
                    f"{path}: journal length disagrees with its anchor -- anchor records "
                    f"{anchor.get('entries')} entries, found {len(events)}"
                )
            if anchor.get("head") and anchor["head"] != previous:
                raise ForgeControlError(
                    f"{path}: journal head disagrees with its anchor -- entries were replaced"
                )
    return events


def write_json_atomically(path: Path, payload: dict[str, Any]) -> None:
    """Write via temp + replace so a crash cannot leave truncated JSON."""
    path.parent.mkdir(parents=True, exist_ok=True)
    temp = path.with_suffix(path.suffix + ".tmp")
    temp.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n")
    os.replace(temp, path)


def consecutive_failures(events: list[dict[str, Any]], phase: str, result: str) -> int:
    """Replay the journal for repeated non-advancing outcomes of one phase.

    Derived rather than accepted from the caller so a coordinator cannot reset
    its own retry budget.
    """
    count = 1
    for event in reversed(events):
        if event.get("from") != phase:
            continue
        # Only *leaving* the phase resets the budget. Counting consecutive
        # identical results let an interleaved alternate result (e.g.
        # council-failed between two architect failures) reset it silently,
        # making "fails twice -> stop" unreachable.
        if event.get("next") != phase:
            break
        if event.get("result") == result:
            count += 1
    return count


def repo_head(worktree: Path) -> str | None:
    try:
        result = subprocess.run(
            ["git", "-C", str(worktree), "rev-parse", "HEAD"],
            text=True,
            capture_output=True,
            timeout=15,
            check=False,
            env=git_env(),
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    return result.stdout.strip() or None if result.returncode == 0 else None


def worktree_dirty(worktree: Path) -> bool | None:
    try:
        result = subprocess.run(
            ["git", "-C", str(worktree), "status", "--porcelain"],
            text=True,
            capture_output=True,
            env=git_env(),
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired):
        return None
    if result.returncode != 0:
        return None
    return bool(result.stdout.strip())


def feature_repos(root: Path, feature_id: str, declared: list[str]) -> list[str]:
    """Every repo in this feature, discovered rather than asserted.

    Drift and dirty checks previously iterated the caller's --repo list, so
    omitting a repo (or the flag entirely) made its post-review commits
    invisible. The worktree directory is ground truth; --repo can only add.
    """
    found = set(declared)
    worktrees = root / "worktrees" / feature_id
    if worktrees.is_dir():
        found.update(child.name for child in worktrees.iterdir() if (child / ".git").exists())
    return sorted(found)


def observe_heads(root: Path, feature_id: str, repos: list[str]) -> dict[str, str]:
    """Read current HEADs straight from the feature worktrees."""
    heads: dict[str, str] = {}
    for repo in repos:
        worktree = root / "worktrees" / feature_id / repo
        if not worktree.is_dir():
            continue
        head = repo_head(worktree)
        if head:
            heads[repo] = head
    return heads


def observe_dirty(root: Path, feature_id: str, repos: list[str]) -> list[str]:
    dirty: list[str] = []
    for repo in repos:
        worktree = root / "worktrees" / feature_id / repo
        if not worktree.is_dir():
            continue
        if worktree_dirty(worktree):
            dirty.append(repo)
    return sorted(dirty)


def journal_blockers(events: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Reconstruct open hard blockers from the append-only journal.

    state.json is a convenience cache and is rewritten on every transition, so
    it must never be the sole authority: deleting it would otherwise discard a
    security hold. A hard stop stays open until a later transition out of the
    same phase clears it explicitly.
    """
    open_blockers: dict[str, dict[str, Any]] = {}
    for event in events:
        if event.get("kind") == "clearance":
            # Only an ATTESTED clearance retires a blocker. A deferred one is an
            # agent's proposal: it records remediation context without hanging an
            # unattended run, and is converted by `phase-attest` at the end.
            if event.get("attested"):
                open_blockers.pop(str(event.get("clears")), None)
            continue
        if event.get("severity") == "hard":
            rule = str(event.get("rule"))
            open_blockers[rule] = {
                "rule": rule,
                "phase": event.get("from"),
                "reason": event.get("reason"),
                "at": event.get("at"),
            }
    return list(open_blockers.values())


def phase_clear(root: Path, ns: argparse.Namespace) -> dict[str, Any]:
    """Record a PROPOSED clearance of one hard blocker. Never blocks a run.

    An agent may write this freely -- it is remediation context, not authority.
    The blocker stays open until an operator attests it via `phase-attest`, so
    an unattended run is never stalled waiting for a human, and an agent can
    never self-authorise its way past a security hold.
    """
    spec_dir = resolve_spec_dir(root, ns.spec_dir, getattr(ns, "feature_id", None))
    events = read_events(spec_dir)
    open_rules = {blocker["rule"] for blocker in journal_blockers(events)}
    if ns.rule not in open_rules:
        raise ForgeControlError(
            f"no open blocker '{ns.rule}'; currently open: {sorted(open_rules) or 'none'}"
        )
    if len(ns.reason.strip()) < 12:
        raise ForgeControlError("clearance requires a substantive --reason (>= 12 characters)")

    event = {
        "schema": "forge.phase-event/v1",
        "kind": "clearance",
        "at": now(),
        "clears": ns.rule,
        # Proposed, not granted. `operator` is who the agent believes should
        # decide; attestation is what makes it authoritative.
        "attested": False,
        "proposed_by": ns.proposed_by,
        "operator": ns.operator,
        "reason": ns.reason.strip(),
        "evidence": ns.evidence,
    }
    append_event(spec_dir, event, root=root)

    state = read_state(spec_dir)
    remaining = journal_blockers(read_events(spec_dir))
    if state:
        state["blockers"] = remaining
        state["updated_at"] = event["at"]
        write_json_atomically(state_path(spec_dir), state)

    return {
        "proposed": ns.rule,
        "attested": False,
        "operator": ns.operator,
        "remaining_blockers": remaining,
        "still_blocking": any(b["rule"] == ns.rule for b in remaining),
        "note": (
            "Proposal recorded; the blocker REMAINS OPEN. An operator retires it with: "
            f"forge-control.py phase-attest --spec-dir <dir> --rule {ns.rule} --operator <you>"
        ),
    }


def repo_base_ref(
    root: Path, repositories: dict[str, dict[str, Any]], worktree: Path, repo: str
) -> str | None:
    """Resolve the ref a feature branch was cut from, per repo.

    Hardcoding origin/develop silently skipped the five declared repos that
    base on main/master, so their commits shipped unverified.
    """
    declared = str((repositories.get(repo) or {}).get("default_branch") or "").strip()
    candidates = [f"origin/{declared}", declared] if declared else []
    candidates += ["origin/develop", "origin/main", "origin/master", "develop", "main", "master"]
    for candidate in candidates:
        if not candidate:
            continue
        probe = subprocess.run(
            ["git", "-C", str(worktree), "rev-parse", "--verify", "--quiet", f"{candidate}^{{commit}}"],
            capture_output=True, timeout=15, check=False, env=git_env(),
        )
        if probe.returncode == 0:
            return candidate
    return None


def unsigned_feature_commits(worktree: Path, base: str) -> list[str]:
    """Commits on this branch that carry no good signature."""
    try:
        result = subprocess.run(
            ["git", "-C", str(worktree), "log", "--format=%H %G?", f"{base}..HEAD"],
            text=True,
            capture_output=True,
            timeout=30,
            check=False,
            env=git_env(),
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise ForgeControlError(f"cannot inspect commits in {worktree}: {exc}") from exc
    if result.returncode != 0:
        raise ForgeControlError(f"cannot inspect commits in {worktree}: {result.stderr.strip()}")
    unsigned = []
    for line in result.stdout.splitlines():
        parts = line.split()
        # Only a good signature from a trusted key counts. "U" is a valid
        # signature from an untrusted key -- that is not attribution.
        if len(parts) == 2 and parts[1] != "G":
            unsigned.append(parts[0])
        elif len(parts) == 1:
            unsigned.append(parts[0])
    return unsigned


def tree_map(worktree: Path, base: str) -> list[tuple[str, str]]:
    """(subject, tree-hash) per commit -- the invariant a re-sign must preserve."""
    result = subprocess.run(
        ["git", "-C", str(worktree), "log", "--format=%T%x09%s", f"{base}..HEAD"],
        text=True,
        capture_output=True,
        timeout=30,
        check=False,
        env=git_env(),
    )
    if result.returncode != 0:
        raise ForgeControlError(f"cannot read trees in {worktree}: {result.stderr.strip()}")
    rows = []
    for line in result.stdout.splitlines():
        tree, _, subject = line.partition("\t")
        rows.append((subject, tree))
    return rows


def phase_sign(root: Path, ns: argparse.Namespace) -> dict[str, Any]:
    """Sign every unsigned feature commit in one batched pass at run close.

    Implementers commit unsigned during a run: signing is hardware-key backed,
    so per-commit signing either stalls on a pinentry timeout in an unattended
    run or demands a touch per commit per repo. Signing once at the end costs
    one touch per repo instead.

    A re-sign rewrites SHAs, which would otherwise trip the post-review drift
    gate. Two things make that safe: the operation is verified to be
    signature-only (every tree hash identical before and after), and the
    reviewed-head baseline is re-anchored to the new SHAs -- but only after
    that verification passes, so a content change can never be laundered
    through the signing step.
    """
    spec_dir = resolve_spec_dir(root, ns.spec_dir, ns.feature_id)
    feature_id = ns.feature_id or spec_dir.name
    events = read_events(spec_dir)
    state = read_state(spec_dir)
    try:
        repositories = load_repositories(root)
    except (OSError, ForgeControlError):
        repositories = {}
    repos = feature_repos(root, feature_id, repo_names(ns.repo))
    if not repos:
        return {"signed": {}, "note": f"no worktrees found for {feature_id}"}

    signed: dict[str, Any] = {}
    remapped: dict[str, str] = {}
    for repo in repos:
        worktree = root / "worktrees" / feature_id / repo
        if not worktree.is_dir():
            continue
        # ONE base resolver for signing and for the gate. A second, weaker
        # resolver here reintroduced the exact defect repo_base_ref()'s own
        # docstring warns about: hardcoding origin/develop silently skipped
        # every repo based on main/master. For those repos phase_sign records
        # "skipped" here while ship_gate correctly finds and denies the
        # unsigned commits -- an unrecoverable deadlock, since the reason text
        # tells the operator to run phase-sign, which skips the same repo
        # again.
        base = ns.base or repo_base_ref(root, repositories, worktree, repo)
        if base is None:
            signed[repo] = {
                "status": "skipped",
                "detail": "cannot establish a signing baseline",
            }
            continue
        try:
            pending = unsigned_feature_commits(worktree, base)
        except ForgeControlError as exc:
            signed[repo] = {"status": "skipped", "detail": str(exc)}
            continue
        if not pending:
            # Do NOT re-anchor here. Nothing was rewritten, so the current head
            # is either the reviewed one (re-anchoring is a no-op) or it moved
            # for some other reason -- and re-anchoring that would launder
            # unreviewed commits past the drift gate. Only a verified rewrite
            # earns a new baseline.
            signed[repo] = {"status": "already-signed", "commits": 0}
            continue

        before = tree_map(worktree, base)
        head_before = repo_head(worktree)
        if ns.dry_run:
            signed[repo] = {"status": "would-sign", "commits": len(pending)}
            continue

        # Signing may only re-anchor a baseline it is entitled to move: the
        # pre-rewrite head must be exactly what review examined. Otherwise the
        # branch already drifted and that is review's business, not signing's.
        prior_reviewed = dict(authoritative_reviewed_heads(state, events))
        entitled = repo not in prior_reviewed or prior_reviewed[repo] == head_before

        # A merge commit would be silently flattened by a plain rebase.
        merges = subprocess.run(
            ["git", "-C", str(worktree), "log", "--merges", "--format=%H", f"{base}..HEAD"],
            text=True, capture_output=True, timeout=30, check=False, env=git_env(),
        )
        if merges.returncode == 0 and merges.stdout.strip():
            signed[repo] = {
                "status": "skipped",
                "detail": "branch contains merge commits; batch signing would flatten history",
            }
            continue

        # Snapshot so a failed verification restores the tree instead of
        # leaving it mid-rewrite. Built from the SANITISED name forge_ref()
        # already produces, not the raw feature_id: a raw id containing '..'
        # or ending '.lock' makes update-ref fail (git rejects the ref name),
        # and that failure was previously silent (check=False). Nothing here
        # noticed, the rebase proceeded, and a failed tree-verification later
        # tried to `reset --hard` a ref that was never created -- silently a
        # no-op, leaving the worktree mid-rewrite while the error text claimed
        # it was restored.
        backup = f"refs/forge-backup/{forge_ref(feature_id).split('/', 2)[-1]}"
        snapshot = subprocess.run(
            ["git", "-C", str(worktree), "update-ref", backup, "HEAD"],
            capture_output=True, timeout=30, check=False, env=git_env(),
        )
        if snapshot.returncode != 0:
            signed[repo] = {
                "status": "skipped",
                "detail": "could not snapshot HEAD; refusing an unrecoverable rewrite",
            }
            continue

        # --no-edit keeps every message byte-identical; only signatures change.
        result = subprocess.run(
            [
                "git", "-C", str(worktree), "rebase", "--exec",
                "git commit --amend --no-edit --no-verify -S", base,
            ],
            text=True,
            capture_output=True,
            timeout=600,
            check=False,
            env=git_env(),
        )
        if result.returncode != 0:
            subprocess.run(
                ["git", "-C", str(worktree), "rebase", "--abort"],
                capture_output=True, timeout=60, check=False, env=git_env(),
            )
            signed[repo] = {"status": "failed", "detail": result.stderr.strip()[:300]}
            continue

        after = tree_map(worktree, base)
        if before != after:
            subprocess.run(
                ["git", "-C", str(worktree), "reset", "--hard", backup],
                capture_output=True, timeout=60, check=False, env=git_env(),
            )
            raise ForgeControlError(
                f"{repo}: signing altered content -- expected a signature-only rewrite. "
                "The worktree was restored to its pre-signing state."
            )
        signed[repo] = {
            "status": "signed",
            "commits": len(pending),
            "head": repo_head(worktree),
            "rebaseline": "entitled" if entitled else "withheld-branch-already-drifted",
        }
        # Re-anchor ONLY when the rewrite started from the reviewed head.
        if entitled:
            remapped[repo] = repo_head(worktree) or ""
        subprocess.run(
            ["git", "-C", str(worktree), "update-ref", "-d", backup],
            capture_output=True, timeout=30, check=False, env=git_env(),
        )

    # Re-anchor the review baseline: the drift gate must not fire on SHAs this
    # very operation rewrote, now that trees are proven unchanged.
    rebaselined = False
    if remapped and not ns.dry_run:
        reviewed = dict(authoritative_reviewed_heads(state, events))
        # Never MATERIALISE a cache that did not exist. read_state() returns
        # {} when state.json is absent, so `if reviewed:` alone wrote a
        # state.json containing only reviewed_heads/updated_at -- no
        # `shippable` key. ship_gate reads a present-but-shippable-less state
        # as `not state.get("shippable")`, i.e. NOT SHIPPABLE: a run whose
        # journal alone was shippable became permanently unshippable after a
        # SUCCESSFUL signature-only rewrite. Requiring `state` (non-empty) to
        # already exist means this can only ever update a cache that was
        # already tracking the feature, never conjure a half-populated one.
        if reviewed and state:
            for repo, head in remapped.items():
                if head and repo in reviewed:
                    reviewed[repo] = head
            state["reviewed_heads"] = reviewed
            state["updated_at"] = now()
            write_json_atomically(state_path(spec_dir), state)
            rebaselined = True
        append_event(
            spec_dir,
            {
                "schema": "forge.phase-event/v1",
                "kind": "signing",
                "at": now(),
                "feature_id": feature_id,
                "repos": sorted(remapped),
                "heads": remapped,
                "rebaselined_review_heads": rebaselined,
                "verification": "tree-identical",
            },
            root=root,
            repos=repos,
        )

    return {
        "signed": signed,
        "rebaselined_review_heads": rebaselined,
        "note": "Signature-only rewrite verified: every tree hash unchanged.",
    }


def phase_attest(root: Path, ns: argparse.Namespace) -> dict[str, Any]:
    """Operator confirmation that retires proposed clearances. Batched by design.

    Runs at the END of a Forge run, once, over everything the run proposed --
    so an unattended run is never stalled mid-flight, and an operator is never
    asked to confirm a blocker that never occurred. Clean runs never call this.

    Identity is asserted, not cryptographically proven: `--assume-operator`
    plus a named `--operator`, for use from a system that already
    authenticates the caller. There is no interactive path and no hardware
    key -- round 6 removed the `isatty()` gate because an agent under a pty
    answers its own prompt, and the journal then recorded `mode:
    "interactive"`, i.e. the ledger lied. Every clearance is therefore
    recorded `mode: "assumed"`, the only thing this command can honestly
    claim.

    This is separation of duties, not an authentication boundary: an agent
    with shell access can pass the flag and any operator name (sec-forge-15,
    accepted residual). What it buys is that retirement is a distinct, named,
    journalled act an agent following its charter will not perform, and that
    a forged clearance is a permanently attributed lie rather than a silent
    state change.
    """
    spec_dir = resolve_spec_dir(root, ns.spec_dir, getattr(ns, "feature_id", None))
    events = read_events(spec_dir)

    proposals = {
        str(event.get("clears")): event
        for event in events
        if event.get("kind") == "clearance" and not event.get("attested")
    }
    open_rules = {blocker["rule"] for blocker in journal_blockers(events)}
    targets = [ns.rule] if ns.rule else sorted(proposals.keys() & open_rules)
    if not targets:
        return {"attested": [], "remaining_blockers": journal_blockers(events),
                "note": "nothing to attest"}

    for rule in targets:
        if rule not in open_rules:
            raise ForgeControlError(f"no open blocker '{rule}'; open: {sorted(open_rules) or 'none'}")

    # NO INTERACTIVE PATH. This gated on `sys.stdin.isatty()`, exactly as
    # phase_reanchor did before round 5 -- and the round-5 fix was applied only
    # at the site where the bug was found, not at every site sharing the
    # pattern. isatty() describes the CALLER's plumbing: an agent that spawns
    # this under a pty answers its own prompt, retires a REVIEW-SECURITY-HOLD,
    # and the journal records `mode: "interactive"`, which asserts a human was
    # at a terminal. That is worse than no attestation: the ledger lies.
    #
    # What remains is an attributable assertion (--assume-operator plus a named
    # --operator), not proof. See sec-forge-15. Every clearance is now recorded
    # `mode: "assumed"` because that is the only mode this command can honestly
    # claim.
    if not ns.assume_operator:
        raise ForgeControlError(
            "phase-attest needs an operator: pass --assume-operator --operator "
            "'<name>' from a system that already authenticates you. There is no "
            "interactive confirmation -- a tty proves nothing about who is calling."
        )
    # argparse `required=True` accepts an empty string, and an anonymous
    # clearance defeats the only property this command still has.
    if not (ns.operator or "").strip():
        raise ForgeControlError(
            "phase-attest needs --operator '<name>': retiring a blocker is a "
            "risk acceptance, and it must be attributable to someone"
        )

    stamped = now()
    for rule in targets:
        proposal = proposals.get(rule, {})
        append_event(
            spec_dir,
            {
                "schema": "forge.phase-event/v1",
                "kind": "clearance",
                "attested": True,
                "at": stamped,
                "clears": rule,
                "operator": ns.operator,
                "mode": "assumed",
                "reason": proposal.get("reason", ns.reason or "attested at run close"),
                "evidence": proposal.get("evidence") or ns.evidence,
            },
            root=root,
        )

    remaining = journal_blockers(read_events(spec_dir))
    state = read_state(spec_dir)
    if state:
        state["blockers"] = remaining
        state["updated_at"] = stamped
        write_json_atomically(state_path(spec_dir), state)

    return {
        "attested": targets,
        "operator": ns.operator,
        "mode": "assumed",
        "remaining_blockers": remaining,
    }


def reviewed_heads(events: list[dict[str, Any]]) -> dict[str, str]:
    """The HEADs the most recent review advance actually examined.

    A later `kind: "signing"` event rewrites those same commits signature-only
    (phase-sign verifies every tree hash is unchanged before appending it), so
    its remapped heads are the SAME reviewed trees under new commit ids. Apply
    them: they come from the append-only journal, not the agent-writable cache,
    so journal-first is preserved. Without this, every successfully signed run
    replays pre-rewrite SHAs and fails the drift and cache-agreement gates
    forever.
    """
    heads: dict[str, str] = {}
    review_seen_at = -1
    for idx, event in enumerate(events):
        if event.get("from") == "review" and event.get("heads"):
            heads = dict(event["heads"])
            review_seen_at = idx
    if not heads:
        return {}
    for event in events[review_seen_at + 1:]:
        if (
            event.get("kind") == "signing"
            and event.get("verification") == "tree-identical"
            and event.get("heads")
        ):
            for repo, head in event["heads"].items():
                if repo in heads and head:
                    heads[repo] = str(head)
        # An operator-attested `kind: "rebaseline"` moves the baseline to
        # commits reviewed AFTER the run reached a terminal phase — the
        # post-ship CodeRabbit-triage flow, which the phase table has no
        # transition for. Journal-sourced like the signing remap above, so the
        # agent-writable cache still cannot loosen a verdict.
        elif event.get("kind") == "rebaseline" and event.get("heads"):
            for repo, head in event["heads"].items():
                if repo in heads and head:
                    heads[repo] = str(head)
    return heads


def authoritative_reviewed_heads(
    state: dict[str, Any], events: list[dict[str, Any]]
) -> dict[str, str]:
    """The reviewed baseline, taking the JOURNAL as authoritative.

    Every call site used to read `state.get("reviewed_heads") or
    reviewed_heads(events)`. That is absent-evidence-as-clean-evidence in its
    purest form: state.json is agent-writable, so the attacker-authored value
    won and the trustworthy source was consulted only when the attacker
    declined to supply one. Pointing the cache at a backdoored head then made
    detect_drift see no drift at all.

    The cache may only make the verdict STRICTER. It is consulted when the
    journal has no review event (nothing to contradict), and a disagreement is
    resolved by taking BOTH -- a repo whose two sources differ can only fail
    the drift check, which is the safe direction.
    """
    journal = reviewed_heads(events)
    cached = {
        str(repo): str(head)
        for repo, head in (state.get("reviewed_heads") or {}).items()
        if head
    }
    if not journal:
        return cached
    merged = dict(journal)
    for repo, head in cached.items():
        # Disagreement is unambiguous evidence of hand-editing. Keep the
        # journal's value for repos it knows, and let an unknown repo through
        # only as an ADDITIONAL constraint -- never as a replacement.
        if repo not in merged:
            merged[repo] = head
    return merged


def detect_drift(observed: dict[str, str], reviewed: dict[str, str]) -> list[str]:
    """Any repo whose HEAD moved since the panel's verdict, or appeared after it."""
    if not reviewed:
        return []
    drifted = {repo for repo, head in observed.items() if reviewed.get(repo) != head}
    # A repo that was reviewed and has since vanished is drift, not absence:
    # its reviewed state can no longer be confirmed.
    drifted.update(repo for repo in reviewed if repo not in observed)
    return sorted(drifted)


def severity_above(value: str | None, threshold: str) -> bool:
    if not value:
        return False
    try:
        return SEVERITY_ORDER.index(value) > SEVERITY_ORDER.index(threshold)
    except ValueError as exc:
        raise ForgeControlError(f"unknown severity: {value}") from exc


def condition_holds(condition: dict[str, Any], facts: dict[str, Any]) -> bool:
    for key, expected in condition.items():
        if key == "consecutive_failures_gte":
            if facts.get("consecutive_failures", 1) < expected:
                return False
        elif key == "max_residual_severity_above":
            if not severity_above(facts.get("max_residual_severity"), expected):
                return False
        elif key in KNOWN_CONDITION_KEYS:
            if bool(facts.get(key)) is not bool(expected):
                return False
        else:
            # Fail closed on malformed policy: a typo'd key silently disabled
            # the hard stop it guarded, with no error and no test failure.
            raise ForgeControlError(
                f"unknown condition key '{key}' in the phase machine; "
                f"known keys: {sorted(KNOWN_CONDITION_KEYS)}"
            )
    return True


def evaluate_transition(
    machine: dict[str, Any], phase: str, result: str, facts: dict[str, Any]
) -> dict[str, Any]:
    """First matching rule wins; deny rules precede allow rules by construction."""
    known = {rule["from"] for rule in machine["rules"]}
    if phase not in known:
        raise ForgeControlError(f"no transitions defined from phase: {phase}")
    candidates = [rule for rule in machine["rules"] if rule["from"] == phase]
    for rule in candidates:
        if rule["result"] != result:
            continue
        if "when" in rule and not condition_holds(rule["when"], facts):
            continue
        return rule
    accepted = sorted({rule["result"] for rule in candidates})
    raise ForgeControlError(
        f"no rule matches phase '{phase}' with result '{result}'; accepted results: {accepted}"
    )


def last_transition_event(events: list[dict[str, Any]]) -> dict[str, Any]:
    """The most recent journal entry that is an actual phase transition.

    Non-transition entries (`kind` in {"clearance", "signing"}, and any future
    annotation `append_event` might record) carry no `from`/`next` and must
    never be read as if they moved the phase -- the same rule
    `sequence_is_contiguous` already enforces when replaying the whole
    journal. `ship_gate` used to read `events[-1]` directly: a trailing
    `signing` record (which `phase_sign` appends at the natural end of every
    signed run) has no `next` key, so `.get("next")` silently returned None
    and fell through to `state.get("phase")` -- the agent-writable cache --
    for the one fact ship_gate exists to get from the journal instead. One
    hand-edited state.json plus the signing step every run performs anyway
    was enough to report `phase: "ship"` over an open REVIEW-SECURITY-HOLD.
    Extracted so `current_phase()` and `ship_gate()` cannot drift the way this
    feature's duplicated primitives have drifted before.
    """
    for event in reversed(events):
        if event.get("kind"):
            continue
        if event.get("next"):
            return event
    return {}


def current_phase(machine: dict[str, Any], state: dict[str, Any], events: list[dict[str, Any]]) -> str:
    """Where the machine actually is, derived from the journal.

    state.json is a cache and may be absent or stale, so the journal wins: the
    last transition's `next` is the authoritative current phase.
    """
    event = last_transition_event(events)
    if event:
        return str(event["next"])
    return str(state.get("phase") or machine["initial"])


def sequence_is_contiguous(machine: dict[str, Any], events: list[dict[str, Any]]) -> list[str]:
    """Replay the journal and report why the path from `initial` is not intact.

    Validating a transition in isolation is not enough: without this, a caller
    can declare `regression pass` then `ship ready` on a virgin directory and
    reach a shippable verdict having never entered review.
    """
    problems: list[str] = []
    expected = str(machine["initial"])
    saw_review = False
    saw_verify = False
    reentry_allowed = False
    for event in events:
        if event.get("kind") == "clearance":
            # An attested clearance authorises exactly one re-entry from a
            # terminal stop: the operator has accepted the risk in the open.
            reentry_allowed = reentry_allowed or bool(event.get("attested"))
            continue
        if event.get("kind"):
            # Non-transition records (signing, future annotations) carry no
            # from/next and must not be read as a phase jump.
            continue
        origin = str(event.get("from"))
        if origin != expected and expected in machine["terminal"] and reentry_allowed:
            reentry_allowed = False
            expected = origin
        if origin != expected:
            problems.append(
                f"journal jumps from '{expected}' to '{origin}' (rule {event.get('rule')})"
            )
        # A `review -> stop` transition is the review FAILING, not concluding,
        # so it must not satisfy "review ran" -- the same rule the verify check
        # below already applies. Every review rule with severity `hard` goes to
        # `stop`, and every non-hard one continues the pipeline, so this
        # separates "concluded" from "failed" without losing a legitimate path.
        #
        # Latent rather than exploitable when written: `no-open-blocker` denies
        # while the hold is open, and after attestation the out-of-sequence
        # guard forces re-entry at the phase that raised the stop, so a genuine
        # new review result is still required. Fixed anyway -- the asymmetry
        # made a safety property depend on two OTHER checks staying correct,
        # and this feature's own history is a list of exactly that going wrong.
        if origin == "review" and event.get("severity") != "hard":
            saw_review = True
        # An explicitly-skipped verify satisfies the requirement: the emergency
        # escape hatch must be able to ship, or it is a deadlock rather than a
        # hatch. The skip is recorded in the journal and surfaces in the ledger.
        # A `verify -> stop` transition is the failure, not the verification,
        # so it must not satisfy the requirement that verify actually ran.
        if (origin == "verify" and event.get("severity") != "hard") or event.get("rule") in {
            "IMPL-OK-SKIP-VERIFY",
            "VERIFY-ENV-DEGRADED-ALLOWED",
        }:
            saw_verify = True
        expected = str(event.get("next"))
    if not saw_review:
        problems.append("no review phase was ever entered")
    if not saw_verify:
        problems.append("no verify phase was ever entered (or explicitly skipped)")
    return problems


def state_path(spec_dir: Path) -> Path:
    return spec_forge_dir(spec_dir) / "state.json"


def read_state(spec_dir: Path) -> dict[str, Any]:
    path = state_path(spec_dir)
    if not path.is_file():
        return {}
    try:
        return json.loads(path.read_text())
    except json.JSONDecodeError as exc:
        raise ForgeControlError(f"corrupt phase state {path}: {exc}") from exc


def phase_state(root: Path, spec_dir: Path) -> dict[str, Any]:
    """Authoritative phase state, re-readable without any conversation context.

    SKILL.md instructs every runtime to re-read this at the start of each
    phase and "go where it says". Reading `phase`/`blockers`/`reviewed_heads`
    from state.json alone made this command misreport itself: deleting the
    cache reported `phase: "intake"` and `blockers: []` while the journal
    still recorded `review` with an open REVIEW-SECURITY-HOLD -- the same
    defect fixed for ship_gate() and current_phase(), just never applied here.
    A coordinator that trusted this output would re-run intake on a
    security-held feature. JOURNAL FIRST, via the same helpers ship_gate uses.
    """
    machine = load_phase_machine(root)
    state = read_state(spec_dir)
    events = read_events(spec_dir)
    current = current_phase(machine, state, events)
    blockers = journal_blockers(events)
    return {
        "schema": "forge.phase-state/v1",
        "spec_dir": relative_display(root, spec_dir.resolve()),
        "phase": current,
        "last_result": state.get("last_result"),
        "last_rule": state.get("last_rule"),
        "shippable": bool(state.get("shippable", False)) and not blockers,
        "blockers": blockers,
        "flags": state.get("flags", []),
        "reviewed_heads": authoritative_reviewed_heads(state, events),
        "transitions": len(events),
        "updated_at": state.get("updated_at"),
        "terminal": current in machine["terminal"],
    }


def phase_advance(root: Path, ns: argparse.Namespace) -> dict[str, Any]:
    machine = load_phase_machine(root)
    spec_dir = resolve_spec_dir(root, ns.spec_dir, ns.feature_id)
    events = read_events(spec_dir)
    state = read_state(spec_dir)
    feature_id = ns.feature_id or spec_dir.name
    repos = feature_repos(root, feature_id, repo_names(ns.repo))

    # A transition is only legal from the phase the machine is actually in.
    # Without this the table validates each step in isolation, letting a caller
    # skip straight to ship and bypass every hard stop by never visiting the
    # phase that raises it.
    at = current_phase(machine, state, events)
    if at in machine["terminal"] and at != "done":
        # `stop` is terminal by design, but a feature whose blocker has been
        # attested must be able to continue -- otherwise the safety control
        # becomes a permanent dead end. Re-entry is earned, not free: every
        # hard blocker must already be attested away.
        if journal_blockers(events):
            raise ForgeControlError(
                f"the machine is at '{at}' with open blockers; an operator must run "
                "phase-attest before this feature can advance again"
            )
        # Resume at the phase that RAISED the stop, not one the caller picks.
        # Letting the caller choose meant a VERIFY-FAIL could re-enter at
        # `review` and reach ship without ever passing verification.
        at = machine["initial"]
        for event in reversed(events):
            if event.get("kind"):
                continue
            if event.get("severity") == "hard":
                at = str(event.get("from") or machine["initial"])
                break

    if ns.phase != at:
        raise ForgeControlError(
            f"out-of-sequence transition: the machine is at '{at}', not '{ns.phase}'. "
            f"Advance from '{at}', or resume with the phase the journal records."
        )

    observed = observe_heads(root, feature_id, repos)
    prior_reviewed = authoritative_reviewed_heads(state, events)
    drift = detect_drift(observed, prior_reviewed) if ns.phase == "ship" else []
    dirty = observe_dirty(root, feature_id, repos) if ns.phase == "ship" else []

    facts: dict[str, Any] = {
        "consecutive_failures": consecutive_failures(events, ns.phase, ns.result),
        "skip_verify": ns.skip_verify,
        "fixes_applied": ns.fixes_applied,
        "has_security_residual": ns.security_residual,
        "high_risk_uncovered": ns.high_risk_uncovered,
        "max_residual_severity": ns.max_residual_severity,
        "drift_detected": bool(drift),
        "dirty_worktrees": bool(dirty),
    }

    rule = evaluate_transition(machine, ns.phase, ns.result, facts)
    next_phase = rule["next"]
    severity = rule.get("severity")

    # Blockers are reconstructed from the append-only journal, never carried
    # forward from the mutable state cache.
    blockers = journal_blockers(events)
    if severity == "hard":
        blockers.append(
            {"rule": rule["id"], "phase": ns.phase, "reason": rule["reason"], "at": now()}
        )
    flags = list(state.get("flags", []))
    if severity == "flagged":
        flags.append({"rule": rule["id"], "phase": ns.phase, "reason": rule["reason"]})

    carried = dict(prior_reviewed)
    if ns.phase == "review" and observed:
        carried = observed

    shippable = next_phase in {"ship", "done"} and severity != "hard" and not blockers

    event = {
        "schema": "forge.phase-event/v1",
        "at": now(),
        "feature_id": feature_id,
        "from": ns.phase,
        "result": ns.result,
        "rule": rule["id"],
        "next": next_phase,
        "severity": severity or "normal",
        "reason": rule["reason"],
        "facts": facts,
        "heads": observed,
        "drift": drift,
        "dirty": dirty,
    }
    anchored = append_event(spec_dir, event, root=root, repos=repos)

    updated = {
        "schema": "forge.phase-state/v1",
        "feature_id": feature_id,
        "phase": next_phase,
        "last_result": ns.result,
        "last_rule": rule["id"],
        "shippable": shippable,
        "blockers": blockers,
        "flags": flags,
        "reviewed_heads": carried,
        "repos": repos,
        "updated_at": event["at"],
    }
    write_json_atomically(state_path(spec_dir), updated)

    # Every edge of the machine leaves an immutable, content-addressed record.
    # Best-effort: the journal above is authoritative for gating, so a failure
    # here is reported, never a reason to wedge a run.
    anchor = anchored.get("anchor")

    return {
        "next": next_phase,
        "rule": rule["id"],
        "severity": severity or "normal",
        "reason": rule["reason"],
        "shippable": shippable,
        "blockers": blockers,
        "flags": flags,
        "drift": drift,
        "dirty": dirty,
        "anchor": anchor or {"status": "not-recorded"},
        "valid": severity != "hard",
    }


def ship_gate(root: Path, spec_dir: Path, feature_id: str | None) -> dict[str, Any]:
    """Fail-closed precondition check used by the runtime deny-hooks."""
    machine = load_phase_machine(root)
    try:
        repositories = load_repositories(root)
    except (OSError, ForgeControlError):
        repositories = {}
    state = read_state(spec_dir)
    events = read_events(spec_dir)
    reasons: list[str] = []
    evaluated: set[str] = set()

    if not state and not events:
        return {
            "allowed": False,
            "valid": False,
            "reasons": [
                "no Forge phase journal for this feature; run forge-control.py phase-advance "
                "through the phases before pushing or opening a PR"
            ],
        }

    # Derive from the journal first: state.json is a cache and may be stale,
    # absent, or hand-edited. The journal is append-only and authoritative.
    #
    # `events[-1]` is NOT the last transition. `phase-sign` appends a `kind:
    # "signing"` record at the natural end of every signed run -- no
    # `from`/`next`, so `events[-1].get("next")` silently returned None and
    # fell through to `state.get("phase")`, the agent-writable cache, for
    # exactly the fact this function exists to take from the journal instead.
    # One hand-edited state.json plus the signing step every run performs
    # anyway reported `phase: "ship"` over an open REVIEW-SECURITY-HOLD.
    # `last_transition_event` skips every `kind`-tagged annotation, matching
    # what `current_phase()` (used by `phase_advance`) already does -- the two
    # must never diverge on what "the last transition" means.
    last_event = last_transition_event(events)
    # JOURNAL FIRST. `state.get("phase") or ...` let a one-key edit to the
    # agent-writable cache move a mid-flight run into a shipping phase, with
    # every tamper-evidence control still green because the journal was never
    # touched. The cache is consulted only when the journal records no
    # transition at all, and a disagreement is itself a deny reason below.
    phase = last_event.get("next") or state.get("phase")
    evaluated.add("phase-is-shipping")
    if phase not in {"ship", "done"}:
        reasons.append(f"phase is '{phase}', not a shipping phase")
    evaluated.add("state-shippable")
    if state and not state.get("shippable"):
        reasons.append("state is not marked shippable")

    # A cache that CONTRADICTS the journal is not stale -- it was edited. The
    # journal now wins everywhere, so a contradiction can no longer change the
    # verdict, but silently overriding it would discard the one signal that
    # says someone hand-edited the run. Surface it as a deny reason.
    evaluated.add("cache-agrees-with-journal")
    if state and last_event.get("next") and state.get("phase"):
        if state["phase"] != last_event["next"]:
            reasons.append(
                f"state.json says phase '{state['phase']}' but the journal's last "
                f"transition says '{last_event['next']}' -- the cache was edited"
            )
    cached_heads = state.get("reviewed_heads") or {}
    journal_heads = reviewed_heads(events)
    if journal_heads and cached_heads:
        conflicting = sorted(
            repo for repo, head in cached_heads.items()
            if repo in journal_heads and journal_heads[repo] != head
        )
        if conflicting:
            reasons.append(
                "state.json reviewed_heads disagree with the journal for: "
                f"{', '.join(conflicting)} -- the cache was edited"
            )

    blockers = journal_blockers(events)
    evaluated.add("no-open-blocker")
    for blocker in blockers:
        reasons.append(f"open blocker {blocker.get('rule')}: {blocker.get('reason')}")

    # Arriving at `ship` is not enough -- the path there must be intact.
    evaluated.add("sequence-contiguous")
    reasons.extend(sequence_is_contiguous(machine, events))

    identifier = feature_id or state.get("feature_id") or spec_dir.name

    # The immutable ref-commit chain is the strongest anchor available: a
    # content-addressed object cannot be edited in place the way a JSON file
    # sitting beside the journal can.
    evaluated.add("journal-anchored")
    reasons.extend(verify_transition_anchor(root, spec_dir, identifier))
    # An unsigned chain is tamper-EVIDENCE; a signed one is a tamper BARRIER an
    # agent cannot forge. The verdict must say which it had, or "anchored"
    # silently overstates itself -- the mistake this feature made twice.
    anchor_mode = anchor_signing_mode(root, identifier)

    # A journal that belongs to a different feature is not this feature's state.
    evaluated.add("feature-identity")
    stamped = {event.get("feature_id") for event in events if event.get("feature_id")}
    if stamped and identifier not in stamped:
        reasons.append(
            f"journal belongs to feature {sorted(stamped)}, not '{identifier}'"
        )
    for blocker in foreign_feature_blockers(root, identifier, spec_dir):
        reasons.append(
            f"open blocker {blocker.get('rule')} recorded for this feature in "
            f"{blocker.get('journal')}"
        )

    repos = feature_repos(
        root,
        identifier,
        list(state.get("repos") or {repo for event in events for repo in (event.get("heads") or {})}),
    )
    observed = observe_heads(root, identifier, repos)
    reviewed = authoritative_reviewed_heads(state, events)
    drift = detect_drift(observed, reviewed)
    evaluated.add("no-drift")
    evaluated.add("no-dirty-worktree")

    # Deferring signing to run close only works if the gate insists it happened.
    inspected = 0
    for repo in repos:
        worktree = root / "worktrees" / identifier / repo
        if not worktree.is_dir():
            continue
        base = repo_base_ref(root, repositories, worktree, repo)
        if base is None:
            # Fail closed: an unresolvable baseline is a verdict of "cannot
            # verify", never a silent pass. Five declared repos base on
            # main/master, and the old hardcoded origin/develop skipped them.
            reasons.append(f"cannot establish a signing baseline for {repo}")
            continue
        try:
            pending = unsigned_feature_commits(worktree, base)
        except ForgeControlError as exc:
            reasons.append(f"cannot verify signatures in {repo}: {exc}")
            continue
        inspected += 1
        if pending:
            reasons.append(
                f"{len(pending)} unsigned commit(s) in {repo}; run "
                f"`forge-control.py phase-sign --spec-dir specs/{identifier}`"
            )
    # The claim above ("the reasons carry a cannot-verify verdict") was false
    # for one case: a declared repo whose worktree directory never existed at
    # all. The loop's `if not worktree.is_dir(): continue` skips it silently
    # -- no reason appended, `inspected` never incremented -- so a feature
    # that declared repos but never created their worktrees passed
    # commits-signed with zero commits actually checked. Denying here closes
    # that gap without narrowing the legitimate zero-repo case (repos == []
    # is handled separately and correctly denies "no journal" upstream).
    if repos and not inspected:
        reasons.append(
            "no feature worktree could be inspected for signed commits "
            f"(declared repos: {', '.join(repos)})"
        )
    evaluated.add("commits-signed")

    # SHIP-DIRTY was only computed inside the ship transition, so skipping that
    # advance pushed uncommitted work straight past it. The gate re-derives it.
    for repo in observe_dirty(root, identifier, repos):
        reasons.append(f"uncommitted changes in {repo}; implementers must commit their work")
    if drift:
        reasons.append(f"post-review drift in: {', '.join(drift)}")

    # Every declared precondition must have been evaluated above. The table's
    # checks list is the contract; silently dropping one is how corr-hq-3 and
    # corr-hq-6 both arose.
    declared = {check["id"] for check in machine["ship_preconditions"]["checks"]}
    missing = declared - set(evaluated)
    if missing:
        raise ForgeControlError(
            f"ship_gate did not evaluate declared preconditions: {sorted(missing)}"
        )

    return {
        "allowed": not reasons,
        "valid": not reasons,
        "phase": phase,
        "drift": drift,
        "terminal": phase in machine["terminal"] if phase else False,
        "preconditions_evaluated": sorted(evaluated),
        "anchor_mode": anchor_mode,
        # Non-zero means the chain is knowingly incomplete: some transitions
        # failed to anchor (or a gap was planted). Not blocking -- anchoring is
        # best-effort by contract -- but a ledger that omitted it would imply
        # complete evidence it does not have.
        "anchor_gaps": anchor_gaps(spec_dir),
        # Non-zero means the chain was REBUILT. A rebuilt chain is weaker
        # evidence than an organic one, and nothing else downstream reveals
        # that, so an auditor must be able to see it without reading refs.
        "anchor_repairs": anchor_repairs(spec_dir),
        "reasons": reasons,
    }


def summarize_handoffs(root: Path, inputs: list[Path]) -> dict[str, Any]:
    paths: list[Path] = []
    for item in inputs:
        item = item.resolve()
        if item.is_dir():
            paths.extend(sorted(item.rglob("*.md")))
        else:
            paths.append(item)
    paths = list(dict.fromkeys(paths))
    if not paths:
        raise ForgeControlError("no handoff files found")
    providers: dict[str, dict[str, Any]] = {}
    known_cost = 0.0
    unpriced = 0
    for path in paths:
        report = verify_handoff(root, path)
        if not report["valid"]:
            raise ForgeControlError(f"invalid handoff {path}: {'; '.join(report['errors'])}")
        metadata = report["metadata"]
        provider = str(metadata["provider"])
        aggregate = providers.setdefault(
            provider,
            {"invocations": 0, "usage": {}, "known_cost_usd": 0.0, "unpriced_invocations": 0},
        )
        aggregate["invocations"] += 1
        for key, amount in numeric_usage(metadata.get("usage", {})).items():
            aggregate["usage"][key] = aggregate["usage"].get(key, 0) + amount
        amount = metadata.get("cost", {}).get("amount")
        if isinstance(amount, (int, float)) and not isinstance(amount, bool):
            known_cost += float(amount)
            aggregate["known_cost_usd"] += float(amount)
        else:
            unpriced += 1
            aggregate["unpriced_invocations"] += 1
    return {
        "schema": "forge.economics-summary/v1",
        "handoffs": len(paths),
        "known_cost_usd": round(known_cost, 10),
        "unpriced_invocations": unpriced,
        "providers": providers,
    }


def add_dirs_for_bind(command: list[str], anchor: Path, target: Path, created: set[Path]) -> None:
    """Create target parents inside an already-created sandbox anchor."""
    relative = target.resolve().relative_to(anchor.resolve())
    current = anchor.resolve()
    for part in relative.parts:
        current = current / part
        if current not in created:
            command.extend(["--dir", str(current)])
            created.add(current)


def git_text(cwd: Path, arguments: list[str], timeout: int = 30) -> str:
    """Run a read-only Git inspection and fail closed on every error."""
    git = shutil.which("git")
    if git is None:
        raise ForgeControlError("cannot attest DeepSeek worktree: git is unavailable")
    try:
        result = subprocess.run(
            [git, "-C", str(cwd), *arguments],
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
            env=git_env(),
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise ForgeControlError(f"cannot attest DeepSeek worktree {cwd}: {exc}") from exc
    if result.returncode != 0:
        detail = result.stderr.strip()[-500:]
        raise ForgeControlError(f"cannot attest DeepSeek worktree {cwd}: {detail or 'Git failed'}")
    return result.stdout.strip()


def git_binary(cwd: Path, arguments: list[str], timeout: int = 30) -> bytes:
    """Run a binary Git inspection and fail closed on every error."""
    git = shutil.which("git")
    if git is None:
        raise ForgeControlError("cannot attest DeepSeek worktree: git is unavailable")
    try:
        result = subprocess.run(
            [git, "-C", str(cwd), *arguments],
            capture_output=True,
            timeout=timeout,
            check=False,
            env=git_env(),
        )
    except (OSError, subprocess.TimeoutExpired) as exc:
        raise ForgeControlError(f"cannot attest DeepSeek worktree {cwd}: {exc}") from exc
    if result.returncode != 0:
        detail = result.stderr.decode(errors="replace").strip()[-500:]
        raise ForgeControlError(f"cannot attest DeepSeek worktree {cwd}: {detail or 'Git failed'}")
    return result.stdout


def resolve_git_path(top: Path, raw: str) -> Path:
    path = Path(raw)
    return path.resolve() if path.is_absolute() else (top / path).resolve()


def attest_deepseek_worktree(worktree: Path, require_dedicated: bool) -> tuple[Path, Path]:
    """Prove a mount contains only tracked content in a dedicated clean worktree."""
    top = Path(git_text(worktree, ["rev-parse", "--show-toplevel"])).resolve()
    git_dir = resolve_git_path(
        top, git_text(top, ["rev-parse", "--path-format=absolute", "--git-dir"])
    )
    common_dir = resolve_git_path(
        top, git_text(top, ["rev-parse", "--path-format=absolute", "--git-common-dir"])
    )
    if require_dedicated and git_dir == common_dir:
        raise ForgeControlError(
            f"DeepSeek requires a dedicated linked worktree, not the base clone: {top}"
        )
    dirty = git_binary(top, ["status", "--porcelain=v1", "-z", "--untracked-files=all"])
    ignored = git_binary(top, ["ls-files", "--others", "--ignored", "--exclude-standard", "-z"])
    if dirty or ignored:
        names = []
        for entry in (dirty + ignored).split(b"\0"):
            if entry:
                names.append(entry.decode(errors="replace"))
        sample = ", ".join(names[:5])
        raise ForgeControlError(
            f"DeepSeek requires an attested clean worktree; local files are present in {top}: {sample}"
        )
    return top, common_dir


def validate_deepseek_cwd(
    root: Path,
    cwd: Path,
    selected_repos: list[str],
    repositories: dict[str, dict[str, Any]],
    write: bool,
) -> Path:
    """Bind cwd only when Git proves it belongs to an approved public repo."""
    cwd = cwd.resolve()
    if cwd == root.resolve():
        raise ForgeControlError(
            "DeepSeek requires a dedicated public repository worktree, not the workspace root"
        )
    try:
        cwd.relative_to(root.resolve())
    except ValueError as exc:
        raise ForgeControlError("DeepSeek cwd must be inside the workspace boundary") from exc
    _, common = attest_deepseek_worktree(cwd, require_dedicated=True)
    approved = {
        (root / str(repositories[name].get("dir") or name) / ".git").resolve()
        for name in selected_repos
    }
    if common not in approved:
        raise ForgeControlError(
            f"DeepSeek cwd belongs to unapproved Git metadata: {common}; "
            "the declared public repository boundary does not match"
        )
    return common


def deepseek_environment(source: dict[str, str], auth_token: str, runner_model: str) -> dict[str, str]:
    """Pass no ambient credentials or provider configuration to DeepSeek."""
    allowed = {
        "HOME",
        "LANG",
        "LC_ALL",
        "LC_CTYPE",
        "LOGNAME",
        "NO_COLOR",
        "SHELL",
        "TERM",
        "TZ",
        "USER",
    }
    env = {key: value for key, value in source.items() if key in allowed}
    env.update(
        {
            "PATH": f"/forge-bin:{Path.home()}/.volta/bin:/usr/local/bin:/usr/bin:/bin",
            "ANTHROPIC_BASE_URL": "https://api.deepseek.com/anthropic",
            "ANTHROPIC_AUTH_TOKEN": auth_token,
            "ANTHROPIC_MODEL": runner_model,
            "ANTHROPIC_DEFAULT_OPUS_MODEL": runner_model,
            "ANTHROPIC_DEFAULT_SONNET_MODEL": runner_model,
            "ANTHROPIC_DEFAULT_HAIKU_MODEL": "deepseek-v4-flash",
            "CLAUDE_CODE_SUBPROCESS_ENV_SCRUB": "1",
            "CLAUDE_CONFIG_DIR": "/forge-state",
        }
    )
    return env


def build_deepseek_bwrap(
    root: Path,
    cwd: Path,
    base_command: list[str],
    selected_repos: list[str],
    repositories: dict[str, dict[str, Any]],
    write: bool,
    state_dir: Path,
) -> list[str]:
    if shutil.which("bwrap") is None:
        raise ForgeControlError("DeepSeek execution requires bubblewrap (bwrap) for filesystem confinement")
    state_dir.mkdir(parents=True, exist_ok=True)
    cwd = cwd.resolve()
    common_git_dir = validate_deepseek_cwd(root, cwd, selected_repos, repositories, write)
    home = Path.home().resolve()
    root = root.resolve()
    try:
        root.relative_to(home)
    except ValueError as exc:
        raise ForgeControlError("DeepSeek confinement requires the workspace to be beneath HOME") from exc

    claude_binary = Path(shutil.which("claude") or "").resolve()
    if not claude_binary.is_file():
        raise ForgeControlError("DeepSeek execution requires a resolvable Claude runner binary")
    rg_binary = Path(shutil.which("rg") or "")
    command = [
        "bwrap",
        "--die-with-parent",
        "--new-session",
        "--unshare-pid",
        "--unshare-ipc",
        "--unshare-uts",
        "--ro-bind",
        "/usr",
        "/usr",
        "--symlink",
        "usr/bin",
        "/bin",
        "--symlink",
        "usr/sbin",
        "/sbin",
        "--symlink",
        "usr/lib",
        "/lib",
        "--symlink",
        "usr/lib64",
        "/lib64",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--tmpfs",
        "/tmp",
        "--dir",
        "/etc",
    ]
    for system_path in (
        Path("/etc/ssl"),
        Path("/etc/resolv.conf"),
        Path("/etc/hosts"),
        Path("/etc/nsswitch.conf"),
        Path("/etc/passwd"),
        Path("/etc/group"),
    ):
        if system_path.exists():
            command.extend(["--ro-bind", str(system_path), str(system_path)])

    command.extend(["--dir", "/forge-bin", "--ro-bind", str(claude_binary), "/forge-bin/claude"])
    if rg_binary.is_file():
        command.extend(["--ro-bind", str(rg_binary.resolve()), "/forge-bin/rg"])

    # HOME is a fresh tmpfs. Only explicit toolchains and eligible repositories
    # are mounted back into it; unrelated clones and credentials never appear.
    command.extend(["--dir", str(home.parent), "--tmpfs", str(home)])
    created = {home, home.parent}
    add_dirs_for_bind(command, home, root, created)
    volta = home / ".volta"
    if volta.is_dir():
        add_dirs_for_bind(command, home, volta, created)
        command.extend(["--ro-bind", str(volta), str(volta)])

    bound: set[Path] = set()
    mounted_trees: set[Path] = set()
    for name in selected_repos:
        repo = repositories[name]
        repo_dir = (root / str(repo.get("dir") or name)).resolve()
        if common_git_dir == (repo_dir / ".git").resolve():
            # The dedicated cwd already provides this repository's files. Do
            # not expose or require cleanliness of its developer base clone.
            continue
        if repo_dir.exists():
            attest_deepseek_worktree(repo_dir, require_dedicated=False)
            add_dirs_for_bind(command, home, repo_dir, created)
            repo_write = write and (cwd == repo_dir or repo_dir in cwd.parents)
            command.extend(["--bind" if repo_write else "--ro-bind", str(repo_dir), str(repo_dir)])
            bound.add(repo_dir)
            mounted_trees.add(repo_dir)

    if cwd != root and not any(cwd == repo_dir or repo_dir in cwd.parents for repo_dir in bound):
        add_dirs_for_bind(command, home, cwd, created)
        command.extend(["--bind" if write else "--ro-bind", str(cwd), str(cwd)])
        mounted_trees.add(cwd)

    # Git has attested that no tracked modifications, untracked files, or
    # ignored files exist. Hide all local Git metadata as it can still contain
    # unpublished objects, credentials, remotes, and hooks.
    for tree in mounted_trees:
        git_entry = tree / ".git"
        if git_entry.is_dir():
            command.extend(["--tmpfs", str(git_entry)])
        elif git_entry.is_file():
            command.extend(["--ro-bind", "/dev/null", str(git_entry)])

    command.extend(["--dir", "/forge-state", "--bind", str(state_dir), "/forge-state"])
    command.extend(["--chdir", str(cwd), "--"])
    base_command[0] = "/forge-bin/claude"
    command.extend(base_command)
    return command


class NoRedirect(urllib.request.HTTPRedirectHandler):
    """Keep public Forge payloads on the explicitly attested Model Studio host."""

    def redirect_request(self, req: Any, fp: Any, code: int, msg: str, headers: Any, newurl: str) -> None:
        return None


def modelstudio_base_url(rules: dict[str, Any], environment: dict[str, str]) -> str:
    raw = environment.get(str(rules["base_url_env"])) or str(rules["default_base_url"])
    parsed = urllib.parse.urlsplit(raw)
    host = (parsed.hostname or "").lower()
    exact = set(rules.get("allowed_base_url_hosts", []))
    suffixes = tuple(rules.get("allowed_base_url_suffixes", []))
    if (
        parsed.scheme != "https"
        or not host
        or parsed.username is not None
        or parsed.password is not None
        or parsed.query
        or parsed.fragment
        or (host not in exact and not any(host.endswith(suffix) for suffix in suffixes))
    ):
        raise ForgeControlError("DASHSCOPE_BASE_URL must be an approved HTTPS Model Studio endpoint")
    path = parsed.path.rstrip("/")
    if not (path.endswith("/compatible-mode/v1") or path == "/v1"):
        raise ForgeControlError(
            "DASHSCOPE_BASE_URL must end in /compatible-mode/v1 (or /v1 for a coding plan)"
        )
    return urllib.parse.urlunsplit(("https", parsed.netloc, path, "", ""))


def qwen_effort(effort: str) -> str:
    return {"low": "minimal", "medium": "medium"}.get(effort, "high")


def qwen_public_root(root: Path, feature: str) -> Path:
    if re.fullmatch(r"[a-zA-Z0-9._-]+", feature) is None:
        raise ForgeControlError("Qwen public evidence requires a safe --feature identifier")
    lexical = root / "specs" / feature / "forge" / "public-evidence"
    cursor = lexical
    while cursor != root and cursor.parent != cursor:
        if cursor.is_symlink():
            raise ForgeControlError("Qwen public evidence path cannot contain symlinks")
        cursor = cursor.parent
    return lexical.resolve()


def qwen_public_path(
    root: Path, cwd: Path, feature: str, raw: str, label: str
) -> tuple[Path, Path, Path]:
    """Resolve a symlink-free path within one feature's public evidence root."""
    public_root = qwen_public_root(root, feature)
    supplied = Path(raw)
    lexical = supplied if supplied.is_absolute() else cwd.resolve() / supplied
    cursor = lexical
    while True:
        if cursor.is_symlink():
            raise ForgeControlError(f"Qwen {label} path cannot contain symlinks: {raw}")
        if cursor == root or cursor.parent == cursor:
            break
        cursor = cursor.parent
    resolved = lexical.resolve()
    try:
        relative = resolved.relative_to(public_root)
    except ValueError as exc:
        raise ForgeControlError(
            f"Qwen {label} must be staged under {public_root}: {raw}"
        ) from exc
    if not resolved.is_file():
        raise ForgeControlError(f"Qwen {label} is not a regular file: {raw}")
    return public_root, relative, resolved


def load_qwen_public_manifest(public_root: Path) -> dict[tuple[str, str], str]:
    """Load the coordinator's durable public-content attestations."""
    manifest_path = public_root / "manifest.json"
    if manifest_path.is_symlink() or not manifest_path.is_file():
        raise ForgeControlError(f"Qwen public evidence manifest is missing: {manifest_path}")
    try:
        manifest = json.loads(manifest_path.read_text())
    except (OSError, json.JSONDecodeError) as exc:
        raise ForgeControlError(f"Qwen public evidence manifest is invalid: {exc}") from exc
    if not isinstance(manifest, dict) or manifest.get("schema") != "forge.public-evidence/v1":
        raise ForgeControlError("Qwen public evidence manifest has an unsupported schema")
    files = manifest.get("files")
    if not isinstance(files, list):
        raise ForgeControlError("Qwen public evidence manifest files must be an array")
    attestations: dict[tuple[str, str], str] = {}
    for index, entry in enumerate(files):
        if not isinstance(entry, dict) or set(entry) != {"path", "sha256", "kind"}:
            raise ForgeControlError(
                f"Qwen public evidence manifest entry {index} must contain only path, sha256, and kind"
            )
        path = entry["path"]
        digest = entry["sha256"]
        kind = entry["kind"]
        if (
            not isinstance(path, str)
            or not path
            or Path(path).is_absolute()
            or ".." in Path(path).parts
            or not isinstance(kind, str)
            or kind not in {"prompt", "schema"}
            or re.fullmatch(r"[0-9a-f]{64}", str(digest)) is None
        ):
            raise ForgeControlError(f"Qwen public evidence manifest entry {index} is invalid")
        key = (kind, Path(path).as_posix())
        if key in attestations:
            raise ForgeControlError(f"Qwen public evidence manifest contains duplicate {kind}: {path}")
        attestations[key] = str(digest)
    return attestations


def attested_qwen_text_inputs(
    root: Path, ns: argparse.Namespace, rules: dict[str, Any]
) -> tuple[str, dict[str, Any] | None, list[dict[str, str]]]:
    """Read only text assets whose exact bytes are attested as public."""
    feature = str(getattr(ns, "feature", "") or "")
    public_root = qwen_public_root(root, feature)
    manifest = load_qwen_public_manifest(public_root)
    assets: list[dict[str, str]] = []

    def read_attested(raw: str, kind: str) -> bytes:
        _, relative, path = qwen_public_path(
            root, Path(ns.cwd), feature, raw, f"{kind} input"
        )
        relative_name = relative.as_posix()
        expected = manifest.get((kind, relative_name))
        if expected is None:
            raise ForgeControlError(
                f"Qwen {kind} input lacks a public manifest attestation: {relative_name}"
            )
        content = path.read_bytes()
        if not content or len(content) > int(rules["max_prompt_bytes"]):
            raise ForgeControlError(
                f"Qwen {kind} input must be between 1 and {rules['max_prompt_bytes']} bytes"
            )
        actual = sha256_bytes(content)
        if actual != expected:
            raise ForgeControlError(f"Qwen {kind} input hash does not match its public attestation")
        assets.append(
            {
                "path": str(Path("specs") / feature / "forge" / "public-evidence" / relative),
                "sha256": actual,
                "media_type": "application/schema+json" if kind == "schema" else "text/plain",
            }
        )
        return content

    try:
        prompt = read_attested(str(ns.prompt_file), "prompt").decode("utf-8")
    except UnicodeDecodeError as exc:
        raise ForgeControlError("Qwen prompt input must be valid UTF-8") from exc
    schema = None
    if ns.schema_file:
        try:
            parsed = json.loads(read_attested(str(ns.schema_file), "schema").decode("utf-8"))
        except (UnicodeDecodeError, json.JSONDecodeError) as exc:
            raise ForgeControlError("Qwen schema input must be valid UTF-8 JSON") from exc
        if not isinstance(parsed, dict):
            raise ForgeControlError("Qwen schema input must contain a JSON object")
        schema = parsed
    return prompt, schema, assets


def public_qwen_images(
    root: Path, ns: argparse.Namespace, rules: dict[str, Any]
) -> tuple[list[dict[str, Any]], list[dict[str, str]]]:
    raw_images = list(getattr(ns, "image", []) or [])
    if not raw_images:
        return [], []
    feature = str(getattr(ns, "feature", "") or "")
    max_images = int(rules["max_images"])
    max_image_bytes = int(rules["max_image_bytes"])
    max_total_bytes = int(rules["max_total_image_bytes"])
    if len(raw_images) > max_images:
        raise ForgeControlError(f"Qwen image input exceeds the {max_images}-image Forge cap")
    content: list[dict[str, Any]] = []
    provenance: list[dict[str, str]] = []
    total = 0
    for raw in raw_images:
        _, relative, image = qwen_public_path(
            root, Path(ns.cwd), feature, raw, "image input"
        )
        image_bytes = image.read_bytes()
        size = len(image_bytes)
        if size <= 0 or size > max_image_bytes:
            raise ForgeControlError(
                f"Qwen image input must be between 1 and {max_image_bytes} bytes: {raw}"
            )
        total += size
        if total > max_total_bytes:
            raise ForgeControlError(
                f"Qwen image input exceeds the {max_total_bytes}-byte aggregate Forge cap"
            )
        mime, _ = mimetypes.guess_type(image.name)
        if mime not in set(rules["allowed_image_media_types"]):
            raise ForgeControlError(f"Qwen image input has an unsupported media type: {raw}")
        encoded = base64.b64encode(image_bytes).decode()
        content.append(
            {"type": "image_url", "image_url": {"url": f"data:{mime};base64,{encoded}"}}
        )
        provenance.append(
            {
                "path": str(Path("specs") / feature / "forge" / "public-evidence" / relative),
                "sha256": sha256_bytes(image_bytes),
                "media_type": mime,
            }
        )
    return content, provenance


def parse_qwen_result(data: dict[str, Any], multimodal: bool) -> tuple[str, str | None, dict[str, Any]]:
    if multimodal:
        choices = data.get("choices")
        if not isinstance(choices, list) or not choices or not isinstance(choices[0], dict):
            raise ForgeControlError("Qwen returned no Chat Completions choice")
        message = choices[0].get("message")
        text = message.get("content") if isinstance(message, dict) else None
    else:
        fragments = []
        for item in data.get("output", []):
            if not isinstance(item, dict) or item.get("type") != "message":
                continue
            for content in item.get("content", []):
                if isinstance(content, dict) and content.get("type") == "output_text":
                    fragments.append(str(content.get("text") or ""))
        text = "\n".join(fragment for fragment in fragments if fragment)
    if not isinstance(text, str) or not text.strip():
        raise ForgeControlError("Qwen returned no final text")
    return text, str(data["id"]) if data.get("id") else None, data.get("usage") or {}


def structured_result(text: str, schema: dict[str, Any] | None) -> str:
    if schema is None:
        return text if text.endswith("\n") else text + "\n"
    candidate = text.strip()
    if candidate.startswith("```") and candidate.endswith("```"):
        lines = candidate.splitlines()
        candidate = "\n".join(lines[1:-1])
    try:
        value = json.loads(candidate)
    except json.JSONDecodeError as exc:
        raise ForgeControlError("Qwen output did not satisfy the requested JSON contract") from exc
    errors = schema_errors(value, schema)
    if errors:
        raise ForgeControlError("Qwen output schema mismatch: " + "; ".join(errors))
    return json.dumps(value, indent=2, sort_keys=True) + "\n"


def call_modelstudio(
    endpoint: str,
    api_key: str,
    payload: dict[str, Any],
    timeout_seconds: int,
) -> dict[str, Any]:
    headers = {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
        "User-Agent": "alkemio-forge/1",
    }
    if endpoint.endswith("/responses"):
        headers["x-dashscope-session-cache"] = "enable"
    request = urllib.request.Request(
        endpoint,
        data=json.dumps(payload, separators=(",", ":")).encode(),
        headers=headers,
        method="POST",
    )
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    try:
        with opener.open(request, timeout=timeout_seconds) as response:
            raw = response.read(32 * 1024 * 1024 + 1)
    except urllib.error.HTTPError as exc:
        detail = exc.read(4096).decode(errors="replace").replace(api_key, "[REDACTED]")
        raise ForgeControlError(f"Model Studio returned HTTP {exc.code}: {detail}") from exc
    except (urllib.error.URLError, TimeoutError, OSError) as exc:
        raise ForgeControlError(f"Model Studio request failed: {exc}") from exc
    if len(raw) > 32 * 1024 * 1024:
        raise ForgeControlError("Model Studio response exceeded the 32 MiB Forge cap")
    try:
        data = json.loads(raw)
    except json.JSONDecodeError as exc:
        raise ForgeControlError("Model Studio returned non-JSON output") from exc
    if not isinstance(data, dict):
        raise ForgeControlError("Model Studio returned a non-object response")
    return data


def invocation_command(
    root: Path,
    policy: dict[str, Any],
    repositories: dict[str, dict[str, Any]],
    ns: argparse.Namespace,
) -> tuple[list[str], dict[str, str], list[str]]:
    cwd = Path(ns.cwd).resolve()
    selected = repo_names(ns.repo)
    env = dict(os.environ)
    secret_names: list[str] = []
    if ns.provider == "codex":
        command = ["codex", "exec"]
        if ns.resume_session:
            command.extend(["resume", ns.resume_session])
        command.extend(["--json", "--model", ns.model])
        if not ns.resume_session:
            command.extend(["--cd", str(cwd), "--sandbox", "workspace-write" if ns.write else "read-only"])
        if ns.schema_file:
            command.extend(["--output-schema", str(Path(ns.schema_file).resolve())])
        command.extend(["--output-last-message", str(Path(ns.result_file).resolve()), "-"])
        return command, env, secret_names

    if ns.provider not in {"claude", "deepseek"}:
        raise ForgeControlError(f"unsupported invocation provider: {ns.provider}")
    if ns.provider == "deepseek":
        # This policy gate intentionally precedes reading or serializing any
        # prompt so denied task classes cannot reach the external runner.
        eligibility = deepseek_eligibility(policy, repositories, ns.role, selected, ns.context_public)
        if not eligibility["eligible"]:
            raise ForgeControlError("DeepSeek invocation denied: " + "; ".join(eligibility["reasons"]))
        if ns.write and ns.role not in policy["providers"]["deepseek"]["allowed_write_roles"]:
            raise ForgeControlError(f"DeepSeek role '{ns.role}' is read-only by policy")
        auth_env = policy["providers"]["deepseek"]["auth_env"]
        if not env.get(auth_env):
            raise ForgeControlError(f"DeepSeek invocation requires {auth_env}")

    prompt = Path(ns.prompt_file).read_text()
    schema = json.loads(Path(ns.schema_file).read_text()) if ns.schema_file else None
    model = ns.model
    command = [
        "claude",
        "-p",
        "--output-format",
        "json",
        "--exclude-dynamic-system-prompt-sections",
        "--model",
        model,
        "--effort",
        ns.effort,
        "--permission-mode",
        "auto" if ns.write else "plan",
    ]
    if ns.resume_session:
        command.extend(["--resume", ns.resume_session])
    if schema is not None:
        command.extend(["--json-schema", json.dumps(schema, separators=(",", ":"))])
    command.append(prompt)
    if ns.provider == "claude":
        return command, env, secret_names
    runner_model = provider_runner_model(policy, "deepseek", model)
    command[command.index(model)] = runner_model
    command.insert(1, "--safe-mode")
    tools = "Read,Glob,Grep,Edit,Write,Bash" if ns.write else "Read,Glob,Grep"
    command[1:1] = ["--tools", tools]
    env = deepseek_environment(env, env[auth_env], runner_model)
    env["CLAUDE_CODE_EFFORT_LEVEL"] = "max" if ns.effort in {"xhigh", "max"} else "high"
    secret_names.extend([auth_env, "ANTHROPIC_AUTH_TOKEN"])
    state_dir = root / ".forge/state" / re.sub(r"[^a-zA-Z0-9._-]+", "-", ns.feature or "adhoc") / "deepseek"
    return build_deepseek_bwrap(root, cwd, command, selected, repositories, ns.write, state_dir), env, secret_names


def redact_command(command: list[str], secret_names: list[str]) -> dict[str, Any]:
    return {"argv": command, "secret_environment": sorted(set(secret_names))}


def redact_text(value: str, environment: dict[str, str], secret_names: list[str]) -> str:
    redacted = value
    for name in secret_names:
        secret = environment.get(name)
        if secret:
            redacted = redacted.replace(secret, "[REDACTED]")
    return redacted


def parse_invocation_output(
    provider: str, billing_mode: str, stdout: str, result_path: Path
) -> tuple[str | None, dict[str, Any], dict[str, Any], list[str]]:
    session_id = None
    usage: dict[str, Any] = {}
    observed_models: list[str] = []
    cost: dict[str, Any] = {
        "currency": "USD",
        "amount": None,
        "basis": "provider-did-not-report",
        "estimated": False,
    }
    if provider == "codex":
        for line in stdout.splitlines():
            try:
                event = json.loads(line)
            except json.JSONDecodeError:
                continue
            if event.get("type") == "thread.started":
                session_id = event.get("thread_id")
            if event.get("type") == "turn.completed":
                usage = event.get("usage") or usage
            if isinstance(event.get("model"), str):
                observed_models.append(event["model"])
        return session_id, usage, {
            **cost,
            "basis": (
                "subscription-no-marginal-price"
                if billing_mode == "subscription"
                else "provider-did-not-report"
            ),
        }, list(dict.fromkeys(observed_models))
    try:
        data = json.loads(stdout)
    except json.JSONDecodeError as exc:
        raise ForgeControlError(f"{provider} returned non-JSON output") from exc
    session_id = data.get("session_id")
    usage = data.get("usage") or {}
    model_usage = data.get("modelUsage") or data.get("model_usage") or {}
    if isinstance(model_usage, dict):
        observed_models.extend(str(name) for name in model_usage)
    if isinstance(data.get("model"), str):
        observed_models.append(data["model"])
    if data.get("total_cost_usd") is not None:
        cost = {
            "currency": "USD",
            "amount": data["total_cost_usd"],
            "basis": "runner-estimated-equivalent",
            "estimated": True,
        }
    elif billing_mode == "subscription":
        cost["basis"] = "subscription-no-marginal-price"
    result = data.get("structured_output")
    if result is None:
        result = data.get("result")
    if result is not None:
        if isinstance(result, str):
            rendered = result if result.endswith("\n") else result + "\n"
        else:
            rendered = json.dumps(result, indent=2, sort_keys=True) + "\n"
        atomic_replace(result_path, rendered.encode(), mode=0o644)
    return session_id, usage, cost, list(dict.fromkeys(observed_models))


def persist_invocation_failure(
    ns: argparse.Namespace,
    result_path: Path,
    kind: str,
    detail: str,
    returncode: int | None = None,
) -> None:
    """Persist a resumable failure record before surfacing an invocation error."""
    failure = {
        "schema": "forge.invocation-failure/v1",
        "status": "failed",
        "provider": ns.provider,
        "model_requested": ns.model,
        "effort_requested": ns.effort,
        "kind": kind,
        "detail": detail,
        "returncode": returncode,
        "completed_at": now(),
    }
    rendered = (json.dumps(failure, indent=2, sort_keys=True) + "\n").encode()
    atomic_replace(result_path, rendered, mode=0o644)
    if ns.metadata_file:
        metadata_path = Path(ns.metadata_file).resolve()
        atomic_replace(metadata_path, rendered, mode=0o644)


def terminate_process_group(process: subprocess.Popen[str]) -> tuple[str, str]:
    """Terminate a provider and all descendants, escalating after a short grace."""
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        return process.communicate(timeout=5)
    except subprocess.TimeoutExpired:
        try:
            os.killpg(process.pid, signal.SIGKILL)
        except ProcessLookupError:
            pass
        return process.communicate()


def invoke_qwen(
    root: Path,
    policy: dict[str, Any],
    repositories: dict[str, dict[str, Any]],
    ns: argparse.Namespace,
) -> dict[str, Any]:
    rules = policy["providers"]["qwen"]
    selected = repo_names(ns.repo)
    eligibility = qwen_eligibility(
        policy, repositories, ns.role, selected, ns.context_public
    )
    if not eligibility["eligible"]:
        raise ForgeControlError("Qwen invocation denied: " + "; ".join(eligibility["reasons"]))
    if ns.write:
        raise ForgeControlError("Qwen roles are read-only by policy")
    environment = dict(os.environ)
    auth_env = str(rules["auth_env"])
    api_key = environment.get(auth_env)
    if not api_key:
        raise ForgeControlError(f"Qwen invocation requires {auth_env}")
    base_url = modelstudio_base_url(rules, environment)
    prompt, output_schema, text_provenance = attested_qwen_text_inputs(root, ns, rules)
    images, image_provenance = public_qwen_images(root, ns, rules)
    provenance = [*text_provenance, *image_provenance]
    multimodal = bool(images)
    if multimodal and ns.resume_session:
        raise ForgeControlError("Qwen multimodal calls resume from written handoffs, not response IDs")
    endpoint = base_url + ("/chat/completions" if multimodal else "/responses")
    if ns.dry_run:
        return {
            "provider": "qwen",
            "dry_run": True,
            "endpoint": endpoint,
            "model": ns.model,
            "role": ns.role,
            "input_assets": provenance,
            "secret_environment": [auth_env],
        }

    schema_instruction = None
    if output_schema is not None:
        schema_instruction = (
            "Return only a JSON value that satisfies this JSON Schema; do not wrap it in "
            "Markdown fences:\n"
            + json.dumps(output_schema, separators=(",", ":"))
        )
    if multimodal:
        content = [
            {
                "type": "text",
                "text": prompt,
                "cache_control": {"type": "ephemeral"},
            },
            *images,
        ]
        payload: dict[str, Any] = {
            "model": ns.model,
            "messages": (
                [{"role": "system", "content": schema_instruction}]
                if schema_instruction
                else []
            ) + [{"role": "user", "content": content}],
            "enable_thinking": ns.effort != "low",
        }
        actual_effort = "none" if ns.effort == "low" else "max"
    else:
        payload = {
            "model": ns.model,
            "input": (
                [
                    {"role": "developer", "content": schema_instruction},
                    {"role": "user", "content": prompt},
                ]
                if schema_instruction
                else prompt
            ),
            "reasoning": {"effort": qwen_effort(ns.effort)},
        }
        if ns.resume_session:
            payload["previous_response_id"] = ns.resume_session
        actual_effort = qwen_effort(ns.effort)
    result_path = Path(ns.result_file).resolve()
    result_path.parent.mkdir(parents=True, exist_ok=True)
    try:
        data = call_modelstudio(endpoint, api_key, payload, ns.timeout_seconds)
    except ForgeControlError as exc:
        persist_invocation_failure(ns, result_path, "provider-error", str(exc))
        raise
    try:
        text, session_id, usage = parse_qwen_result(data, multimodal)
        rendered = structured_result(text, output_schema)
    except ForgeControlError as exc:
        persist_invocation_failure(ns, result_path, "invalid-output", str(exc))
        raise
    atomic_replace(result_path, rendered.encode(), mode=0o644)
    if session_id and ns.feature and ns.invocation:
        remember_session(root, ns.feature, ns.invocation, "qwen", session_id)
    model_actual = str(data.get("model") or ns.model)
    metadata = {
        "provider": "qwen",
        "model_requested": ns.model,
        "model_actual": model_actual,
        "models_observed": [model_actual],
        "effort_requested": ns.effort,
        "effort_actual": actual_effort,
        "runner": "modelstudio-openai-compatible",
        "billing_mode": "api",
        "session_fingerprint": f"sha256:{sha256_bytes((session_id or 'none').encode())}",
        "usage": normalize_usage(usage),
        "cost": {
            "currency": "USD",
            "amount": None,
            "basis": "tiered-provider-pricing-not-reported",
            "estimated": False,
        },
        "cache_mode": "explicit-prefix" if multimodal else "session-cache",
        "input_assets": provenance,
        "result_sha256": sha256_file(result_path),
    }
    if ns.metadata_file:
        path = Path(ns.metadata_file).resolve()
        atomic_replace(
            path, (json.dumps(metadata, indent=2, sort_keys=True) + "\n").encode(), mode=0o644
        )
    return metadata


def invoke(root: Path, policy: dict[str, Any], repositories: dict[str, dict[str, Any]], ns: argparse.Namespace) -> dict[str, Any]:
    if ns.provider == "qwen":
        return invoke_qwen(root, policy, repositories, ns)
    if getattr(ns, "image", None):
        raise ForgeControlError("--image is supported only by the confined Qwen adapter")
    command, env, secrets = invocation_command(root, policy, repositories, ns)
    if ns.dry_run:
        return {"provider": ns.provider, "dry_run": True, **redact_command(command, secrets)}
    result_path = Path(ns.result_file).resolve()
    result_path.parent.mkdir(parents=True, exist_ok=True)
    process = subprocess.Popen(
        command,
        cwd=Path(ns.cwd).resolve(),
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=env,
        start_new_session=True,
    )
    timeout_seconds = getattr(ns, "timeout_seconds", 3600)
    provider_input = "" if ns.provider != "codex" else Path(ns.prompt_file).read_text()
    try:
        stdout, stderr = process.communicate(input=provider_input, timeout=timeout_seconds)
    except subprocess.TimeoutExpired:
        stdout, stderr = terminate_process_group(process)
        detail = f"provider exceeded {timeout_seconds}s deadline"
        persist_invocation_failure(ns, result_path, "timeout", detail)
        raise ForgeControlError(f"{ns.provider} invocation timed out after {timeout_seconds}s")
    if process.returncode != 0:
        detail = redact_text(stderr[-2000:], env, secrets)
        persist_invocation_failure(ns, result_path, "nonzero-exit", detail, process.returncode)
        raise ForgeControlError(
            f"{ns.provider} invocation failed with exit {process.returncode}: {detail}"
        )
    try:
        session_id, usage, cost, observed_models = parse_invocation_output(
            ns.provider, ns.billing_mode, stdout, result_path
        )
        if not result_path.is_file():
            raise ForgeControlError(f"{ns.provider} invocation produced no result file")
    except ForgeControlError as exc:
        persist_invocation_failure(ns, result_path, "invalid-output", str(exc))
        raise
    if session_id and ns.feature and ns.invocation:
        remember_session(root, ns.feature, ns.invocation, ns.provider, session_id)
    metadata = {
        "provider": ns.provider,
        "model_requested": ns.model,
        "model_actual": observed_models[0] if len(observed_models) == 1 else ns.model,
        "models_observed": observed_models,
        "effort_requested": ns.effort,
        "effort_actual": ns.effort,
        "runner": "claude-anthropic-compatible" if ns.provider == "deepseek" else ns.provider,
        "billing_mode": ns.billing_mode,
        "session_fingerprint": f"sha256:{sha256_bytes((session_id or 'none').encode())}",
        "usage": normalize_usage(usage),
        "cost": cost,
        "result_sha256": sha256_file(result_path),
    }
    if ns.metadata_file:
        path = Path(ns.metadata_file).resolve()
        atomic_replace(path, (json.dumps(metadata, indent=2, sort_keys=True) + "\n").encode(), mode=0o644)
    return metadata


def parser() -> argparse.ArgumentParser:
    root_parser = argparse.ArgumentParser(description=__doc__)
    root_parser.add_argument("--workspace", help="workspace root or descendant")
    commands = root_parser.add_subparsers(dest="command", required=True)

    resolve = commands.add_parser("resolve-profile", help="resolve and preflight a provider profile")
    resolve.add_argument("--providers", default="auto")
    resolve.add_argument("--available", help="test-only availability override")

    eligible = commands.add_parser("check-eligibility", help="check confined API-provider eligibility")
    eligible.add_argument("--provider", default="deepseek", choices=["deepseek", "qwen"])
    eligible.add_argument("--role", required=True)
    eligible.add_argument("--repo", action="append", default=[])
    eligible.add_argument("--context-public", action="store_true")

    council = commands.add_parser("council-route", help="resolve deterministic council assignments")
    council.add_argument("--feature-id", required=True)
    council.add_argument("--providers", default="auto")
    council.add_argument("--available", help="test-only availability override")
    council.add_argument("--repo", action="append", default=[])
    council.add_argument("--context-public", action="store_true")

    route = commands.add_parser("route", help="choose an eligible role route")
    route.add_argument("--role", required=True)
    route.add_argument("--providers", default="auto")
    route.add_argument("--available", help="test-only availability override")
    route.add_argument("--repo", action="append", default=[])
    route.add_argument("--context-public", action="store_true")
    route.add_argument("--exclude-provider", action="append", default=[])

    write = commands.add_parser("write-handoff", help="write one append-only Markdown handoff")
    write.add_argument("--run-id", required=True)
    write.add_argument("--invocation-id", required=True)
    write.add_argument("--phase", required=True)
    write.add_argument("--role", required=True)
    write.add_argument(
        "--provider", required=True,
        choices=["codex", "claude", "deepseek", "qwen", "deterministic"]
    )
    write.add_argument("--model-requested", required=True)
    write.add_argument("--model-actual")
    write.add_argument("--effort", required=True)
    write.add_argument("--effort-actual")
    write.add_argument("--runner", required=True)
    write.add_argument("--billing-mode", default="unknown", choices=["subscription", "api", "none", "unknown"])
    write.add_argument("--session-id")
    write.add_argument("--prompt-schema", required=True)
    write.add_argument("--repo", action="append", default=[])
    write.add_argument("--repo-head", action="append", default=[])
    write.add_argument("--parent", action="append", default=[])
    write.add_argument("--body-file", required=True)
    write.add_argument("--usage-file")
    write.add_argument("--cost-file")
    write.add_argument("--status", default="completed", choices=["completed", "failed", "blocked", "superseded"])
    write.add_argument("--started-at")
    write.add_argument("--completed-at")
    write.add_argument("--output", required=True)

    verify = commands.add_parser("verify-handoff", help="verify hashes and required handoff metadata")
    verify.add_argument("path", nargs="+")

    state = commands.add_parser("phase-state", help="read authoritative Forge phase state")
    state.add_argument("--spec-dir", required=True)

    advance = commands.add_parser(
        "phase-advance", help="evaluate one phase transition against the deterministic table"
    )
    advance.add_argument("--spec-dir", required=True)
    advance.add_argument("--feature-id")
    advance.add_argument("--phase", required=True)
    advance.add_argument("--result", required=True)
    advance.add_argument("--repo", action="append", default=[])
    advance.add_argument("--skip-verify", action="store_true")
    advance.add_argument("--fixes-applied", action="store_true")
    advance.add_argument("--security-residual", action="store_true")
    advance.add_argument("--high-risk-uncovered", action="store_true")
    advance.add_argument("--max-residual-severity", choices=list(SEVERITY_ORDER))

    clear = commands.add_parser(
        "phase-clear",
        help="propose retiring a hard blocker (does NOT retire it; agents may call this)",
    )
    clear.add_argument("--spec-dir", required=True)
    clear.add_argument("--rule", required=True, help="rule id of the open blocker")
    clear.add_argument("--operator", required=True, help="who should decide")
    clear.add_argument("--reason", required=True, help="why it is safe to retire")
    clear.add_argument("--evidence", help="link or path to supporting evidence")
    clear.add_argument("--proposed-by", default="agent", help="who authored the proposal")

    sign = commands.add_parser(
        "phase-sign",
        help="batch-sign every unsigned feature commit at run close (one key touch per repo)",
    )
    sign.add_argument("--spec-dir", required=True)
    sign.add_argument("--feature-id")
    sign.add_argument("--repo", action="append", default=[])
    sign.add_argument(
        "--base",
        help="explicit base ref override; defaults to the per-repo resolution "
        "ship-gate itself uses (repo_base_ref: declared default_branch, then "
        "develop/main/master)",
    )
    sign.add_argument("--dry-run", action="store_true", help="report what would be signed")

    attest = commands.add_parser(
        "phase-attest",
        help="operator-only: confirm proposed clearances at the end of a run",
    )
    attest.add_argument("--spec-dir", required=True)
    attest.add_argument("--rule", help="one rule id; omit to attest every proposal")
    attest.add_argument("--operator", required=True, help="who is accepting the risk")
    attest.add_argument("--reason", help="used only when no proposal recorded one")
    attest.add_argument("--evidence", help="link or path to supporting evidence")
    attest.add_argument(
        "--assume-operator",
        action="store_true",
        help="non-interactive attestation; only for systems that already authenticate the operator",
    )

    governed = commands.add_parser(
        "is-governed", help="whether a shell command performs a phase-machine-governed action"
    )
    governed.add_argument(
        "--command",
        dest="command_text",
        required=True,
        help="the command string; use - to read stdin",
    )

    reanchor = commands.add_parser(
        "phase-reanchor", help="rebuild a feature's anchor chain from its journal"
    )
    reanchor.add_argument("--spec-dir", required=True)
    reanchor.add_argument("--feature-id")
    reanchor.add_argument("--repo", action="append", default=[])
    reanchor.add_argument(
        "--assume-operator",
        action="store_true",
        help="non-interactive repair; only for systems that already authenticate the operator",
    )
    reanchor.add_argument(
        "--operator",
        help="name of the operator accountable for this repair (required)",
    )

    gate = commands.add_parser(
        "ship-gate", help="fail-closed push/PR precondition check used by runtime hooks"
    )
    gate.add_argument("--spec-dir", required=True)
    gate.add_argument("--feature-id")

    blind = commands.add_parser("blind-handoff", help="render model-blind handoff bodies")
    blind.add_argument("path", nargs="+")

    economics = commands.add_parser(
        "summarize-handoffs", help="aggregate provider token/cache/cost economics"
    )
    economics.add_argument("path", nargs="+")

    run = commands.add_parser("invoke", help="invoke one provider through its isolated adapter")
    run.add_argument("--provider", required=True, choices=["codex", "claude", "deepseek", "qwen"])
    run.add_argument("--model", required=True)
    run.add_argument("--effort", required=True, choices=["low", "medium", "high", "xhigh", "max"])
    run.add_argument(
        "--billing-mode",
        default="unknown",
        choices=["subscription", "api", "unknown"],
    )
    run.add_argument("--role", required=True)
    run.add_argument("--cwd", required=True)
    run.add_argument("--prompt-file", required=True)
    run.add_argument("--schema-file")
    run.add_argument("--result-file", required=True)
    run.add_argument("--metadata-file")
    run.add_argument("--feature")
    run.add_argument("--invocation")
    run.add_argument("--repo", action="append", default=[])
    run.add_argument("--context-public", action="store_true")
    run.add_argument("--write", action="store_true")
    run.add_argument("--resume-session")
    run.add_argument(
        "--image", action="append", default=[],
        help="public visual evidence staged under specs/<feature>/forge/public-evidence/"
    )
    run.add_argument("--timeout-seconds", type=positive_int, default=3600)
    run.add_argument("--dry-run", action="store_true")
    return root_parser


def main(argv: list[str] | None = None) -> int:
    ns = parser().parse_args(argv)
    try:
        root = find_workspace(Path(ns.workspace) if ns.workspace else None)
        policy = load_policy(root)
        repositories = load_repositories(root)
        if ns.command == "resolve-profile":
            availability = detect_availability(policy, ns.available)
            result = resolve_profile(policy, ns.providers, availability)
        elif ns.command == "check-eligibility":
            result = confined_provider_eligibility(
                policy, repositories, ns.provider, ns.role,
                repo_names(ns.repo), ns.context_public
            )
        elif ns.command == "council-route":
            availability = detect_availability(policy, ns.available)
            resolved = resolve_profile(policy, ns.providers, availability)
            result = council_route(
                policy, resolved, repositories, ns.feature_id, repo_names(ns.repo), ns.context_public
            )
        elif ns.command == "route":
            availability = detect_availability(policy, ns.available)
            resolved = resolve_profile(policy, ns.providers, availability)
            result = choose_route(
                policy,
                resolved,
                repositories,
                ns.role,
                repo_names(ns.repo),
                ns.context_public,
                {normalize_provider(x) for x in ns.exclude_provider},
            )
        elif ns.command == "write-handoff":
            result = {"path": relative_display(root, build_handoff(root, repositories, ns))}
        elif ns.command == "phase-state":
            result = phase_state(root, resolve_spec_dir(root, ns.spec_dir, None))
        elif ns.command == "phase-advance":
            result = phase_advance(root, ns)
        elif ns.command == "phase-clear":
            result = phase_clear(root, ns)
        elif ns.command == "phase-sign":
            result = phase_sign(root, ns)
        elif ns.command == "phase-attest":
            result = phase_attest(root, ns)
        elif ns.command == "is-governed":
            raw = sys.stdin.read() if ns.command_text == "-" else ns.command_text
            result = is_governed(raw)
        elif ns.command == "phase-reanchor":
            result = phase_reanchor(root, ns)
        elif ns.command == "ship-gate":
            result = ship_gate(root, resolve_spec_dir(root, ns.spec_dir, ns.feature_id), ns.feature_id)
        elif ns.command == "verify-handoff":
            reports = [verify_handoff(root, Path(path).resolve()) for path in ns.path]
            result = {"valid": all(report["valid"] for report in reports), "handoffs": reports}
        elif ns.command == "blind-handoff":
            result = {
                "handoffs": [blind_handoff(root, Path(path).resolve()) for path in ns.path]
            }
        elif ns.command == "summarize-handoffs":
            result = summarize_handoffs(root, [Path(path) for path in ns.path])
        elif ns.command == "invoke":
            result = invoke(root, policy, repositories, ns)
        else:
            raise ForgeControlError(f"unsupported command: {ns.command}")
        print(json.dumps(result, indent=2, sort_keys=True))
        return 0 if result.get("valid", True) and result.get("eligible", True) else 1
    except ForgeControlError as exc:
        print(f"forge-control: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
