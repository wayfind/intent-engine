#!/bin/bash
set -euo pipefail

# ie CLI Manager - Install, Upgrade, Uninstall
# Usage: ie-manager.sh [install|upgrade|uninstall] [version] [-y|--force]

FORCE_MODE=false

REPO="wayfind/intent-engine"
BINARY_NAME="ie"

# Ensure HOME is set (may be unset in CI environments)
if [[ -z "${HOME:-}" ]]; then
    echo "[ERROR] HOME environment variable is not set" >&2
    exit 1
fi

INSTALL_DIR="${HOME}/.local/bin"
DATA_DIR="${HOME}/.intent-engine"

# Colors (disabled if not a terminal)
if [[ -t 1 ]]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    NC='\033[0m'
else
    RED=''
    GREEN=''
    YELLOW=''
    NC=''
fi

info() { echo -e "${GREEN}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; exit 1; }

detect_platform() {
    local os arch

    case "$(uname -s)" in
        Linux*)  os="linux" ;;
        Darwin*) os="macos" ;;
        *)       error "Unsupported OS: $(uname -s). Use ie-manager.ps1 for Windows." ;;
    esac

    case "$(uname -m)" in
        x86_64|amd64)  arch="x86_64" ;;
        aarch64|arm64) arch="aarch64" ;;
        *)             error "Unsupported architecture: $(uname -m)" ;;
    esac

    echo "${os}-${arch}"
}

get_latest_version() {
    local http_code body
    local tmp_file
    tmp_file=$(mktemp)

    # Cleanup function for this temp file
    cleanup_tmp() { rm -f "${tmp_file}" 2>/dev/null; }

    # Use -w to capture HTTP status code
    http_code=$(curl -sL -w "%{http_code}" -o "${tmp_file}" \
        ${GITHUB_TOKEN:+-H "Authorization: token ${GITHUB_TOKEN}"} \
        "https://api.github.com/repos/${REPO}/releases/latest") || {
        cleanup_tmp
        error "Failed to connect to GitHub API"
    }

    body=$(cat "${tmp_file}")
    cleanup_tmp

    case "${http_code}" in
        200)
            echo "${body}" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'
            ;;
        403)
            error "GitHub API rate limit exceeded. Set GITHUB_TOKEN env var or wait."
            ;;
        404)
            error "Release not found. Check repository: ${REPO}"
            ;;
        *)
            error "GitHub API error (HTTP ${http_code})"
            ;;
    esac
}

get_current_version() {
    if command -v "${BINARY_NAME}" &>/dev/null; then
        # Extract version number using regex - handles various formats:
        # "ie 0.10.10", "ie version 0.10.10", "0.10.10", etc.
        local output
        output=$("${BINARY_NAME}" --version 2>/dev/null | head -1) || true
        if [[ "${output}" =~ ([0-9]+\.[0-9]+\.[0-9]+) ]]; then
            echo "${BASH_REMATCH[1]}"
        else
            echo ""
        fi
    else
        echo ""
    fi
}

download_and_install() {
    local version="${1:-$(get_latest_version)}"
    local platform
    platform=$(detect_platform)

    local asset_name="intent-engine-${platform}.tar.gz"
    local download_url="https://github.com/${REPO}/releases/download/${version}/${asset_name}"

    info "Downloading ${BINARY_NAME} ${version} for ${platform}..."

    local tmp_dir
    tmp_dir=$(mktemp -d)
    # Use subshell trap to avoid unbound variable when script exits from other paths
    trap '[[ -d "${tmp_dir:-}" ]] && rm -rf "${tmp_dir}"' EXIT

    # Download with retry
    local max_retries=3
    local retry_delay=2
    local attempt=1

    while [[ ${attempt} -le ${max_retries} ]]; do
        if curl -fsSL "${download_url}" -o "${tmp_dir}/${asset_name}"; then
            break
        fi
        if [[ ${attempt} -eq ${max_retries} ]]; then
            error "Failed to download from ${download_url} after ${max_retries} attempts"
        fi
        warn "Download failed, retrying ($((attempt + 1))/${max_retries})..."
        sleep ${retry_delay}
        ((attempt++))
    done

    # Optional: Verify checksum if SHA256SUMS is available
    local checksum_url="https://github.com/${REPO}/releases/download/${version}/SHA256SUMS"
    local checksum_file="${tmp_dir}/SHA256SUMS"

    if curl -fsSL "${checksum_url}" -o "${checksum_file}" 2>/dev/null; then
        info "Verifying checksum..."
        local expected_hash
        # Match exact filename at end of line (format: "hash  filename" or "hash *filename")
        expected_hash=$(awk -v name="${asset_name}" '$NF == name || $NF == "*"name {print $1; exit}' "${checksum_file}")
        if [[ -n "${expected_hash}" ]]; then
            local actual_hash=""
            if command -v sha256sum &>/dev/null; then
                actual_hash=$(sha256sum "${tmp_dir}/${asset_name}" | awk '{print $1}')
            elif command -v shasum &>/dev/null; then
                actual_hash=$(shasum -a 256 "${tmp_dir}/${asset_name}" | awk '{print $1}')
            else
                warn "No sha256sum or shasum available - skipping checksum verification"
            fi

            if [[ -n "${actual_hash}" ]]; then
                if [[ "${actual_hash}" != "${expected_hash}" ]]; then
                    error "Checksum mismatch! Expected: ${expected_hash}, Got: ${actual_hash}"
                fi
                info "Checksum verified"
            fi
        else
            warn "Asset not found in SHA256SUMS - skipping verification"
        fi
    fi

    info "Extracting..."
    tar -xzf "${tmp_dir}/${asset_name}" -C "${tmp_dir}"

    # Create install directory if needed
    mkdir -p "${INSTALL_DIR}"

    # Find and install binary
    local binary_path
    binary_path=$(find "${tmp_dir}" -name "${BINARY_NAME}" -type f | head -1)
    if [[ -z "${binary_path}" ]]; then
        error "Binary '${BINARY_NAME}' not found in archive"
    fi

    # Handle running binary: remove first, then move (works even if binary is running)
    local dest="${INSTALL_DIR}/${BINARY_NAME}"
    if [[ -f "${dest}" ]]; then
        rm -f "${dest}" 2>/dev/null || {
            warn "Cannot replace running binary. Stop any ie processes and retry."
            error "Failed to remove existing binary at ${dest}"
        }
    fi
    mv "${binary_path}" "${dest}"
    chmod +x "${dest}"

    info "Installed to ${INSTALL_DIR}/${BINARY_NAME}"

    # Check PATH
    if [[ ":${PATH}:" != *":${INSTALL_DIR}:"* ]]; then
        warn "${INSTALL_DIR} is not in your PATH"
        echo ""
        echo "Add it to your shell profile:"
        echo "  echo 'export PATH=\"\${HOME}/.local/bin:\${PATH}\"' >> ~/.bashrc"
        echo "  # or for zsh:"
        echo "  echo 'export PATH=\"\${HOME}/.local/bin:\${PATH}\"' >> ~/.zshrc"
        echo ""
    fi

    # Verify
    if "${INSTALL_DIR}/${BINARY_NAME}" --version &>/dev/null; then
        info "Successfully installed: $(${INSTALL_DIR}/${BINARY_NAME} --version)"
    fi
}

do_install() {
    local version="${1:-}"

    if command -v "${BINARY_NAME}" &>/dev/null; then
        local current
        current=$(get_current_version)
        warn "${BINARY_NAME} is already installed (${current})"
        echo "Use 'upgrade' to update or 'uninstall' first"
        exit 1
    fi

    download_and_install "${version}"
    info "Installation complete!"
}

# Compare semantic versions: returns 0 if equal, 1 if v1 > v2, 2 if v1 < v2
# Only compares numeric parts (ignores prerelease suffixes like -beta)
version_compare() {
    local v1="${1#v}" v2="${2#v}"

    # Extract only numeric parts (strip -beta, -rc, etc.)
    v1="${v1%%-*}"
    v2="${v2%%-*}"

    local IFS='.'
    local i v1_parts v2_parts

    read -ra v1_parts <<< "${v1}"
    read -ra v2_parts <<< "${v2}"

    for ((i=0; i<3; i++)); do
        # Ensure numeric comparison (default to 0 if not a number)
        local n1="${v1_parts[i]:-0}"
        local n2="${v2_parts[i]:-0}"
        # Strip any non-numeric suffix and validate
        n1="${n1%%[!0-9]*}"
        n2="${n2%%[!0-9]*}"
        n1="${n1:-0}"
        n2="${n2:-0}"

        if ((n1 > n2)); then return 1; fi
        if ((n1 < n2)); then return 2; fi
    done
    return 0
}

do_upgrade() {
    local version="${1:-$(get_latest_version)}"
    local current
    current=$(get_current_version)

    if [[ -z "${current}" ]]; then
        warn "${BINARY_NAME} is not installed. Running install instead..."
        download_and_install "${version}"
    else
        # Strip 'v' prefix for comparison if present
        local current_clean="${current#v}"
        local version_clean="${version#v}"

        version_compare "${current_clean}" "${version_clean}"
        local cmp_result=$?

        if [[ ${cmp_result} -eq 0 ]]; then
            info "Already at version ${version}"
            exit 0
        elif [[ ${cmp_result} -eq 1 ]]; then
            warn "Current version (${current}) is newer than target (${version})"
            warn "Use explicit version to downgrade if intended"
            exit 0
        fi

        info "Upgrading from ${current} to ${version}..."
        download_and_install "${version}"
    fi

    info "Upgrade complete!"
}

do_uninstall() {
    local binary_path="${INSTALL_DIR}/${BINARY_NAME}"
    local removed=false

    # Remove binary
    if [[ -f "${binary_path}" ]]; then
        rm -f "${binary_path}"
        info "Removed ${binary_path}"
        removed=true
    else
        warn "Binary not found at ${binary_path}"

        # Check if installed elsewhere
        local alt_path
        alt_path=$(command -v "${BINARY_NAME}" 2>/dev/null || true)
        if [[ -n "${alt_path}" ]]; then
            warn "${BINARY_NAME} found at ${alt_path}"
            echo "If installed via other methods, use:"
            echo "  npm uninstall -g @origintask/intent-engine"
            echo "  cargo uninstall intent-engine"
            echo "  brew uninstall intent-engine"
        fi
    fi

    # Ask about data directory
    if [[ -d "${DATA_DIR}" ]]; then
        echo ""
        if [[ "${FORCE_MODE}" == true ]]; then
            # Force mode - remove without asking
            rm -rf "${DATA_DIR}"
            info "Removed ${DATA_DIR}"
        elif [[ -t 0 ]]; then
            # Interactive mode - ask user (with 30s timeout)
            if read -t 30 -p "Remove data directory ${DATA_DIR}? [y/N] " -n 1 -r; then
                echo
                if [[ $REPLY =~ ^[Yy]$ ]]; then
                    rm -rf "${DATA_DIR}"
                    info "Removed ${DATA_DIR}"
                else
                    info "Kept ${DATA_DIR}"
                fi
            else
                echo
                warn "Timeout - keeping ${DATA_DIR}"
            fi
        else
            # Non-interactive (pipe) mode - keep data by default
            warn "Non-interactive mode. Use --force to remove data, or 'rm -rf ${DATA_DIR}'"
            info "Kept ${DATA_DIR}"
        fi
    fi

    if [[ "${removed}" == true ]]; then
        info "Uninstall complete!"
    fi
}

show_help() {
    cat <<EOF
ie CLI Manager - Install, Upgrade, Uninstall

Usage: $(basename "$0") <command> [version] [options]

Commands:
  install [version]   Install ie CLI (default: latest)
  upgrade [version]   Upgrade to specified version (default: latest)
  uninstall           Remove ie CLI and optionally data

Options:
  -y, --force         Skip confirmation prompts (for automation)

Examples:
  $(basename "$0") install
  $(basename "$0") install v0.10.10
  $(basename "$0") upgrade
  $(basename "$0") uninstall
  $(basename "$0") uninstall --force

Environment:
  GITHUB_TOKEN        GitHub API token (avoids rate limiting)

Install directory: ${INSTALL_DIR}
Data directory:    ${DATA_DIR}
EOF
}

# Main - parse arguments directly (not in subshell to preserve FORCE_MODE)
cmd=""
version=""

for arg in "$@"; do
    case "${arg}" in
        -y|--force) FORCE_MODE=true ;;
        -h|--help|help) cmd="help" ;;
        install|upgrade|uninstall) cmd="${arg}" ;;
        -*)
            error "Unknown option: ${arg}" ;;
        *)
            # Version must match semver pattern (with optional v prefix and prerelease suffix)
            if [[ "${arg}" =~ ^v?[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
                version="${arg}"
            elif [[ -z "${cmd}" ]]; then
                error "Unknown command: ${arg}"
            else
                error "Invalid version format: ${arg}. Expected: v1.2.3 or 1.2.3-beta"
            fi
            ;;
    esac
done

case "${cmd}" in
    install)   do_install "${version}" ;;
    upgrade)   do_upgrade "${version}" ;;
    uninstall) do_uninstall ;;
    help)      show_help ;;
    *)
        show_help
        exit 1
        ;;
esac
