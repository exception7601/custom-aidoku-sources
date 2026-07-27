#!/usr/bin/env bash
#
# ToonLivre Integration Tests
#
# Starts proxy server, runs live WASM tests, then stops the server.
# Tests marked with live:test require the proxy server running.
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
PROXY_DIR="$REPO_ROOT/toons-total-proxy"
PROXY_PID=""

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
	if [[ -n "$PROXY_PID" ]]; then
		log_info "Stopping proxy server (PID: $PROXY_PID)..."
		kill "$PROXY_PID" 2>/dev/null || true
		wait "$PROXY_PID" 2>/dev/null || true
		log_success "Proxy server stopped"
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

check_proxy() {
	if curl -s http://localhost:4000/health >/dev/null 2>&1; then
		return 0
	fi
	return 1
}

start_proxy() {
	log_info "Checking if proxy server is already running..."

	if check_proxy; then
		log_warning "Proxy server already running, skipping start"
		return 0
	fi

	log_info "Starting proxy server..."

	if [[ ! -d "$PROXY_DIR" ]]; then
		log_error "Proxy directory not found: $PROXY_DIR"
		exit 1
	fi

	cd "$PROXY_DIR"

	if [[ ! -d "node_modules" ]]; then
		log_info "Installing proxy server dependencies..."
		bun install
	fi

	# Start server in background, redirect output to log file
	PORT=4000 bun run src/index.ts >"$PROXY_DIR/server.log" 2>&1 &
	PROXY_PID=$!

	log_info "Proxy server starting (PID: $PROXY_PID)..."

	# Wait for server to be ready
	local max_attempts=30
	local attempt=0
	while [[ $attempt -lt $max_attempts ]]; do
		if check_proxy; then
			log_success "Proxy server is ready"
			
			# Show encryption status
			local enc_status=$(curl -s http://localhost:4000/health | grep -o '"enabled":[^,}]*' | cut -d':' -f2)
			log_info "Encryption mode: $enc_status"
			
			return 0
		fi
		((attempt++))
		sleep 1
	done

	log_error "Proxy server failed to start within ${max_attempts}s"
	log_info "Check logs at: $PROXY_DIR/server.log"
	exit 1
}

run_cargo_tests() {
	log_info "Running cargo tests (including live:test)..."

	cd "$SCRIPT_DIR"

	if cargo test --lib 2>&1 | tee /tmp/toonlivre-test-output.log; then
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
		echo ""
		
		# Show proxy stats if available
		log_info "Proxy statistics:"
		curl -s http://localhost:4000/api/logs/stats | grep -E '"total"|"errors"|"avgResponseTime"' || true
	fi
}

main() {
	echo "================================================"
	echo "ToonLivre Integration Tests (WASM Environment)"
	echo "Using Proxy Server v2.0.0"
	echo "================================================"
	echo ""

	check_dependencies
	echo ""

	start_proxy
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
