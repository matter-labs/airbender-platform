#!/usr/bin/env bash

set -euo pipefail

output_dir="${1:?usage: build-api-docs.sh <output-dir>}"

workspace_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "${workspace_root}"

export ZKSYNC_USE_CUDA_STUBS="${ZKSYNC_USE_CUDA_STUBS:-true}"

rm -rf target/doc "${output_dir}"

cargo doc --workspace --no-deps \
  --exclude airbender-host \
  --exclude cargo-airbender

cargo doc -p airbender-host --no-deps \
  --no-default-features \
  --features docs-only

cargo doc -p cargo-airbender --no-deps \
  --no-default-features \
  --features docs-only

mkdir -p "${output_dir}"
cp -a target/doc/. "${output_dir}/"
rm -f "${output_dir}/.lock"
touch "${output_dir}/.nojekyll"

mapfile -t published_crates < <(
  cargo metadata --no-deps --format-version 1 \
    | jq -r '
        .packages[]
        | . as $pkg
        | .targets[]
        | select(.kind[0] == "lib" or .kind[0] == "proc-macro" or .kind[0] == "bin")
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
        Generated from the workspace crates with <code>cargo doc --workspace --no-deps</code>.
      </p>
      <section class="panel">
        <h2>Published Crates</h2>
        <ul>
EOF

  for crate in "${published_crates[@]}"; do
    IFS=$'\t' read -r package_name target_name target_kind <<<"${crate}"
    target_doc_name="${target_name//-/_}"
    package_doc_name="${package_name//-/_}"
    if [[ ! -f "${output_dir}/${target_doc_name}/index.html" ]]; then
      continue
    fi
    suffix=""
    if [[ "${target_doc_name}" != "${package_doc_name}" ]]; then
      suffix=" (crate: <code>${target_name}</code>)"
    fi
    if [[ "${target_kind}" == "proc-macro" ]]; then
      suffix="${suffix} <span>proc-macro</span>"
    elif [[ "${target_kind}" == "bin" ]]; then
      suffix="${suffix} <span>binary</span>"
    fi
    printf '          <li><a href="./%s/">%s</a>%s</li>\n' \
      "${target_doc_name}" \
      "${package_name}" \
      "${suffix}"
  done

  cat <<'EOF'
        </ul>
        <p>
          Builds run with <code>ZKSYNC_USE_CUDA_STUBS=true</code> so the published API
          reference stays available on standard GitHub Actions runners.
        </p>
      </section>
    </main>
  </body>
</html>
EOF
} > "${output_dir}/index.html"
