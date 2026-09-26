#!/usr/bin/env bash
#
# scripts/plugins.sh - Manage Karakuri plugin extensions
#
# Manages cloning sibling repositories, building, and installing into plugins/.
#
# Supported plugins:
#   link   - Ableton Link tempo source (input)       [macOS, Linux, Windows]
#   syphon - Syphon output sink (output)             [macOS]
#   spout  - Spout2 output sink (output)             [Windows]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SIBLING_ROOT="$(cd "$WORKSPACE_ROOT/.." && pwd)"
PLUGINS_DIR="$WORKSPACE_ROOT/plugins"

# Detect host OS
detect_os() {
    case "$(uname -s)" in
        Darwin*)
            echo "macos"
            ;;
        Linux*)
            echo "linux"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "windows"
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

HOST_OS="$(detect_os)"

# Plugin metadata
plugin_repo() {
    case "$1" in
        link)   echo "Karakuri-link" ;;
        syphon) echo "Karakuri-syphon" ;;
        spout)  echo "Karakuri-spout" ;;
        *)      return 1 ;;
    esac
}

plugin_bin() {
    case "$1" in
        link)
            if [ "$HOST_OS" = "windows" ]; then
                echo "karakuri-link.exe"
            else
                echo "karakuri-link"
            fi
            ;;
        syphon)
            echo "karakuri-syphon"
            ;;
        spout)
            if [ "$HOST_OS" = "windows" ]; then
                echo "karakuri-spout.exe"
            else
                echo "karakuri-spout"
            fi
            ;;
        *)
            return 1
            ;;
    esac
}

plugin_type() {
    case "$1" in
        link)   echo "input (tempo-source)" ;;
        syphon) echo "output (iosurface)" ;;
        spout)  echo "output (dxgi)" ;;
        *)      return 1 ;;
    esac
}

plugin_supported_on() {
    local plugin="$1"
    local os="$2"
    case "$plugin" in
        link)   return 0 ;;
        syphon) [ "$os" = "macos" ] && return 0 || return 1 ;;
        spout)  [ "$os" = "windows" ] && return 0 || return 1 ;;
        *)      return 1 ;;
    esac
}

# Determine default remote URL base from current workspace origin
get_remote_url() {
    local repo_name="$1"
    local origin_url
    origin_url="$(git -C "$WORKSPACE_ROOT" remote get-url origin 2>/dev/null || echo "")"
    if [ -n "$origin_url" ]; then
        local base="${origin_url%/*}"
        echo "${base}/${repo_name}.git"
    else
        echo "https://github.com/toyoshim-i/${repo_name}.git"
    fi
}

cmd_clone() {
    local name="$1"
    local repo
    repo="$(plugin_repo "$name")"
    local target_dir="$SIBLING_ROOT/$repo"

    if [ -d "$target_dir/.git" ]; then
        echo "==> [$name] Sibling repository already exists at $target_dir"
        echo "    Updating $target_dir via git pull..."
        git -C "$target_dir" pull --ff-only || {
            echo "    Warning: git pull failed or local changes present; continuing."
        }
    else
        local url
        url="$(get_remote_url "$repo")"
        echo "==> [$name] Cloning $url into $target_dir..."
        git clone "$url" "$target_dir"
    fi
}

cmd_build() {
    local name="$1"
    local profile="${2:-release}"
    local repo
    repo="$(plugin_repo "$name")"
    local target_dir="$SIBLING_ROOT/$repo"

    if [ ! -d "$target_dir" ]; then
        echo "Error: [$name] Directory not found at $target_dir. Run clone first." >&2
        return 1
    fi

    if ! plugin_supported_on "$name" "$HOST_OS"; then
        echo "Warning: [$name] is not supported on $HOST_OS. Skipping build." >&2
        return 0
    fi

    echo "==> [$name] Building in $target_dir (profile: $profile)..."
    local flags=()
    if [ "$profile" = "release" ]; then
        flags+=(--release)
    fi

    (cd "$target_dir" && cargo build "${flags[@]}")
}

cmd_install() {
    local name="$1"
    local profile="${2:-release}"
    local repo
    repo="$(plugin_repo "$name")"
    local bin
    bin="$(plugin_bin "$name")"
    local target_dir="$SIBLING_ROOT/$repo"
    local bin_path="$target_dir/target/$profile/$bin"

    # Fallback to debug if release binary doesn't exist
    if [ ! -f "$bin_path" ] && [ "$profile" = "release" ] && [ -f "$target_dir/target/debug/$bin" ]; then
        bin_path="$target_dir/target/debug/$bin"
        profile="debug"
    fi

    if [ ! -f "$bin_path" ]; then
        echo "Error: [$name] Binary not found at $bin_path. Run build first." >&2
        return 1
    fi

    mkdir -p "$PLUGINS_DIR"
    local dest="$PLUGINS_DIR/$bin"

    echo "==> [$name] Installing $bin into $PLUGINS_DIR/..."
    if [ "${USE_COPY:-0}" = "1" ] || [ "$HOST_OS" = "windows" ]; then
        cp -f "$bin_path" "$dest"
        echo "    Copied to $dest"
    else
        # Relative symlink from plugins/ to ../../repo/target/...
        local rel_path="../../$repo/target/$profile/$bin"
        ln -sf "$rel_path" "$dest"
        echo "    Symlinked $dest -> $rel_path"
    fi
}

cmd_clean() {
    local name="$1"
    local bin
    bin="$(plugin_bin "$name")"
    local dest="$PLUGINS_DIR/$bin"
    if [ -e "$dest" ] || [ -L "$dest" ]; then
        echo "==> [$name] Removing $dest..."
        rm -f "$dest"
    else
        echo "==> [$name] Nothing to clean in $PLUGINS_DIR."
    fi
}

cmd_status() {
    echo "Host OS: $HOST_OS"
    echo "Workspace root: $WORKSPACE_ROOT"
    echo "Plugins dir:    $PLUGINS_DIR"
    echo ""
    printf "%-8s %-22s %-12s %-12s %-12s\n" "Plugin" "Type" "Checkout" "Build" "Installed"
    printf "%-8s %-22s %-12s %-12s %-12s\n" "------" "----" "--------" "-----" "---------"

    for p in link syphon spout; do
        local repo
        repo="$(plugin_repo "$p")"
        local bin
        bin="$(plugin_bin "$p")"
        local ptype
        ptype="$(plugin_type "$p")"
        local target_dir="$SIBLING_ROOT/$repo"

        local checkout_st="[ ] missing"
        if [ -d "$target_dir/.git" ]; then
            checkout_st="[✓] present"
        fi

        local build_st="[ ] none"
        if [ -f "$target_dir/target/release/$bin" ]; then
            build_st="[✓] release"
        elif [ -f "$target_dir/target/debug/$bin" ]; then
            build_st="[✓] debug"
        fi

        local install_st="[ ] none"
        if [ -L "$PLUGINS_DIR/$bin" ]; then
            install_st="[✓] symlink"
        elif [ -f "$PLUGINS_DIR/$bin" ]; then
            install_st="[✓] file"
        fi

        local supported=""
        if ! plugin_supported_on "$p" "$HOST_OS"; then
            supported=" (non-native)"
        fi

        printf "%-8s %-22s %-12s %-12s %-12s%s\n" "$p" "$ptype" "$checkout_st" "$build_st" "$install_st" "$supported"
    done
    echo ""
}

resolve_plugins() {
    local target="$1"
    if [ "$target" = "all" ]; then
        case "$HOST_OS" in
            macos)   echo "syphon link" ;;
            windows) echo "spout link" ;;
            linux)   echo "link" ;;
            *)       echo "link" ;;
        esac
    else
        case "$target" in
            link|syphon|spout)
                echo "$target"
                ;;
            *)
                echo "Error: Unknown plugin '$target'. Valid plugins: link, syphon, spout, all" >&2
                exit 1
                ;;
        esac
    fi
}

usage() {
    cat <<EOF
Usage: $(basename "$0") <command> [plugin] [options]
       $(basename "$0") [plugin]

Manage Karakuri plugin extensions (Ableton Link, Syphon, Spout).

Commands:
  setup [plugin]     Clone, build, and install specified plugin (default: 'all' for this OS)
  clone [plugin]     Clone sibling repository (../Karakuri-<name>)
  build [plugin]     Build plugin using cargo
  install [plugin]   Install (symlink/copy) built binary into plugins/
  clean [plugin]     Remove installed plugin from plugins/
  status             Show current checkout, build, and install status
  help               Show this help message

Plugins:
  link     Ableton Link tempo source (input)       [macOS, Linux, Windows]
  syphon   Syphon output sink (output)             [macOS]
  spout    Spout2 output sink (output)             [Windows]
  all      All native plugins for current platform (default if omitted)

Options:
  --debug            Build/install debug profile instead of release
  --copy             Copy binary instead of symlinking into plugins/

Examples:
  $(basename "$0") status
  $(basename "$0") setup syphon
  $(basename "$0") setup all
  $(basename "$0") syphon             # Shortcut for 'setup syphon'
  $(basename "$0") build link --debug
EOF
}

# Parse options
COMMAND=""
TARGET=""
PROFILE="release"
USE_COPY=0

while [ $# -gt 0 ]; do
    case "$1" in
        --debug)
            PROFILE="debug"
            shift
            ;;
        --release)
            PROFILE="release"
            shift
            ;;
        --copy)
            USE_COPY=1
            shift
            ;;
        -h|--help|help)
            usage
            exit 0
            ;;
        status)
            COMMAND="status"
            shift
            ;;
        setup|clone|build|install|clean)
            COMMAND="$1"
            shift
            if [ $# -gt 0 ] && ! [[ "$1" =~ ^-- ]]; then
                TARGET="$1"
                shift
            fi
            ;;
        link|syphon|spout|all)
            if [ -z "$COMMAND" ]; then
                COMMAND="setup"
                TARGET="$1"
            else
                TARGET="$1"
            fi
            shift
            ;;
        *)
            echo "Error: Unknown argument '$1'" >&2
            usage
            exit 1
            ;;
    esac
done

if [ -z "$COMMAND" ]; then
    COMMAND="status"
fi

if [ "$COMMAND" = "status" ]; then
    cmd_status
    exit 0
fi

if [ -z "$TARGET" ]; then
    TARGET="all"
fi

PLUGINS_TO_RUN="$(resolve_plugins "$TARGET")"

for p in $PLUGINS_TO_RUN; do
    case "$COMMAND" in
        setup)
            cmd_clone "$p"
            if plugin_supported_on "$p" "$HOST_OS"; then
                cmd_build "$p" "$PROFILE"
                cmd_install "$p" "$PROFILE"
            fi
            ;;
        clone)
            cmd_clone "$p"
            ;;
        build)
            cmd_build "$p" "$PROFILE"
            ;;
        install)
            cmd_install "$p" "$PROFILE"
            ;;
        clean)
            cmd_clean "$p"
            ;;
    esac
done

echo ""
echo "Done! Current status:"
cmd_status
