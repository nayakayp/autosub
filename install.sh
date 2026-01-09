#!/bin/bash
#
# Autosub Installer
# 
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/nayakayp/autosub/main/install.sh | bash
#
# Or download and run:
#   ./install.sh
#
# Options:
#   --version VERSION    Install a specific version (default: latest)
#   --install-dir DIR    Install to a specific directory (default: /usr/local/bin)
#   --help               Show this help message
#

set -e

# Configuration
REPO="nayakayp/autosub"  # UPDATE: Set to your actual GitHub repo (e.g., "owner/repo")
BINARY_NAME="autosub"
DEFAULT_INSTALL_DIR="/usr/local/bin"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print colored messages
info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Show help
show_help() {
    cat << EOF
Autosub Installer

A CLI tool for automatic subtitle generation using OpenAI Whisper or Google Gemini.

USAGE:
    ./install.sh [OPTIONS]

OPTIONS:
    --version VERSION    Install a specific version (e.g., v0.1.0)
                         Default: latest release
    --install-dir DIR    Install binary to DIR
                         Default: /usr/local/bin
    --help               Show this help message

EXAMPLES:
    # Install latest version to /usr/local/bin
    ./install.sh

    # Install specific version
    ./install.sh --version v0.1.0

    # Install to custom directory
    ./install.sh --install-dir ~/.local/bin

REQUIREMENTS:
    - curl or wget
    - tar (for extraction)
    - FFmpeg (runtime dependency)

After installation, set up your API keys:
    export OPENAI_API_KEY="sk-..."
    export GEMINI_API_KEY="..."

EOF
    exit 0
}

# Parse arguments
VERSION="latest"
INSTALL_DIR="$DEFAULT_INSTALL_DIR"

while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --install-dir)
            INSTALL_DIR="$2"
            shift 2
            ;;
        --help|-h)
            show_help
            ;;
        *)
            error "Unknown option: $1. Use --help for usage."
            ;;
    esac
done

# Detect OS and architecture
detect_platform() {
    local os arch
    
    # Detect OS
    case "$(uname -s)" in
        Linux*)
            os="linux"
            ;;
        Darwin*)
            os="macos"
            ;;
        MINGW*|MSYS*|CYGWIN*)
            os="windows"
            ;;
        *)
            error "Unsupported operating system: $(uname -s)"
            ;;
    esac
    
    # Detect architecture
    case "$(uname -m)" in
        x86_64|amd64)
            arch="x86_64"
            ;;
        arm64|aarch64)
            arch="aarch64"
            ;;
        *)
            error "Unsupported architecture: $(uname -m)"
            ;;
    esac
    
    echo "${os}-${arch}"
}

# Get latest version from GitHub API
get_latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    local version
    
    if command -v curl &> /dev/null; then
        version=$(curl -fsSL "$url" | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
    elif command -v wget &> /dev/null; then
        version=$(wget -qO- "$url" | grep '"tag_name"' | head -1 | sed -E 's/.*"([^"]+)".*/\1/')
    else
        error "Neither curl nor wget found. Please install one of them."
    fi
    
    if [[ -z "$version" ]]; then
        error "Failed to get latest version from GitHub"
    fi
    
    echo "$version"
}

# Download binary
download_binary() {
    local version="$1"
    local platform="$2"
    local dest="$3"
    
    # Construct artifact name
    local artifact="${BINARY_NAME}-${platform}"
    if [[ "$platform" == "windows-x86_64" ]]; then
        artifact="${artifact}.exe"
    fi
    
    local url="https://github.com/${REPO}/releases/download/${version}/${artifact}"
    local temp_file=$(mktemp)
    
    info "Downloading ${BINARY_NAME} ${version} for ${platform}..."
    
    if command -v curl &> /dev/null; then
        if ! curl -fsSL "$url" -o "$temp_file"; then
            rm -f "$temp_file"
            error "Failed to download from: $url"
        fi
    elif command -v wget &> /dev/null; then
        if ! wget -q "$url" -O "$temp_file"; then
            rm -f "$temp_file"
            error "Failed to download from: $url"
        fi
    fi
    
    mv "$temp_file" "$dest"
    success "Downloaded successfully"
}

# Verify checksum (optional)
verify_checksum() {
    local version="$1"
    local platform="$2"
    local file="$3"
    
    local artifact="${BINARY_NAME}-${platform}"
    if [[ "$platform" == "windows-x86_64" ]]; then
        artifact="${artifact}.exe"
    fi
    
    local checksums_url="https://github.com/${REPO}/releases/download/${version}/checksums.txt"
    local temp_checksums=$(mktemp)
    
    info "Verifying checksum..."
    
    # Download checksums file
    if command -v curl &> /dev/null; then
        curl -fsSL "$checksums_url" -o "$temp_checksums" 2>/dev/null || {
            warn "Could not download checksums file, skipping verification"
            rm -f "$temp_checksums"
            return 0
        }
    elif command -v wget &> /dev/null; then
        wget -q "$checksums_url" -O "$temp_checksums" 2>/dev/null || {
            warn "Could not download checksums file, skipping verification"
            rm -f "$temp_checksums"
            return 0
        }
    fi
    
    # Get expected checksum
    local expected=$(grep "$artifact" "$temp_checksums" | awk '{print $1}')
    rm -f "$temp_checksums"
    
    if [[ -z "$expected" ]]; then
        warn "Checksum not found for $artifact, skipping verification"
        return 0
    fi
    
    # Calculate actual checksum
    local actual
    if command -v sha256sum &> /dev/null; then
        actual=$(sha256sum "$file" | awk '{print $1}')
    elif command -v shasum &> /dev/null; then
        actual=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        warn "No sha256sum or shasum found, skipping checksum verification"
        return 0
    fi
    
    if [[ "$expected" != "$actual" ]]; then
        error "Checksum mismatch!\nExpected: $expected\nActual:   $actual"
    fi
    
    success "Checksum verified"
}

# Install binary
install_binary() {
    local src="$1"
    local dest_dir="$2"
    local dest="${dest_dir}/${BINARY_NAME}"
    
    # Create install directory if needed
    if [[ ! -d "$dest_dir" ]]; then
        info "Creating directory: $dest_dir"
        mkdir -p "$dest_dir" || {
            error "Failed to create directory. Try running with sudo or use --install-dir"
        }
    fi
    
    # Check if we can write to the directory
    if [[ ! -w "$dest_dir" ]]; then
        info "Root privileges required to install to $dest_dir"
        sudo mv "$src" "$dest"
        sudo chmod +x "$dest"
    else
        mv "$src" "$dest"
        chmod +x "$dest"
    fi
    
    success "Installed to: $dest"
}

# Check for FFmpeg
check_ffmpeg() {
    if ! command -v ffmpeg &> /dev/null; then
        warn "FFmpeg not found. autosub requires FFmpeg to extract audio."
        echo ""
        echo "Install FFmpeg:"
        echo "  macOS:   brew install ffmpeg"
        echo "  Ubuntu:  sudo apt install ffmpeg"
        echo "  Windows: choco install ffmpeg"
        echo ""
    else
        success "FFmpeg found: $(ffmpeg -version 2>&1 | head -1)"
    fi
}

# Main installation flow
main() {
    echo ""
    echo "╔═══════════════════════════════════════════════════════════╗"
    echo "║                    Autosub Installer                      ║"
    echo "║    Automatic subtitle generation with AI transcription    ║"
    echo "╚═══════════════════════════════════════════════════════════╝"
    echo ""
    
    # Detect platform
    local platform
    platform=$(detect_platform)
    info "Detected platform: $platform"
    
    # Get version
    if [[ "$VERSION" == "latest" ]]; then
        VERSION=$(get_latest_version)
    fi
    info "Version: $VERSION"
    
    # Download
    local temp_binary=$(mktemp)
    download_binary "$VERSION" "$platform" "$temp_binary"
    
    # Verify checksum
    verify_checksum "$VERSION" "$platform" "$temp_binary"
    
    # Install
    install_binary "$temp_binary" "$INSTALL_DIR"
    
    # Check FFmpeg
    echo ""
    check_ffmpeg
    
    # Print success message
    echo ""
    echo "═══════════════════════════════════════════════════════════"
    success "Installation complete!"
    echo ""
    echo "To get started:"
    echo "  1. Set up your API key:"
    echo "     export OPENAI_API_KEY=\"sk-...\""
    echo "     # or"
    echo "     export GEMINI_API_KEY=\"...\""
    echo ""
    echo "  2. Generate subtitles:"
    echo "     autosub video.mp4 -o subtitles.srt"
    echo ""
    echo "  3. View help:"
    echo "     autosub --help"
    echo ""
    
    # Verify installation
    if command -v "$BINARY_NAME" &> /dev/null; then
        success "$BINARY_NAME is ready to use!"
    else
        warn "$INSTALL_DIR may not be in your PATH"
        echo "Add it with: export PATH=\"\$PATH:$INSTALL_DIR\""
    fi
}

# Run main
main
