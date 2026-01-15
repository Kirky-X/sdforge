#!/bin/bash
# =============================================================================
# Axiom Pre-commit Check Script
# =============================================================================
# This script performs comprehensive checks before allowing a git commit.
# It checks code formatting, linting, compilation, and more.
#
# Usage:
#   ./pre-commit-check.sh                    # Run all checks
#   ./pre-commit-check.sh --check-conflicts  # Check for merge conflicts
#   ./pre-commit-check.sh --check-large-files # Check for large files
#   ./pre-commit-check.sh --fix-whitespace   # Fix trailing whitespace
#   ./pre-commit-check.sh --check-compile    # Check compilation only
#   ./pre-commit-check.sh --check-build      # Check build only
#   ./pre-commit-check.sh --check-format     # Check formatting only
#   ./pre-commit-check.sh --check-clippy     # Check clippy only
#   ./pre-commit-check.sh --help             # Show this help
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
MAX_FILE_SIZE_MB=1
MAX_FILE_SIZE_BYTES=$((MAX_FILE_SIZE_MB * 1024 * 1024))
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LOG_FILE="${PROJECT_ROOT}/.pre-commit-check.log"

# =============================================================================
# Utility Functions
# =============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${RESET} $1"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [INFO] $1" >> "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}[✓]${RESET} $1"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [SUCCESS] $1" >> "$LOG_FILE"
}

log_warning() {
    echo -e "${YELLOW}[!]${RESET} $1"
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [WARNING] $1" >> "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[✗]${RESET} $1" >&2
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] [ERROR] $1" >> "$LOG_FILE"
}

log_section() {
    echo ""
    echo -e "${BOLD}${CYAN}═══════════════════════════════════════════════════════════════${RESET}"
    echo -e "${BOLD}${CYAN}  $1${RESET}"
    echo -e "${BOLD}${CYAN}═══════════════════════════════════════════════════════════════${RESET}"
    echo "" >> "$LOG_FILE"
    echo "══════════════════════════════════════════════════════════════=" >> "$LOG_FILE"
    echo "  $1" >> "$LOG_FILE"
    echo "══════════════════════════════════════════════════════════════=" >> "$LOG_FILE"
}

print_duration() {
    local duration=$1
    if (( duration < 60 )); then
        echo "${duration}s"
    else
        local minutes=$((duration / 60))
        local seconds=$((duration % 60))
        echo "${minutes}m ${seconds}s"
    fi
}

check_prerequisites() {
    log_info "Checking prerequisites..."
    
    local missing_deps=()
    
    # Check for Rust
    if ! command -v cargo &> /dev/null; then
        missing_deps+=("cargo (Rust)")
    fi
    
    # Check for git
    if ! command -v git &> /dev/null; then
        missing_deps+=("git")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        log_info "Please install the missing dependencies and try again."
        exit 1
    fi
    
    log_success "All prerequisites satisfied"
}

start_timer() {
    START_TIME=$(date +%s)
}

stop_timer() {
    END_TIME=$(date +%s)
    DURATION=$((END_TIME - START_TIME))
}

# =============================================================================
# Check Functions
# =============================================================================

check_merge_conflicts() {
    log_section "Checking for Merge Conflict Markers"
    start_timer
    
    local conflicts_found=0
    local conflict_patterns=("<<<<<<< HEAD" "=======" ">>>>>>> ")
    
    # Check staged files
    local staged_files
    staged_files=$(git diff --cached --name-only -- '*.rs' '*.toml' '*.md' '*.yml' '*.yaml' 2>/dev/null || echo "")
    
    if [ -z "$staged_files" ]; then
        log_warning "No staged files to check"
        stop_timer
        return 0
    fi
    
    while IFS= read -r file; do
        if [ -f "$file" ]; then
            for pattern in "${conflict_patterns[@]}"; do
                if grep -q "$pattern" "$file" 2>/dev/null; then
                    log_error "Merge conflict marker found in: $file"
                    grep -n "$pattern" "$file" 2>/dev/null | head -5
                    conflicts_found=1
                fi
            done
        fi
    done <<< "$staged_files"
    
    stop_timer
    
    if [ $conflicts_found -eq 0 ]; then
        log_success "No merge conflict markers found ($(print_duration $DURATION))"
        return 0
    else
        log_error "Merge conflict markers detected!"
        log_info "Solution: Remove the conflict markers from the files listed above."
        return 1
    fi
}

check_large_files() {
    log_section "Checking for Large Files (>${MAX_FILE_SIZE_MB}MB)"
    start_timer
    
    local large_files=()
    
    # Check staged files
    local staged_files
    staged_files=$(git diff --cached --name-only 2>/dev/null || echo "")
    
    if [ -z "$staged_files" ]; then
        log_warning "No staged files to check"
        stop_timer
        return 0
    fi
    
    while IFS= read -r file; do
        if [ -f "$file" ]; then
            local file_size
            file_size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo "0")
            file_size=$((file_size))
            
            if [ $file_size -gt $MAX_FILE_SIZE_BYTES ]; then
                local size_mb=$((file_size / 1024 / 1024))
                large_files+=("$file (${size_mb}MB)")
            fi
        fi
    done <<< "$staged_files"
    
    stop_timer
    
    if [ ${#large_files[@]} -eq 0 ]; then
        log_success "No large files found ($(print_duration $DURATION))"
        return 0
    else
        log_error "Large files detected:"
        for file in "${large_files[@]}"; do
            echo -e "  ${RED}  - $file${RESET}"
        done
        log_info "Solution: Use Git LFS for large files or remove them from staging."
        return 1
    fi
}

fix_trailing_whitespace() {
    log_section "Checking and Fixing Trailing Whitespace"
    start_timer
    
    local files_fixed=0
    local files_with_issues=()
    
    # Check staged Rust files
    local staged_files
    staged_files=$(git diff --cached --name-only -- '*.rs' 2>/dev/null || echo "")
    
    if [ -z "$staged_files" ]; then
        log_warning "No staged Rust files to check"
        stop_timer
        return 0
    fi
    
    while IFS= read -r file; do
        if [ -f "$file" ]; then
            # Check for trailing whitespace
            if grep -q '[[:space:]]$' "$file" 2>/dev/null; then
                files_with_issues+=("$file")
                
                # Fix trailing whitespace
                sed -i 's/[[:space:]]*$//' "$file" 2>/dev/null || \
                sed -i 's/[[:space:]]\+$//' "$file" 2>/dev/null
                
                # Re-stage the fixed file
                git add "$file" 2>/dev/null || true
                ((files_fixed++)) || true
            fi
        fi
    done <<< "$staged_files"
    
    stop_timer
    
    if [ $files_fixed -eq 0 ]; then
        log_success "No trailing whitespace issues found ($(print_duration $DURATION))"
        return 0
    else
        log_warning "Fixed $files_fixed file(s) with trailing whitespace:"
        for file in "${files_with_issues[@]}"; do
            echo -e "  ${YELLOW}  - $file${RESET}"
        done
        log_info "Files have been fixed and re-staged."
        return 0
    fi
}

check_code_formatting() {
    log_section "Checking Code Formatting (cargo fmt)"
    start_timer
    
    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust."
        stop_timer
        return 1
    fi
    
    # Check if rustfmt is available
    if ! cargo fmt --version &> /dev/null; then
        log_warning "rustfmt not available. Installing..."
        rustup component add rustfmt 2>/dev/null || {
            log_error "Failed to install rustfmt"
            stop_timer
            return 1
        }
    fi
    
    # Run formatting check
    local fmt_output
    fmt_output=$(cargo fmt --all -- --check 2>&1) && fmt_result=0 || fmt_result=$?
    
    stop_timer
    
    if [ $fmt_result -eq 0 ]; then
        log_success "Code formatting is correct ($(print_duration $DURATION))"
        return 0
    else
        log_error "Code formatting issues detected!"
        echo "$fmt_output" | head -30
        log_info "Solution: Run 'cargo fmt --all' to fix formatting issues."
        return 1
    fi
}

check_code_quality() {
    log_section "Checking Code Quality (cargo clippy)"
    start_timer
    
    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust."
        stop_timer
        return 1
    fi
    
    # Check if clippy is available
    if ! cargo clippy --version &> /dev/null; then
        log_warning "clippy not available. Installing..."
        rustup component add clippy 2>/dev/null || {
            log_error "Failed to install clippy"
            stop_timer
            return 1
        }
    fi
    
    log_info "Running clippy with --all-features and -- -D warnings..."
    
    # Run clippy with all features
    local clippy_output
    local clippy_result=0
    clippy_output=$(cargo clippy --all-targets --all-features --workspace -- -D warnings 2>&1) || clippy_result=$?
    
    # Filter out some known warnings that are acceptable
    local filtered_output
    filtered_output=$(echo "$clippy_output" | grep -v "warning: unused import" || true)
    
    stop_timer
    
    if [ $clippy_result -eq 0 ] || [ -z "$filtered_output" ]; then
        log_success "Code quality check passed ($(print_duration $DURATION))"
        return 0
    else
        log_error "Code quality issues detected!"
        echo "$filtered_output" | head -50
        
        if [ $(echo "$filtered_output" | wc -l) -gt 50 ]; then
            log_info "... (output truncated, run 'cargo clippy --all-features --workspace' for full output)"
        fi
        
        log_info "Solution: Review the warnings above and fix them, or run 'cargo clippy --fix' for auto-fixable issues."
        return 1
    fi
}

check_compilation() {
    log_section "Checking Compilation (cargo check)"
    start_timer
    
    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust."
        stop_timer
        return 1
    fi
    
    log_info "Running cargo check with --all-features..."
    
    # Run cargo check with all features
    local check_output
    local check_result=0
    check_output=$(cargo check --all-features --workspace 2>&1) || check_result=$?
    
    stop_timer
    
    if [ $check_result -eq 0 ]; then
        log_success "Compilation check passed ($(print_duration $DURATION))"
        return 0
    else
        log_error "Compilation errors detected!"
        echo "$check_output" | head -50
        
        log_info "Solution: Fix the compilation errors listed above."
        return 1
    fi
}

check_build() {
    log_section "Checking Build (cargo build)"
    start_timer
    
    if ! command -v cargo &> /dev/null; then
        log_error "cargo not found. Please install Rust."
        stop_timer
        return 1
    fi
    
    log_info "Running cargo build with --all-features..."
    
    # Run cargo build with all features
    local build_output
    local build_result=0
    build_output=$(cargo build --all-features --workspace 2>&1) || build_result=$?
    
    stop_timer
    
    if [ $build_result -eq 0 ]; then
        log_success "Build check passed ($(print_duration $DURATION))"
        return 0
    else
        log_error "Build errors detected!"
        echo "$build_output" | head -50
        
        log_info "Solution: Fix the build errors listed above."
        return 1
    fi
}

# =============================================================================
# Main Functions
# =============================================================================

run_all_checks() {
    local overall_start=$(date +%s)
    local failed_checks=()
    
    # Clear log file
    : > "$LOG_FILE"
    
    log_section "Axiom Pre-commit Checks"
    log_info "Project: $(basename "$PROJECT_ROOT")"
    log_info "Rust version: $(rustc --version 2>/dev/null || echo 'N/A')"
    log_info "Cargo version: $(cargo --version 2>/dev/null || echo 'N/A')"
    echo ""
    
    # Prerequisites
    check_prerequisites || exit 1
    echo ""
    
    # Run checks
    local checks=(
        "check_merge_conflicts"
        "check_large_files"
        "fix_trailing_whitespace"
        "check_code_formatting"
        "check_code_quality"
        "check_compilation"
        "check_build"
    )
    
    for check in "${checks[@]}"; do
        start_timer
        if ! $check; then
            failed_checks+=("$check")
        fi
        echo ""
    done
    
    local overall_end=$(date +%s)
    local overall_duration=$((overall_end - overall_start))
    
    log_section "Pre-commit Check Summary"
    echo ""
    
    if [ ${#failed_checks[@]} -eq 0 ]; then
        log_success "All checks passed! ✓"
        log_info "Total time: $(print_duration $overall_duration)"
        echo ""
        log_info "You can now commit your changes."
        echo ""
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
        echo -e "${GREEN}  Commit ready!${RESET}"
        echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
        echo ""
        return 0
    else
        log_error "Some checks failed! ✗"
        log_info "Total time: $(print_duration $overall_duration)"
        echo ""
        log_error "Failed checks:"
        for check in "${failed_checks[@]}"; do
            echo -e "  ${RED}  - $check${RESET}"
        done
        echo ""
        log_info "Please fix the issues above and try again."
        log_info "Log file: $LOG_FILE"
        echo ""
        echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
        echo -e "${RED}  Commit blocked - fix the issues above${RESET}"
        echo -e "${RED}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${RESET}"
        echo ""
        return 1
    fi
}

show_help() {
    cat << EOF
${BOLD}Axiom Pre-commit Check Script${RESET}

${BOLD}Usage:${RESET}
  $(basename "$0") [OPTIONS]

${BOLD}Options:${RESET}
  --check-conflicts    Check for merge conflict markers
  --check-large-files  Check for files larger than ${MAX_FILE_SIZE_MB}MB
  --fix-whitespace     Fix trailing whitespace in staged files
  --check-format       Check code formatting only (rustfmt)
  --check-clippy       Check code quality only (clippy)
  --check-compile      Check compilation only (cargo check)
  --check-build        Check build only (cargo build)
  --all                Run all checks (default)
  --help, -h           Show this help message

${BOLD}Examples:${RESET}
  $(basename "$0")                    # Run all checks
  $(basename "$0") --check-format     # Check formatting only
  $(basename "$0") --check-compile    # Check compilation only

${BOLD}Installation:${RESET}
  To install as a pre-commit hook, run:
    ./scripts/install-pre-commit.sh

${BOLD}For more information:${RESET}
  See .pre-commit-config.yaml
EOF
}

# =============================================================================
# Main Entry Point
# =============================================================================

main() {
    # Parse arguments
    local mode="all"
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --check-conflicts)
                mode="check_conflicts"
                shift
                ;;
            --check-large-files)
                mode="check_large_files"
                shift
                ;;
            --fix-whitespace)
                mode="fix_whitespace"
                shift
                ;;
            --check-format)
                mode="check_format"
                shift
                ;;
            --check-clippy)
                mode="check_clippy"
                shift
                ;;
            --check-compile)
                mode="check_compile"
                shift
                ;;
            --check-build)
                mode="check_build"
                shift
                ;;
            --all)
                mode="all"
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
    
    # Run the appropriate mode
    case $mode in
        check_conflicts)
            check_merge_conflicts
            ;;
        check_large_files)
            check_large_files
            ;;
        fix_whitespace)
            fix_trailing_whitespace
            ;;
        check_format)
            check_code_formatting
            ;;
        check_clippy)
            check_code_quality
            ;;
        check_compile)
            check_compilation
            ;;
        check_build)
            check_build
            ;;
        all)
            run_all_checks
            ;;
    esac
}

# Run main function
main "$@"
