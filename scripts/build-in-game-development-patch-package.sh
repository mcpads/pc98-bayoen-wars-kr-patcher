#!/bin/sh
set -eu

usage() {
    printf '%s\n' \
        "usage: $0 <disc-station-vol05-disk1.hdm> <source-title.png> <retro-patcher-dir> <output.zip>" >&2
}

if [ "$#" -ne 4 ]; then
    usage
    exit 2
fi

source_hdm_path=$1
source_title_path=$2
patcher_tool_dir=$3
package_output_path=$4

resolve_existing_path() (
    requested_path=$1
    if [ ! -e "$requested_path" ]; then
        printf 'required input is missing: %s\n' "$requested_path" >&2
        exit 1
    fi
    requested_dir=$(dirname -- "$requested_path")
    requested_name=$(basename -- "$requested_path")
    resolved_dir=$(CDPATH= cd -- "$requested_dir" && pwd)
    printf '%s/%s\n' "$resolved_dir" "$requested_name"
)

source_hdm_path=$(resolve_existing_path "$source_hdm_path")
source_title_path=$(resolve_existing_path "$source_title_path")
patcher_tool_dir=$(resolve_existing_path "$patcher_tool_dir")

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_dir=$(dirname -- "$script_dir")

if [ ! -f "$patcher_tool_dir/patch-core/Cargo.toml" ]; then
    printf 'Retro Patcher tool is missing patch-core: %s\n' "$patcher_tool_dir" >&2
    exit 1
fi

if [ -e "$package_output_path" ]; then
    printf 'refusing to overwrite patch package: %s\n' "$package_output_path" >&2
    exit 1
fi

package_output_dir=$(dirname -- "$package_output_path")
package_output_name=$(basename -- "$package_output_path")
mkdir -p "$package_output_dir"
package_output_dir=$(CDPATH= cd -- "$package_output_dir" && pwd)
package_output_path="$package_output_dir/$package_output_name"

package_workspace=$(mktemp -d "$package_output_dir/.bayoen-fat12-package.XXXXXX")
remove_package_workspace() {
    case "$package_workspace" in
        "$package_output_dir"/.bayoen-fat12-package.*) rm -r -- "$package_workspace" ;;
        *) printf 'refusing to remove unexpected package workspace: %s\n' "$package_workspace" >&2 ;;
    esac
}
trap remove_package_workspace EXIT HUP INT TERM

content_hdm_path="$package_workspace/content.hdm"
plan_path="$package_workspace/plan.json"
candidate_package_path="$package_workspace/candidate.zip"
repeated_package_path="$package_workspace/repeated.zip"
reapplied_hdm_path="$package_workspace/reapplied.hdm"

cd "$project_dir"
cargo run --locked --release -- build-mad-scene-narrative-development \
    --source "$source_hdm_path" \
    --title-source-preview "$source_title_path" \
    --output "$content_hdm_path"

cargo run --locked --release -- write-in-game-retro-patcher-plan \
    --source "$source_hdm_path" \
    --content "$content_hdm_path" \
    --output "$plan_path"

create_package() {
    package_path=$1
    cargo run --locked --release \
        --manifest-path "$patcher_tool_dir/patch-core/Cargo.toml" \
        --bin retro-patch-author -- create \
        "$plan_path" "$source_hdm_path" "$content_hdm_path" "$package_path"
}

create_package "$candidate_package_path"
create_package "$repeated_package_path"

if ! cmp -s "$candidate_package_path" "$repeated_package_path"; then
    printf '%s\n' 'repeated package creation produced different ZIP bytes' >&2
    exit 1
fi

cargo run --locked --release \
    --manifest-path "$patcher_tool_dir/patch-core/Cargo.toml" \
    --bin retro-patch-author -- inspect "$candidate_package_path"

cargo run --locked --release \
    --manifest-path "$patcher_tool_dir/patch-core/Cargo.toml" \
    --bin retro-patch-author -- apply \
    "$source_hdm_path" "$candidate_package_path" "$reapplied_hdm_path"

cargo run --locked --release -- verify-in-game-retro-patcher-result \
    --source "$source_hdm_path" \
    --content "$content_hdm_path" \
    --candidate "$reapplied_hdm_path"

ln "$candidate_package_path" "$package_output_path"
cmp -s "$candidate_package_path" "$package_output_path"
shasum -a 256 "$package_output_path" "$content_hdm_path" "$reapplied_hdm_path"
