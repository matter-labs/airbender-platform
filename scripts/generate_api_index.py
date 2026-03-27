#!/usr/bin/env python3

"""Generate a stable landing page for workspace rustdocs.

`cargo doc --workspace` produces one page per crate but no shared `/api/index.html`.
We generate a tiny index from `cargo metadata` so the published API docs stay
discoverable without hard-coding a crate list in CI.
"""

import argparse
import html
import json
import subprocess
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class WorkspaceDoc:
    package_name: str
    target_name: str
    target_kind: str

    @property
    def rustdoc_path(self) -> str:
        return f"{self.target_name.replace('-', '_')}/index.html"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workspace-root", default=".")
    parser.add_argument("--template", required=True)
    parser.add_argument("--output", required=True)
    return parser.parse_args()


def load_workspace_docs(workspace_root: Path) -> list[WorkspaceDoc]:
    metadata = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        check=True,
        capture_output=True,
        text=True,
        cwd=workspace_root,
    )
    payload = json.loads(metadata.stdout)

    package_by_id = {package["id"]: package for package in payload["packages"]}
    docs: list[WorkspaceDoc] = []
    for member_id in payload["workspace_members"]:
        package = package_by_id[member_id]
        target = pick_primary_target(package["targets"])
        if target is None:
            continue
        docs.append(
            WorkspaceDoc(
                package_name=package["name"],
                target_name=target["name"],
                target_kind=target["kind"][0],
            )
        )
    return docs


def pick_primary_target(targets: list[dict[str, object]]) -> dict[str, object] | None:
    preferred_kinds = ("lib", "proc-macro", "bin")
    for preferred_kind in preferred_kinds:
        for target in targets:
            kinds = target["kind"]
            if preferred_kind in kinds:
                return target
    return None


def render_cards(docs: list[WorkspaceDoc]) -> str:
    cards: list[str] = []
    for doc in docs:
        crate_name = doc.target_name.replace("-", "_")
        kind_label = target_kind_label(doc.target_kind)
        cards.append(
            "\n".join(
                [
                    '        <li class="crate-card">',
                    f'          <a class="crate-link" href="{html.escape(doc.rustdoc_path, quote=True)}">',
                    f'            <p class="crate-name"><code>{html.escape(doc.package_name)}</code></p>',
                    "          </a>",
                    f'          <p class="crate-meta">{kind_label} crate <code>{html.escape(crate_name)}</code></p>',
                    "        </li>",
                ]
            )
        )
    return "\n".join(cards)


def target_kind_label(target_kind: str) -> str:
    if target_kind == "proc-macro":
        return "proc-macro"
    if target_kind == "bin":
        return "binary"
    return "library"


def main() -> None:
    args = parse_args()
    workspace_root = Path(args.workspace_root).resolve()
    template_path = Path(args.template).resolve()
    output_path = Path(args.output).resolve()

    docs = load_workspace_docs(workspace_root)
    template = template_path.read_text(encoding="utf-8")
    rendered = template.replace("{{ crate_cards }}", render_cards(docs))

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(rendered, encoding="utf-8")


if __name__ == "__main__":
    main()
