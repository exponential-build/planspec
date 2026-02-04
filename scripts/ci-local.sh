#!/usr/bin/env bash
#
# Run CI checks locally by parsing .github/workflows/ci.yaml
# This ensures local validation matches actual CI behavior.
#
# Usage: ./scripts/ci-local.sh [job-name]
#   Without args: runs all jobs
#   With job name: runs only that job (e.g., lint, build, test-unit)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKFLOW_FILE="$REPO_ROOT/.github/workflows/ci.yaml"

cd "$REPO_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check dependencies
if ! command -v yq &> /dev/null; then
    echo -e "${RED}Error: yq is required but not installed.${NC}"
    echo "Install with: brew install yq (macOS) or snap install yq (Linux)"
    exit 1
fi

if [ ! -f "$WORKFLOW_FILE" ]; then
    echo -e "${RED}Error: Workflow file not found: $WORKFLOW_FILE${NC}"
    exit 1
fi

# Get list of jobs from workflow
get_jobs() {
    yq '.jobs | keys | .[]' "$WORKFLOW_FILE"
}

# Get number of steps with 'run' commands for a job
get_step_count() {
    local job="$1"
    yq ".jobs.${job}.steps | map(select(.run)) | length" "$WORKFLOW_FILE"
}

# Get a specific step's run command (0-indexed)
get_step_run() {
    local job="$1"
    local index="$2"
    yq ".jobs.${job}.steps | map(select(.run)) | .[$index].run" "$WORKFLOW_FILE"
}

# Get a specific step's name (0-indexed)
get_step_name() {
    local job="$1"
    local index="$2"
    yq ".jobs.${job}.steps | map(select(.run)) | .[$index].name // \"Step $((index+1))\"" "$WORKFLOW_FILE"
}

# Get job name/description
get_job_name() {
    local job="$1"
    yq ".jobs.${job}.name // \"${job}\"" "$WORKFLOW_FILE"
}

# Check if job has dependencies that we should skip
should_skip_job() {
    local job="$1"

    # For local testing, we can run test-integration and validate-specs
    # if we build first
    case "$job" in
        test-integration|validate-specs)
            # Check if binary exists
            if [ ! -f "target/release/planspec" ]; then
                echo "skip-needs-build"
                return
            fi
            ;;
    esac

    echo "run"
}

# Run a single job
run_job() {
    local job="$1"
    local job_name
    job_name=$(get_job_name "$job")

    echo -e "\n${BLUE}════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}Job: ${job_name} (${job})${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}\n"

    local skip_status
    skip_status=$(should_skip_job "$job")

    if [ "$skip_status" = "skip-needs-build" ]; then
        echo -e "${YELLOW}⚠ Skipping: requires release build. Run 'cargo build --workspace --release' first.${NC}"
        return 1
    fi

    local step_count
    step_count=$(get_step_count "$job")

    if [ "$step_count" -eq 0 ]; then
        echo -e "${YELLOW}No run steps found for job: $job${NC}"
        return 0
    fi

    # Integration tests use CARGO_BIN_EXE_planspec set by cargo

    # Run each step
    for ((i=0; i<step_count; i++)); do
        local step_name
        local step_run
        step_name=$(get_step_name "$job" "$i")
        step_run=$(get_step_run "$job" "$i")

        echo -e "${YELLOW}Step $((i+1)): ${step_name}${NC}"
        # Show first few lines of the command
        echo "$step_run" | head -5
        [ "$(echo "$step_run" | wc -l)" -gt 5 ] && echo "..."
        echo ""

        # Execute the step and explicitly check exit status
        # (set -e is disabled when function is called in if-condition)
        local exit_code=0
        eval "$step_run" || exit_code=$?

        if [ "$exit_code" -ne 0 ]; then
            echo -e "${RED}✗ Step $((i+1)) failed (exit code: $exit_code)${NC}\n"
            return 1
        fi

        echo -e "${GREEN}✓ Step $((i+1)) passed${NC}\n"
    done

    echo -e "${GREEN}✓ Job '$job_name' passed${NC}"
}

# Main
main() {
    local target_job="$1"
    local failed_jobs=()
    local passed_jobs=()
    local skipped_jobs=()

    echo -e "${BLUE}PlanSpec CI Local Runner${NC}"
    echo -e "${BLUE}Workflow: $WORKFLOW_FILE${NC}"
    echo ""

    local jobs
    if [ -n "$target_job" ]; then
        jobs="$target_job"
    else
        jobs=$(get_jobs)
    fi

    for job in $jobs; do
        if run_job "$job"; then
            passed_jobs+=("$job")
        else
            # Check if it was skipped vs failed
            if [ "$(should_skip_job "$job")" = "skip-needs-build" ]; then
                skipped_jobs+=("$job")
            else
                failed_jobs+=("$job")
            fi
        fi
    done

    # Summary
    echo -e "\n${BLUE}════════════════════════════════════════════════════════════${NC}"
    echo -e "${BLUE}Summary${NC}"
    echo -e "${BLUE}════════════════════════════════════════════════════════════${NC}"

    if [ ${#passed_jobs[@]} -gt 0 ]; then
        echo -e "${GREEN}Passed:${NC} ${passed_jobs[*]}"
    fi

    if [ ${#skipped_jobs[@]} -gt 0 ]; then
        echo -e "${YELLOW}Skipped:${NC} ${skipped_jobs[*]}"
    fi

    if [ ${#failed_jobs[@]} -gt 0 ]; then
        echo -e "${RED}Failed:${NC} ${failed_jobs[*]}"
        exit 1
    fi

    if [ ${#skipped_jobs[@]} -gt 0 ]; then
        echo -e "\n${YELLOW}Some jobs were skipped. To run all jobs:${NC}"
        echo "  cargo build --workspace --release"
        echo "  ./scripts/ci-local.sh"
    fi

    echo -e "\n${GREEN}✓ All executed jobs passed!${NC}"
}

main "$@"
