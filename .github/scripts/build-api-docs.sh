#!/usr/bin/env bash

set -euo pipefail

output_dir="${1:?usage: build-api-docs.sh <output-dir>}"

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${workspace_root}"

rm -rf target/doc "${output_dir}"

cargo doc --workspace --no-deps \
  --exclude airbender-host \
  --exclude cargo-airbender

mkdir -p "${output_dir}"
cp -a target/doc/. "${output_dir}/"
rm -f "${output_dir}/.lock"
touch "${output_dir}/.nojekyll"

mapfile -t published_crates < <(
  cargo metadata --no-deps --format-version 1 \
    | jq -r '
        .packages[]
        | select(.name != "airbender-host" and .name != "cargo-airbender")
        | . as $pkg
        | .targets[]
        | select(.kind[0] == "lib" or .kind[0] == "proc-macro")
        | [$pkg.name, .name, .kind[0]]
        | @tsv
      ' \
    | sort
)

{
  cat <<'EOF'
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <title>Airbender Platform Rust API Docs</title>
    <style>
      :root {
        color-scheme: light dark;
        font-family: "Iosevka Web", "Fira Code", monospace;
      }
      body {
        margin: 0;
        background:
          radial-gradient(circle at top left, rgba(86, 156, 214, 0.18), transparent 30%),
          radial-gradient(circle at bottom right, rgba(46, 204, 113, 0.14), transparent 35%),
          #0d1117;
        color: #e6edf3;
      }
      main {
        max-width: 960px;
        margin: 0 auto;
        padding: 48px 24px 64px;
      }
      h1, h2, p, li {
        line-height: 1.5;
      }
      h1 {
        margin: 0 0 12px;
        font-size: clamp(2rem, 4vw, 3rem);
      }
      p {
        margin: 0 0 20px;
        max-width: 72ch;
      }
      .panel {
        margin-top: 28px;
        padding: 24px;
        border: 1px solid rgba(230, 237, 243, 0.16);
        border-radius: 20px;
        background: rgba(13, 17, 23, 0.78);
        backdrop-filter: blur(12px);
      }
      ul {
        margin: 0;
        padding-left: 20px;
      }
      li + li {
        margin-top: 12px;
      }
      a {
        color: #7ee787;
      }
      code {
        font-size: 0.95em;
      }
    </style>
  </head>
  <body>
    <main>
      <h1>Airbender Platform Rust API Docs</h1>
      <p>
        Generated from the workspace crates that currently build successfully under
        <code>cargo doc --workspace --no-deps</code> in CI.
      </p>
      <section class="panel">
        <h2>Published Crates</h2>
        <ul>
EOF

  for crate in "${published_crates[@]}"; do
    IFS=$'\t' read -r package_name target_name target_kind <<<"${crate}"
    package_doc_name="${package_name//-/_}"
    suffix=""
    if [[ "${target_name}" != "${package_doc_name}" ]]; then
      suffix=" (crate: <code>${target_name}</code>)"
    fi
    if [[ "${target_kind}" == "proc-macro" ]]; then
      suffix="${suffix} <span>proc-macro</span>"
    fi
    printf '          <li><a href="./%s/">%s</a>%s</li>\n' \
      "${target_name}" \
      "${package_name}" \
      "${suffix}"
  done

  cat <<'EOF'
        </ul>
      </section>
      <section class="panel">
        <h2>Temporarily Excluded</h2>
        <p>
          <code>airbender-host</code> and <code>cargo-airbender</code> are not part of the
          published API site yet. Their rustdoc builds currently fail in CI because the pinned
          nightly toolchain hits a rustdoc query cycle in <code>riscv_transpiler</code>, and the
          default host feature set also requires CUDA through the GPU prover path.
        </p>
      </section>
    </main>
  </body>
</html>
EOF
} > "${output_dir}/index.html"
