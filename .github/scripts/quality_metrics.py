#!/usr/bin/env python3

from __future__ import annotations

import argparse
import csv
import pathlib
import re
import sys
import tomllib
from collections import defaultdict


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Summarize lizard quality metrics.")
    parser.add_argument("--functions", required=True, help="CSV output from lizard --csv.")
    parser.add_argument("--duplicates", required=True, help="Text output from lizard -Eduplicate.")
    parser.add_argument("--summary", required=True, help="Markdown summary path.")
    parser.add_argument(
        "--max-duplicate-rate",
        type=float,
        required=True,
        help="Maximum allowed duplicate rate percentage.",
    )
    parser.add_argument(
        "--max-ccn",
        type=int,
        required=True,
        help="Maximum allowed cyclomatic complexity per function.",
    )
    parser.add_argument(
        "--max-length",
        type=int,
        required=True,
        help="Maximum allowed function length in lines.",
    )
    parser.add_argument(
        "--workspace-root",
        required=True,
        help="Repository root used for coupling and documentation metrics.",
    )
    parser.add_argument(
        "--min-public-doc-coverage",
        type=float,
        default=None,
        help="Optional minimum percentage of public items that must have Rustdoc comments.",
    )
    return parser.parse_args()


def normalize_path(raw: str) -> str:
    return pathlib.PurePosixPath(raw.replace("\\", "/")).as_posix()


def load_functions(csv_path: pathlib.Path) -> list[dict[str, object]]:
    rows: list[dict[str, object]] = []
    with csv_path.open(newline="", encoding="utf-8-sig") as handle:
        reader = csv.reader(handle)
        for row in reader:
            if len(row) != 11:
                raise ValueError(f"Unexpected CSV row with {len(row)} columns: {row!r}")
            rows.append(
                {
                    "nloc": int(row[0]),
                    "ccn": int(row[1]),
                    "tokens": int(row[2]),
                    "params": int(row[3]),
                    "length": int(row[4]),
                    "location": row[5],
                    "file": normalize_path(row[6]),
                    "name": row[7],
                    "long_name": row[8],
                    "start_line": int(row[9]),
                    "end_line": int(row[10]),
                }
            )
    return rows


def parse_duplicate_report(report_path: pathlib.Path) -> tuple[float, int]:
    text = report_path.read_text(encoding="utf-8-sig")
    match = re.search(r"Total duplicate rate:\s*([0-9.]+)%", text)
    if not match:
        raise ValueError("Could not find total duplicate rate in duplicate report.")
    duplicate_rate = float(match.group(1))
    duplicate_blocks = text.count("Duplicate block:")
    return duplicate_rate, duplicate_blocks


def display_module_name(crate_name: str, module_path: str) -> str:
    if module_path == "crate":
        return f"{crate_name}::crate"
    return f"{crate_name}::{module_path.removeprefix('crate::')}"


def format_rows(rows: list[dict[str, object]], kind: str) -> list[str]:
    lines = [
        "| Function | File | Value | Secondary |",
        "| --- | --- | ---: | ---: |",
    ]
    for row in rows:
        value = row[kind]
        secondary = row["ccn"] if kind == "length" else row["length"]
        lines.append(
            f"| `{row['name']}` | `{row['file']}:{row['start_line']}` | "
            f"`{value}` | `{secondary}` |"
        )
    return lines


def format_file_rows(rows: list[tuple[str, dict[str, float]]]) -> list[str]:
    lines = [
        "| File | Summed NLOC | Functions | Max CCN | Max Length |",
        "| --- | ---: | ---: | ---: | ---: |",
    ]
    for file_path, stats in rows:
        lines.append(
            f"| `{file_path}` | `{int(stats['nloc'])}` | `{int(stats['functions'])}` | "
            f"`{int(stats['max_ccn'])}` | `{int(stats['max_length'])}` |"
        )
    return lines


def format_crate_rows(rows: list[tuple[str, int, int]]) -> list[str]:
    lines = [
        "| Crate | Fan-out | Fan-in |",
        "| --- | ---: | ---: |",
    ]
    for crate_name, fan_out, fan_in in rows:
        lines.append(f"| `{crate_name}` | `{fan_out}` | `{fan_in}` |")
    return lines


def format_module_rows(rows: list[tuple[str, int, int]]) -> list[str]:
    lines = [
        "| Module | Fan-out | Fan-in |",
        "| --- | ---: | ---: |",
    ]
    for module_name, fan_out, fan_in in rows:
        lines.append(f"| `{module_name}` | `{fan_out}` | `{fan_in}` |")
    return lines


def format_doc_rows(rows: list[tuple[str, dict[str, float]]]) -> list[str]:
    lines = [
        "| Crate | Rustdoc Lines | Code Lines | Rustdoc Line % | Public Items Documented | Public Items | Public API Doc % |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for crate_name, stats in rows:
        lines.append(
            f"| `{crate_name}` | `{int(stats['doc_lines'])}` | `{int(stats['code_lines'])}` | "
            f"`{stats['doc_line_percent']:.2f}%` | `{int(stats['documented_public_items'])}` | "
            f"`{int(stats['public_items'])}` | `{stats['public_item_percent']:.2f}%` |"
        )
    return lines


def workspace_crate_manifests(workspace_root: pathlib.Path) -> dict[str, pathlib.Path]:
    manifests: dict[str, pathlib.Path] = {}
    for manifest_path in sorted(workspace_root.glob("crates/*/Cargo.toml")):
        with manifest_path.open("rb") as handle:
            data = tomllib.load(handle)
        crate_name = data["package"]["name"]
        manifests[crate_name] = manifest_path
    return manifests


def analyze_crate_coupling(crate_manifests: dict[str, pathlib.Path]) -> tuple[dict[str, set[str]], dict[str, int]]:
    crate_names = set(crate_manifests)
    fan_out: dict[str, set[str]] = {}
    fan_in: dict[str, int] = {crate_name: 0 for crate_name in crate_names}
    for crate_name, manifest_path in crate_manifests.items():
        with manifest_path.open("rb") as handle:
            data = tomllib.load(handle)
        dependencies = data.get("dependencies", {})
        internal_deps = {dep_name for dep_name in dependencies if dep_name in crate_names}
        fan_out[crate_name] = internal_deps
        for dep_name in internal_deps:
            fan_in[dep_name] += 1
    return fan_out, fan_in


def module_path_for_file(src_dir: pathlib.Path, file_path: pathlib.Path) -> str:
    rel = file_path.relative_to(src_dir)
    parts = list(rel.parts)
    if parts[-1] == "lib.rs":
        return "crate"
    if parts[-1] == "mod.rs":
        return "crate::" + "::".join(parts[:-1])
    parts[-1] = parts[-1][:-3]
    return "crate::" + "::".join(parts)


def strip_non_doc_comments(text: str) -> str:
    block_pattern = re.compile(r"/\*(?![!*]).*?\*/", re.S)
    line_pattern = re.compile(r"(?m)//(?![/!]).*$")
    text = block_pattern.sub("", text)
    return line_pattern.sub("", text)


def resolve_module_reference(reference: str, current_module: str, module_paths: set[str]) -> str | None:
    current_segments = [] if current_module == "crate" else current_module.split("::")[1:]
    if reference.startswith("crate::"):
        target_segments = reference.split("::")[1:]
    else:
        base_segments = current_segments.copy()
        remaining = reference
        while remaining.startswith("super::"):
            remaining = remaining[len("super::") :]
            if base_segments:
                base_segments.pop()
        if remaining.startswith("self::"):
            remaining = remaining[len("self::") :]
        target_segments = base_segments + remaining.split("::")
    candidate = ["crate", *[segment for segment in target_segments if segment]]
    for prefix_len in range(len(candidate), 0, -1):
        prefix = "::".join(candidate[:prefix_len])
        if prefix in module_paths:
            return prefix
    return None


def count_documented_public_items(lines: list[str]) -> tuple[int, int]:
    public_item_pattern = re.compile(
        r"pub(?:\([^)]*\))?\s+(?:struct|enum|trait|fn|type|mod|const|static)\b"
    )
    public_items = 0
    documented_public_items = 0
    doc_buffer = 0
    in_block_doc = False
    for raw_line in lines:
        stripped = raw_line.strip()
        if in_block_doc:
            doc_buffer += 1
            if "*/" in stripped:
                in_block_doc = False
            continue
        if stripped.startswith("///") or stripped.startswith("//!"):
            doc_buffer += 1
            continue
        if stripped.startswith("/**") or stripped.startswith("/*!"):
            doc_buffer += 1
            if "*/" not in stripped:
                in_block_doc = True
            continue
        if not stripped:
            continue
        if stripped.startswith("#["):
            continue
        if public_item_pattern.match(stripped):
            public_items += 1
            if doc_buffer > 0:
                documented_public_items += 1
        doc_buffer = 0
    return documented_public_items, public_items


def analyze_source_metrics(
    workspace_root: pathlib.Path, crate_manifests: dict[str, pathlib.Path]
) -> tuple[dict[str, dict[str, float]], list[tuple[str, int, int]], list[tuple[str, int, int]], int, int]:
    path_pattern = re.compile(r"\b(?:crate|self|super)(?:::[A-Za-z_][A-Za-z0-9_]*)+")
    crate_docs: dict[str, dict[str, float]] = {}
    module_outgoing_rows: list[tuple[str, int, int]] = []
    module_incoming_rows: list[tuple[str, int, int]] = []
    total_modules = 0
    total_module_edges = 0

    for crate_name, manifest_path in sorted(crate_manifests.items()):
        crate_dir = manifest_path.parent
        src_dir = crate_dir / "src"
        source_files = sorted(src_dir.rglob("*.rs"))
        module_map = {
            module_path_for_file(src_dir, file_path): file_path for file_path in source_files
        }
        module_paths = set(module_map)
        graph: dict[str, set[str]] = defaultdict(set)
        doc_lines = 0
        code_lines = 0
        documented_public_items = 0
        public_items = 0

        for module_path, file_path in module_map.items():
            text = file_path.read_text(encoding="utf-8")
            lines = text.splitlines()
            stripped = strip_non_doc_comments(text)

            for line in lines:
                line_stripped = line.strip()
                if line_stripped.startswith(("///", "//!")):
                    doc_lines += 1
                    continue
                if line_stripped.startswith(("/**", "/*!")):
                    doc_lines += 1
                    continue
                if line_stripped and not line_stripped.startswith("//"):
                    code_lines += 1

            file_documented, file_public = count_documented_public_items(lines)
            documented_public_items += file_documented
            public_items += file_public

            for reference in path_pattern.findall(stripped):
                target = resolve_module_reference(reference, module_path, module_paths)
                if target and target != module_path:
                    graph[module_path].add(target)

        incoming_counts: dict[str, int] = defaultdict(int)
        for source_module, target_modules in graph.items():
            for target_module in target_modules:
                incoming_counts[target_module] += 1

        for module_path in module_map:
            display_name = display_module_name(crate_name, module_path)
            outgoing = len(graph.get(module_path, set()))
            incoming = incoming_counts.get(module_path, 0)
            module_outgoing_rows.append((display_name, outgoing, incoming))
            module_incoming_rows.append((display_name, outgoing, incoming))

        total_modules += len(module_map)
        total_module_edges += sum(len(targets) for targets in graph.values())

        doc_line_percent = (doc_lines * 100.0 / code_lines) if code_lines else 0.0
        public_item_percent = (
            documented_public_items * 100.0 / public_items if public_items else 100.0
        )
        crate_docs[crate_name] = {
            "doc_lines": float(doc_lines),
            "code_lines": float(code_lines),
            "doc_line_percent": doc_line_percent,
            "documented_public_items": float(documented_public_items),
            "public_items": float(public_items),
            "public_item_percent": public_item_percent,
        }

    module_outgoing_rows.sort(key=lambda row: (row[1], row[2], row[0]), reverse=True)
    module_incoming_rows.sort(key=lambda row: (row[2], row[1], row[0]), reverse=True)
    return crate_docs, module_outgoing_rows, module_incoming_rows, total_modules, total_module_edges


def main() -> int:
    args = parse_args()
    functions_path = pathlib.Path(args.functions)
    duplicates_path = pathlib.Path(args.duplicates)
    summary_path = pathlib.Path(args.summary)
    workspace_root = pathlib.Path(args.workspace_root).resolve()

    functions = load_functions(functions_path)
    duplicate_rate, duplicate_blocks = parse_duplicate_report(duplicates_path)
    crate_manifests = workspace_crate_manifests(workspace_root)
    crate_fan_out, crate_fan_in = analyze_crate_coupling(crate_manifests)
    crate_docs, module_out_rows, module_in_rows, total_modules, total_module_edges = analyze_source_metrics(
        workspace_root, crate_manifests
    )

    total_functions = len(functions)
    unique_files = {row["file"] for row in functions}
    total_nloc = sum(int(row["nloc"]) for row in functions)
    avg_nloc = total_nloc / total_functions if total_functions else 0.0
    avg_ccn = sum(int(row["ccn"]) for row in functions) / total_functions if total_functions else 0.0

    longest_functions = sorted(
        functions,
        key=lambda row: (int(row["length"]), int(row["ccn"]), int(row["nloc"])),
        reverse=True,
    )[:5]
    most_complex_functions = sorted(
        functions,
        key=lambda row: (int(row["ccn"]), int(row["length"]), int(row["nloc"])),
        reverse=True,
    )[:5]

    by_file: dict[str, dict[str, float]] = defaultdict(
        lambda: {"nloc": 0.0, "functions": 0.0, "max_ccn": 0.0, "max_length": 0.0}
    )
    for row in functions:
        stats = by_file[str(row["file"])]
        stats["nloc"] += int(row["nloc"])
        stats["functions"] += 1
        stats["max_ccn"] = max(stats["max_ccn"], int(row["ccn"]))
        stats["max_length"] = max(stats["max_length"], int(row["length"]))

    largest_files = sorted(
        by_file.items(),
        key=lambda item: (item[1]["nloc"], item[1]["max_length"], item[1]["max_ccn"]),
        reverse=True,
    )[:5]

    crate_rows = sorted(
        (
            crate_name,
            len(crate_fan_out.get(crate_name, set())),
            crate_fan_in.get(crate_name, 0),
        )
        for crate_name in sorted(crate_manifests)
    )
    directed_possible_edges = len(crate_rows) * (len(crate_rows) - 1)
    directed_edges = sum(fan_out for _, fan_out, _ in crate_rows)
    crate_coupling_density = (
        directed_edges * 100.0 / directed_possible_edges if directed_possible_edges else 0.0
    )
    max_crate_fan_out = max(crate_rows, key=lambda row: (row[1], row[2], row[0]))
    max_crate_fan_in = max(crate_rows, key=lambda row: (row[2], row[1], row[0]))

    overall_doc_lines = sum(stats["doc_lines"] for stats in crate_docs.values())
    overall_code_lines = sum(stats["code_lines"] for stats in crate_docs.values())
    overall_public_items = sum(stats["public_items"] for stats in crate_docs.values())
    overall_documented_public_items = sum(
        stats["documented_public_items"] for stats in crate_docs.values()
    )
    overall_doc_line_percent = (
        overall_doc_lines * 100.0 / overall_code_lines if overall_code_lines else 0.0
    )
    overall_public_doc_percent = (
        overall_documented_public_items * 100.0 / overall_public_items
        if overall_public_items
        else 100.0
    )
    doc_rows = sorted(
        crate_docs.items(),
        key=lambda item: (item[1]["public_item_percent"], item[1]["doc_line_percent"], item[0]),
    )
    doc_status = "PASS"
    if (
        args.min_public_doc_coverage is not None
        and overall_public_doc_percent < args.min_public_doc_coverage
    ):
        doc_status = "FAIL"

    duplicate_status = "PASS"
    if duplicate_rate > args.max_duplicate_rate:
        duplicate_status = "FAIL"

    worst_module_fan_out = module_out_rows[0] if module_out_rows else ("n/a", 0, 0)
    worst_module_fan_in = module_in_rows[0] if module_in_rows else ("n/a", 0, 0)

    lines = [
        "## Quality Metrics",
        "",
        "Thresholds:",
        f"- `cyclomatic complexity <= {args.max_ccn}` per function (with audited exceptions in `.github/qa/whitelizard.txt`)",
        f"- `function length <= {args.max_length}` lines (same exception policy)",
        f"- `duplicate rate <= {args.max_duplicate_rate:.2f}%` across production Rust code",
        (
            f"- `public API documentation coverage >= {args.min_public_doc_coverage:.2f}%`"
            if args.min_public_doc_coverage is not None
            else "- `public API documentation coverage` is reported only in this run"
        ),
        "",
        "Current snapshot:",
        f"- Functions analyzed: `{total_functions}` across `{len(unique_files)}` production Rust files",
        f"- Average function size: `{avg_nloc:.1f}` NLOC",
        f"- Average cyclomatic complexity: `{avg_ccn:.1f}`",
        f"- Duplicate blocks detected: `{duplicate_blocks}`",
        f"- Duplicate rate status: `{duplicate_status}` at `{duplicate_rate:.2f}%`",
        f"- Workspace crate coupling: `{directed_edges}` internal edges across `{len(crate_rows)}` crates (`{crate_coupling_density:.2f}%` directed density)",
        f"- Max crate fan-out: `{max_crate_fan_out[1]}` in `{max_crate_fan_out[0]}`",
        f"- Max crate fan-in: `{max_crate_fan_in[2]}` in `{max_crate_fan_in[0]}`",
        f"- Source-module graph: `{total_module_edges}` internal edges across `{total_modules}` production modules",
        f"- Max source-module fan-out: `{worst_module_fan_out[1]}` in `{worst_module_fan_out[0]}`",
        f"- Max source-module fan-in: `{worst_module_fan_in[2]}` in `{worst_module_fan_in[0]}`",
        f"- Rustdoc line coverage: `{overall_doc_line_percent:.2f}%` (`{int(overall_doc_lines)}` doc lines / `{int(overall_code_lines)}` code lines)",
        f"- Public API documentation status: `{doc_status}` at `{overall_public_doc_percent:.2f}%` (`{int(overall_documented_public_items)}` documented / `{int(overall_public_items)}` public items)",
        "",
        "Top function hotspots by length:",
        *format_rows(longest_functions, "length"),
        "",
        "Top function hotspots by cyclomatic complexity:",
        *format_rows(most_complex_functions, "ccn"),
        "",
        "Top files by summed function NLOC:",
        *format_file_rows(largest_files),
        "",
        "Workspace crate fan-in / fan-out:",
        *format_crate_rows(crate_rows),
        "",
        "Top production modules by fan-out:",
        *format_module_rows(module_out_rows[:5]),
        "",
        "Top production modules by fan-in:",
        *format_module_rows(module_in_rows[:5]),
        "",
        "Documentation coverage by crate:",
        *format_doc_rows(doc_rows),
        "",
        "Metric notes:",
        "- Crate fan-in / fan-out comes from direct internal workspace dependencies declared in `Cargo.toml`.",
        "- Source-module fan-in / fan-out is derived from resolved `crate::`, `self::`, and `super::` references in production Rust files.",
        "- Documentation metrics use Rustdoc comments (`///`, `//!`, `/**`, `/*!`) on production code only; public API coverage is a line-oriented heuristic rather than a full Rust parser.",
        "",
    ]

    summary_path.write_text("\n".join(lines), encoding="utf-8")
    return 1 if duplicate_status == "FAIL" or doc_status == "FAIL" else 0


if __name__ == "__main__":
    sys.exit(main())
