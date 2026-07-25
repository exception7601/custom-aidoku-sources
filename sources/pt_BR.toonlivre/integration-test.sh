#!/usr/bin/env bash
#
# ToonLivre Integration Tests
#
# Starts token server, runs live WASM tests, then stops the server.
# Tests marked with live:test require the token server running.
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TOKEN_SERVER_DIR="$REPO_ROOT/token-server"
TOKEN_SERVER_PID=""

log_info() {
	echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
	echo -e "${GREEN}[✓]${NC} $1"
}

log_error() {
	echo -e "${RED}[✗]${NC} $1"
}

log_warning() {
	echo -e "${YELLOW}[!]${NC} $1"
}

cleanup() {
	if [[ -n "$TOKEN_SERVER_PID" ]]; then
		log_info "Stopping token server (PID: $TOKEN_SERVER_PID)..."
		kill "$TOKEN_SERVER_PID" 2>/dev/null || true
		wait "$TOKEN_SERVER_PID" 2>/dev/null || true
		log_success "Token server stopped"
	fi
}

trap cleanup EXIT INT TERM

check_dependencies() {
	log_info "Checking dependencies..."

	if ! command -v bun &>/dev/null; then
		log_error "bun is not installed. Install from https://bun.sh"
		exit 1
	fi

	if ! command -v cargo &>/dev/null; then
		log_error "cargo is not installed"
		exit 1
	fi

	log_success "All dependencies found"
}

check_token_server() {
	if curl -s http://localhost:3000/health >/dev/null 2>&1; then
		return 0
	fi
	return 1
}

start_token_server() {
	log_info "Checking if token server is already running..."

	if check_token_server; then
		log_warning "Token server already running, skipping start"
		return 0
	fi

	log_info "Starting token server..."

	if [[ ! -d "$TOKEN_SERVER_DIR" ]]; then
		log_error "Token server directory not found: $TOKEN_SERVER_DIR"
		exit 1
	fi

	cd "$TOKEN_SERVER_DIR"

	if [[ ! -d "node_modules" ]]; then
		log_info "Installing token server dependencies..."
		bun install
	fi

	# Start server in background, redirect output to log file
	bun run src/server.ts >"$TOKEN_SERVER_DIR/server.log" 2>&1 &
	TOKEN_SERVER_PID=$!

	log_info "Token server starting (PID: $TOKEN_SERVER_PID)..."

	# Wait for server to be ready
	local max_attempts=30
	local attempt=0
	while [[ $attempt -lt $max_attempts ]]; do
		if check_token_server; then
			log_success "Token server is ready"
			return 0
		fi
		((attempt++))
		sleep 1
	done

	log_error "Token server failed to start within ${max_attempts}s"
	log_info "Check logs at: $TOKEN_SERVER_DIR/server.log"
	exit 1
}

run_cargo_tests() {
	log_info "Running cargo tests with dev-server feature (including live:test)..."

	cd "$SCRIPT_DIR"

	if cargo test --lib --features dev-server 2>&1 | tee /tmp/toonlivre-test-output.log; then
		log_success "All tests passed"
		return 0
	else
		log_error "Some tests failed"
		return 1
	fi
}

show_test_summary() {
	log_info "Test summary:"
	echo ""

	if [[ -f /tmp/toonlivre-test-output.log ]]; then
		# Extract test results
		grep "test result:" /tmp/toonlivre-test-output.log || true
	fi
}

main() {
	echo "================================================"
	echo "ToonLivre Integration Tests (WASM Environment)"
	echo "================================================"
	echo ""

	check_dependencies
	echo ""

	start_token_server
	echo ""

	local exit_code=0
	if ! run_cargo_tests; then
		exit_code=1
	fi
	echo ""

	show_test_summary
	echo ""

	if [[ $exit_code -eq 0 ]]; then
		log_success "Integration tests completed successfully"
	else
		log_error "Integration tests failed"
	fi

	echo "================================================"

	exit $exit_code
}

main "$@"
