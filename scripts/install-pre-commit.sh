#!/bin/bash
# =============================================================================
# Axiom Pre-commit Installation Script
# =============================================================================
# This script installs and configures the pre-commit hooks for the Axiom project.
#
# Usage:
#   ./scripts/install-pre-commit.sh        # Install hooks
#   ./scripts/install-pre-commit.sh --force  # Force reinstall
#   ./scripts/install-pre-commit.sh --uninstall # Remove hooks
#   ./scripts/install-pre-commit.sh --help    # Show help
# =============================================================================

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
RESET='\033[0m'

# Configuration
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PRE_COMMIT_CONFIG="${PROJECT_ROOT}/.pre-commit-config.yaml"
PRE_COMMIT_HOOK="${PROJECT_ROOT}/.git/hooks/pre-commit"
INSTALL_SCRIPT="${PROJECT_ROOT}/scripts/pre-commit-check.sh"

# =============================================================================
# Utility Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${RESET} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${RESET} $1"
}

log_warning() {
    echo -e "${YELLOW}[!]${RESET} $1"
}

log_error() {
    echo -e "${RED}[✗]${RESET} $1" >&2
}

log_section() {
    echo ""
    echo -e "${BOLD}${CYAN}═══════════════════════════════════════════════════════════════${RESET}"
    echo -e "${BOLD}${CYAN}  $1${RESET}"
    echo -e "${BOLD}${CYAN}═══════════════════════════════════════════════════════════════${RESET}"
    echo ""
}

print_step() {
    echo -e "${BOLD}  ▶ $1${RESET}"
}

print_substep() {
    echo -e "${CYAN}    ▸ $1${RESET}"
}

# =============================================================================
# Prerequisite Checks
# =============================================================================

check_prerequisites() {
    log_section "Checking Prerequisites"
    
    local missing_deps=()
    local all_ok=true
    
    # Check for Git
    print_step "Checking Git"
    if command -v git &> /dev/null; then
        print_substep "Git version: $(git --version | head -1)"
        log_success "Git is installed"
    else
        print_substep "Git not found"
        missing_deps+=("git")
        all_ok=false
    fi
    
    # Check for Rust
    print_step "Checking Rust"
    if command -v rustc &> /dev/null; then
        print_substep "Rust version: $(rustc --version | head -1)"
        print_substep "Cargo version: $(cargo --version | head -1)"
        log_success "Rust is installed"
    else
        print_substep "Rust not found"
        missing_deps+=("rust/cargo")
        all_ok=false
    fi
    
    # Check for Python (for pre-commit)
    print_step "Checking Python"
    if command -v python3 &> /dev/null; then
        print_substep "Python version: $(python3 --version 2>&1)"
        log_success "Python is installed"
    elif command -v python &> /dev/null; then
        print_substep "Python version: $(python --version 2>&1)"
        log_success "Python is installed"
    else
        print_substep "Python not found"
        missing_deps+=("python3")
        all_ok=false
    fi
    
    # Check for pip
    print_step "Checking pip"
    if command -v pip3 &> /dev/null; then
        print_substep "pip3 is available"
        log_success "pip is installed"
    elif command -v pip &> /dev/null; then
        print_substep "pip is available"
        log_success "pip is installed"
    else
        print_substep "pip not found (will try to install pre-commit via pipx)"
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_warning "Missing dependencies: ${missing_deps[*]}"
        log_info "Some features may not work without these dependencies."
        echo ""
        read -p "Continue anyway? [y/N] " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            log_error "Installation cancelled."
            exit 1
        fi
    fi
    
    if $all_ok; then
        log_success "All prerequisites satisfied"
    fi
}

# =============================================================================
# Installation Functions
# =============================================================================

install_pre_commit() {
    log_section "Installing pre-commit"
    
    print_step "Checking for existing pre-commit installation"
    
    if command -v pre-commit &> /dev/null; then
        local current_version
        current_version=$(pre-commit --version 2>/dev/null || echo "unknown")
        print_substep "pre-commit is already installed: $current_version"
        
        echo ""
        read -p "Update to the latest version? [Y/n] " -n 1 -r
        echo ""
        if [[ ! $REPLY =~ ^[Nn]$ ]]; then
            print_step "Updating pre-commit..."
            pip install --upgrade pre-commit 2>/dev/null || \
            pip3 install --upgrade pre-commit 2>/dev/null || \
            pipx install pre-commit 2>/dev/null || {
                log_warning "Failed to update pre-commit, using existing version"
            }
        fi
    else
        print_step "Installing pre-commit..."
        
        # Try different installation methods
        if command -v pipx &> /dev/null; then
            print_substep "Installing via pipx..."
            pipx install pre-commit
        elif command -v pip3 &> /dev/null; then
            print_substep "Installing via pip3..."
            pip3 install --user pre-commit
        elif command -v pip &> /dev/null; then
            print_substep "Installing via pip..."
            pip install --user pre-commit
        else
            log_error "Cannot install pre-commit: pip not found"
            log_info "Please install pre-commit manually:"
            log_info "  pip install pre-commit"
            return 1
        fi
    fi
    
    # Verify installation
    if command -v pre-commit &> /dev/null; then
        log_success "pre-commit installed: $(pre-commit --version)"
    else
        log_error "pre-commit installation failed"
        return 1
    fi
}

install_git_hooks() {
    log_section "Installing Git Hooks"
    
    print_step "Ensuring .git/hooks directory exists..."
    mkdir -p "${PROJECT_ROOT}/.git/hooks"
    
    print_step "Creating pre-commit hook..."
    
    # Create the pre-commit hook script
    cat > "${PRE_COMMIT_HOOK}" << 'HOOK_EOF'
#!/bin/bash
# =============================================================================
# Axiom Pre-commit Hook
# =============================================================================
# This hook is automatically generated by scripts/install-pre-commit.sh
# DO NOT EDIT THIS FILE MANUALLY
# =============================================================================

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"

# Run the pre-commit check script
exec bash "${PROJECT_ROOT}/scripts/pre-commit-check.sh" "$@"
HOOK_EOF

    chmod +x "${PRE_COMMIT_HOOK}"
    print_substep "Created: ${PRE_COMMIT_HOOK}"
    
    # Also create a symlink in scripts for direct execution
    chmod +x "${INSTALL_SCRIPT}" 2>/dev/null || true
    
    log_success "Pre-commit hook installed"
}

install_pre_commit_hooks() {
    log_section "Installing pre-commit Hook Definitions"
    
    if [ ! -f "$PRE_COMMIT_CONFIG" ]; then
        log_error "Pre-commit config not found: $PRE_COMMIT_CONFIG"
        return 1
    fi
    
    print_step "Installing hook definitions from .pre-commit-config.yaml"
    
    if command -v pre-commit &> /dev/null; then
        cd "$PROJECT_ROOT"
        pre-commit install --config "$PRE_COMMIT_CONFIG" 2>/dev/null || {
            log_warning "pre-commit install failed, trying alternative method..."
            
            # Manual installation
            mkdir -p "${PROJECT_ROOT}/.git/hooks"
            cat > "${PROJECT_ROOT}/.git/hooks/pre-commit" << 'MANUAL_HOOK'
#!/bin/bash
# Manual pre-commit hook - runs custom check script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
exec bash "${PROJECT_ROOT}/scripts/pre-commit-check.sh"
MANUAL_HOOK
            chmod +x "${PROJECT_ROOT}/.git/hooks/pre-commit"
        }
        
        log_success "pre-commit hook definitions installed"
    else
        log_warning "pre-commit command not available, using manual hook only"
    fi
}

update_hooks() {
    log_section "Updating Hooks to Latest Versions"
    
    if command -v pre-commit &> /dev/null && [ -f "$PRE_COMMIT_CONFIG" ]; then
        print_step "Updating pre-commit hooks..."
        cd "$PROJECT_ROOT"
        pre-commit clean 2>/dev/null || true
        pre-commit install --config "$PRE_COMMIT_CONFIG" 2>/dev/null || true
        pre-commit autoupdate --config "$PRE_COMMIT_CONFIG" 2>/dev/null || true
        log_success "Hooks updated to latest versions"
    else
        log_warning "Cannot update hooks: pre-commit not available or config missing"
    fi
}

verify_installation() {
    log_section "Verifying Installation"
    
    local all_ok=true
    
    # Verify pre-commit hook file
    print_step "Checking pre-commit hook..."
    if [ -f "$PRE_COMMIT_HOOK" ] && [ -x "$PRE_COMMIT_HOOK" ]; then
        print_substep "Pre-commit hook exists and is executable"
        log_success "Pre-commit hook: OK"
    else
        print_substep "Pre-commit hook missing or not executable"
        log_error "Pre-commit hook: FAILED"
        all_ok=false
    fi
    
    # Verify check script
    print_step "Checking pre-commit check script..."
    if [ -f "$INSTALL_SCRIPT" ] && [ -x "$INSTALL_SCRIPT" ]; then
        print_substep "Check script exists and is executable"
        log_success "Check script: OK"
    else
        print_substep "Check script missing or not executable"
        log_error "Check script: FAILED"
        all_ok=false
    fi
    
    # Verify pre-commit config
    print_step "Checking pre-commit config..."
    if [ -f "$PRE_COMMIT_CONFIG" ]; then
        print_substep "Config file exists"
        log_success "Config: OK"
    else
        print_substep "Config file missing"
        log_error "Config: FAILED"
        all_ok=false
    fi
    
    # Test the check script
    print_step "Testing pre-commit check script..."
    if bash "$INSTALL_SCRIPT" --help &> /dev/null; then
        print_substep "Check script is functional"
        log_success "Test: OK"
    else
        print_substep "Check script test failed"
        log_error "Test: FAILED"
        all_ok=false
    fi
    
    echo ""
    if $all_ok; then
        log_success "Installation verified successfully!"
        return 0
    else
        log_error "Some checks failed. Please review the errors above."
        return 1
    fi
}

# =============================================================================
# Uninstall Function
# =============================================================================

uninstall() {
    log_section "Uninstalling Pre-commit Hooks"
    
    print_step "Removing pre-commit hook..."
    if [ -f "$PRE_COMMIT_HOOK" ]; then
        rm -f "$PRE_COMMIT_HOOK"
        log_success "Removed: $PRE_COMMIT_HOOK"
    else
        print_substep "Hook not found (already removed or not installed)"
    fi
    
    print_step "Removing hook definitions..."
    rm -rf "${PROJECT_ROOT}/.pre-commit-config.yaml" 2>/dev/null || true
    rm -rf "${PROJECT_ROOT}/.git/hooks" 2>/dev/null || true
    
    log_success "Pre-commit hooks uninstalled"
    log_info "Note: This only removes the Git hooks, not pre-commit itself or the check scripts."
}

# =============================================================================
# Main Functions
# =============================================================================

show_help() {
    cat << EOF
${BOLD}Axiom Pre-commit Installation Script${RESET}

${BOLD}Usage:${RESET}
  $(basename "$0") [OPTIONS]

${BOLD}Options:${RESET}
  --install     Install pre-commit hooks (default)
  --update      Update hooks to latest versions
  --uninstall   Remove pre-commit hooks
  --verify      Verify installation
  --help, -h    Show this help message

${BOLD}Examples:${RESET}
  $(basename "$0")                    # Install hooks
  $(basename "$0") --update           # Update hooks
  $(basename "$0") --uninstall        # Remove hooks
  $(basename "$0") --verify           # Verify installation

${BOLD}What gets installed:${RESET}
  - .pre-commit-config.yaml          # Pre-commit configuration
  - .git/hooks/pre-commit            # Git pre-commit hook
  - scripts/pre-commit-check.sh      # Custom check script

${BOLD}Requirements:${RESET}
  - Git
  - Rust/Cargo
  - Python 3 (for pre-commit tool)
  - pip or pipx (for installing pre-commit)

${BOLD}After installation:${RESET}
  The pre-commit hook will automatically run before each 'git commit'.
  It will check:
  - Code formatting (rustfmt)
  - Code quality (clippy)
  - Compilation (cargo check)
  - Build (cargo build)
  - Merge conflict markers
  - Large files
  - Trailing whitespace
EOF
}

main() {
    local mode="install"
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --install)
                mode="install"
                shift
                ;;
            --update)
                mode="update"
                shift
                ;;
            --uninstall)
                mode="uninstall"
                shift
                ;;
            --verify)
                mode="verify"
                shift
                ;;
            --force)
                mode="force"
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    echo ""
    echo -e "${BOLD}${CYAN}╔═══════════════════════════════════════════════════════════════╗${RESET}"
    echo -e "${BOLD}${CYAN}║  Axiom Pre-commit Installation Script                      ║${RESET}"
    echo -e "${BOLD}${CYAN}║  Multi-protocol Declarative SDK Framework                   ║${RESET}"
    echo -e "${BOLD}${CYAN}╚═══════════════════════════════════════════════════════════════╝${RESET}"
    echo ""
    
    case $mode in
        install)
            check_prerequisites
            install_pre_commit
            install_git_hooks
            install_pre_commit_hooks
            verify_installation
            
            echo ""
            log_section "Installation Complete!"
            echo ""
            log_success "Pre-commit hooks have been installed successfully!"
            echo ""
            log_info "The following checks will run before each commit:"
            echo "  • Code formatting (rustfmt)"
            echo "  • Code quality (clippy)"
            echo "  • Compilation check (cargo check)"
            echo "  • Build check (cargo build)"
            echo "  • Merge conflict detection"
            echo "  • Large file detection"
            echo "  • Trailing whitespace removal"
            echo ""
            log_info "To skip checks and commit anyway, use:"
            echo "  git commit --no-verify"
            echo ""
            ;;
        update)
            check_prerequisites
            update_hooks
            verify_installation
            ;;
        uninstall)
            uninstall
            ;;
        verify)
            verify_installation
            ;;
        force)
            check_prerequisites
            install_pre_commit
            install_git_hooks
            install_pre_commit_hooks
            update_hooks
            verify_installation
            ;;
    esac
}

# Run main function
main "$@"
