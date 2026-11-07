#!/bin/bash
# ==============================================================================
# Meetily Linux Build Script (No Local LLMs / Hosted Mode)
# ==============================================================================
# Builds Meetily for Linux configured for hosted models (LiteLLM / OpenAI ASR).
# Bypasses local LLM compilation (llama.cpp) and Ollama downloads.
# ==============================================================================

set -e

# ANSI Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m' # No Color

echo -e "${BLUE}${BOLD}======================================================${NC}"
echo -e "${BLUE}${BOLD}  🐧 Meetily Linux Build (Hosted Models Mode)        ${NC}"
echo -e "${BLUE}${BOLD}======================================================${NC}"
echo -e "${CYAN}ℹ️  Local LLM setup (Ollama & llama.cpp) will be bypassed.${NC}"
echo -e "${CYAN}ℹ️  Configured for hosted ASR & LiteLLM endpoints.${NC}"
echo ""

# 1. Verify OS
if [[ "$OSTYPE" != "linux"* ]]; then
    echo -e "${RED}❌ Error: This script is intended for Linux systems.${NC}"
    exit 1
fi

# 2. Determine project directories
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [ -f "$SCRIPT_DIR/Cargo.toml" ] && [ -d "$SCRIPT_DIR/frontend" ]; then
    ROOT_DIR="$SCRIPT_DIR"
    FRONTEND_DIR="$SCRIPT_DIR/frontend"
elif [ -f "$SCRIPT_DIR/package.json" ] && [ -d "$SCRIPT_DIR/src-tauri" ]; then
    ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
    FRONTEND_DIR="$SCRIPT_DIR"
else
    echo -e "${RED}❌ Error: Could not locate project root directory.${NC}"
    exit 1
fi

echo -e "${BLUE}📁 Project Root:${NC} $ROOT_DIR"
echo -e "${BLUE}📁 Frontend Dir:${NC} $FRONTEND_DIR"
echo ""

# 3. Parse command-line flags
BUILD_MODE="release"
BUNDLE_TYPES=""
RUN_DEV=false

while [[ "$#" -gt 0 ]]; do
    case "$1" in
        --debug)
            BUILD_MODE="debug"
            shift
            ;;
        --release)
            BUILD_MODE="release"
            shift
            ;;
        --dev)
            RUN_DEV=true
            shift
            ;;
        --bundles)
            BUNDLE_TYPES="$2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: ./build-linux.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --release       Build production release bundle (default)"
            echo "  --debug         Build debug bundle"
            echo "  --dev           Run in live development mode"
            echo "  --bundles TYPE  Comma-separated bundle formats (e.g. appimage,deb)"
            echo "  -h, --help      Show this help message"
            exit 0
            ;;
        *)
            echo -e "${YELLOW}⚠️ Unknown argument: $1${NC}"
            shift
            ;;
    esac
done

# 4. Check system prerequisites
echo -e "${BLUE}🔍 Checking build dependencies...${NC}"

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

if ! command_exists rustc || ! command_exists cargo; then
    echo -e "${RED}❌ Rust is not installed or not in PATH.${NC}"
    echo -e "${YELLOW}💡 Run: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
    exit 1
fi

if ! command_exists node; then
    echo -e "${RED}❌ Node.js is not installed.${NC}"
    echo -e "${YELLOW}💡 Install Node.js (v20+): curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash - && sudo apt install -y nodejs${NC}"
    exit 1
fi

if command_exists pnpm; then
    PKG_MGR="pnpm"
elif command_exists npm; then
    PKG_MGR="npm"
else
    echo -e "${RED}❌ Neither pnpm nor npm was found.${NC}"
    echo -e "${YELLOW}💡 Install pnpm: npm install -g pnpm${NC}"
    exit 1
fi

RUST_VERSION=$(rustc --version)
NODE_VERSION=$(node --version)
echo -e "${GREEN}✅ Rust:${NC}     $RUST_VERSION"
echo -e "${GREEN}✅ Node.js:${NC}  $NODE_VERSION"
echo -e "${GREEN}✅ Package:${NC}  $PKG_MGR"
echo ""

# 5. Detect Rust Target Triple
TARGET_TRIPLE=$(rustc -vV | grep "host:" | awk '{print $2}')
if [ -z "$TARGET_TRIPLE" ]; then
    TARGET_TRIPLE="x86_64-unknown-linux-gnu"
fi
echo -e "${BLUE}🎯 Target Triple:${NC} $TARGET_TRIPLE"

# 6. Setup sidecar stub (bypasses heavy llama.cpp compilation)
echo ""
echo -e "${BLUE}⚙️  Configuring sidecars (bypassing local llama-helper compilation)...${NC}"
BINARIES_DIR="$FRONTEND_DIR/src-tauri/binaries"
mkdir -p "$BINARIES_DIR"

STUB_BINARY="$BINARIES_DIR/llama-helper-$TARGET_TRIPLE"
if [ ! -f "$STUB_BINARY" ]; then
    echo -e "${CYAN}Creating lightweight sidecar stub...${NC}"
    rustc - -o "$STUB_BINARY" << 'EOF'
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        if let Ok(l) = line {
            if l.contains("ping") {
                let _ = writeln!(stdout, "{{\"type\":\"pong\"}}");
                let _ = stdout.flush();
            } else if l.contains("shutdown") {
                let _ = writeln!(stdout, "{{\"type\":\"goodbye\"}}");
                let _ = stdout.flush();
                break;
            } else {
                let _ = writeln!(stdout, "{{\"type\":\"response\",\"text\":\"\",\"error\":\"Local LLM is disabled; using hosted models.\"}}");
                let _ = stdout.flush();
            }
        }
    }
}
EOF
    chmod +x "$STUB_BINARY"
    echo -e "${GREEN}✅ Sidecar stub created:${NC} $STUB_BINARY"
else
    echo -e "${GREEN}✅ Existing sidecar stub verified:${NC} $STUB_BINARY"
fi

# 7. Install frontend dependencies
echo ""
echo -e "${BLUE}📦 Installing frontend dependencies with $PKG_MGR...${NC}"
cd "$FRONTEND_DIR"
if [ "$PKG_MGR" = "pnpm" ]; then
    pnpm install --frozen-lockfile 2>/dev/null || pnpm install
else
    npm install
fi

# 8. Run in Dev or Build Mode
# AppImage packaging requires NO_STRIP=true to avoid glibc symbol stripping issues
export NO_STRIP=true

if [ "$RUN_DEV" = true ]; then
    echo ""
    echo -e "${GREEN}${BOLD}🚀 Launching Meetily in live development mode...${NC}"
    $PKG_MGR run tauri dev
    exit 0
fi

# Build Next.js frontend
echo ""
echo -e "${BLUE}🏗️  Building Next.js frontend...${NC}"
$PKG_MGR run build

# Build Tauri Application
echo ""
echo -e "${BLUE}🔨 Compiling native Linux Tauri application ($BUILD_MODE)...${NC}"

TAURI_ARGS="build"
if [ "$BUILD_MODE" = "debug" ]; then
    TAURI_ARGS="build --debug"
fi

if [ -n "$BUNDLE_TYPES" ]; then
    TAURI_ARGS="$TAURI_ARGS --bundles $BUNDLE_TYPES"
fi

echo -e "${CYAN}Running: $PKG_MGR tauri $TAURI_ARGS${NC}"
$PKG_MGR tauri $TAURI_ARGS

echo ""
echo -e "${GREEN}${BOLD}======================================================${NC}"
echo -e "${GREEN}${BOLD}  🎉 Build Completed Successfully!                   ${NC}"
echo -e "${GREEN}${BOLD}======================================================${NC}"
echo ""

# Find and display built artifacts
TARGET_BUNDLE_DIR="$ROOT_DIR/target/$BUILD_MODE/bundle"
if [ ! -d "$TARGET_BUNDLE_DIR" ]; then
    TARGET_BUNDLE_DIR="$FRONTEND_DIR/src-tauri/target/$BUILD_MODE/bundle"
fi
if [ -d "$TARGET_BUNDLE_DIR" ]; then
    echo -e "${BLUE}📦 Generated Linux packages:${NC}"
    find "$TARGET_BUNDLE_DIR" -type f \( -name "*.AppImage" -o -name "*.deb" -o -name "*.rpm" \) -exec ls -lh {} + 2>/dev/null || true
    echo ""
    echo -e "${CYAN}To run the AppImage:${NC}"
    APPIMAGE_PATH=$(find "$TARGET_BUNDLE_DIR/appimage" -name "*.AppImage" 2>/dev/null | head -n 1 || true)
    if [ -n "$APPIMAGE_PATH" ]; then
        echo -e "  chmod +x \"$APPIMAGE_PATH\""
        echo -e "  \"$APPIMAGE_PATH\""
    fi
fi
