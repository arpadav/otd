#!/bin/bash
# Author: aav
# --------------------------------------------------
# Builds frontend + backend and creates a distributable tarball
# --------------------------------------------------
usage() {
    echo "Usage: bash scripts/bundle.sh [-o|--output <FILE>] [-d|--debug] [-v|--verbose] [--dry-run] [-h|--help]"
    echo ""
    echo "Arguments:"
    echo "  -o|--output <FILE>  the output file path (default: bundle.tar.gz)"
    echo "  -d|--debug          build debug instead of release"
    echo "  -v|--verbose        verbose output"
    echo "  --dry-run           print commands only"
    echo "  -h|--help           help"
}

# --------------------------------------------------
# pre-req: get script/utils dir
# --------------------------------------------------
SCRIPTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
UTILS_DIR="$SCRIPTS_DIR/utils"
CRATE_DIR="$(realpath "$SCRIPTS_DIR/..")"

# --------------------------------------------------
# parse cli
# --------------------------------------------------
OUTPUT="bundle.tar.gz" # default output file path
IS_RELEASE=true        # default to release build
VERBOSE=false          # default to not verbose
PRINT_CMD=false        # default to not printing commands
DRY_RUN=false          # default to not dry run
while [[ $# -gt 0 ]]; do
    case "$1" in
        -o | --output)
            OUTPUT="$2"
            shift 2
            ;;
        -d | --debug)
            IS_RELEASE=false
            shift
            ;;
        -v | --verbose)
            VERBOSE=true
            PRINT_CMD=true
            shift
            ;;
        --dry-run)
            DRY_RUN=true
            PRINT_CMD=true
            shift
            ;;
        -h | --help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown argument: $1"
            usage
            exit 1
            ;;
    esac
done

# --------------------------------------------------
# get colors, if supported
# --------------------------------------------------
read -r BLUE YELLOW RESET <<<"$(bash "$UTILS_DIR/colors.sh" blue yellow reset)"

# --------------------------------------------------
# build frontend
# --------------------------------------------------
echo -e "${BLUE}Building frontend...${RESET}"
CMD="cd $CRATE_DIR/frontend && npm run build"
[[ $PRINT_CMD == true ]] && echo "$CMD"
[[ $DRY_RUN == false ]] && eval "$CMD" || {
    echo "Frontend build failed"
    exit 1
}

# --------------------------------------------------
# build backend (embeds frontend via rust-embed)
# --------------------------------------------------
echo -e "${BLUE}Building backend...${RESET}"
CARGO_CMD="cd $CRATE_DIR && cargo build"
[[ $IS_RELEASE == true ]] && CARGO_CMD+=" --release"
[[ $PRINT_CMD == true ]] && echo "$CARGO_CMD"
[[ $DRY_RUN == false ]] && eval "$CARGO_CMD" || {
    echo "Cargo build failed"
    exit 1
}

# --------------------------------------------------
# determine binary path
# --------------------------------------------------
if [ "$IS_RELEASE" = true ]; then
    BINARY="$CRATE_DIR/target/release/otd"
else
    BINARY="$CRATE_DIR/target/debug/otd"
fi

# --------------------------------------------------
# create tarball
# --------------------------------------------------
echo -e "${BLUE}Creating tarball...${RESET}"
VERBOSE_FLAG=$([[ $VERBOSE == true ]] && echo "v")
tar_args=(
    -${VERBOSE_FLAG}czf "$OUTPUT"
    -C "$(dirname "$BINARY")"
    "$(basename "$BINARY")"
)

CMD="tar ${tar_args[*]}"
[[ $PRINT_CMD == true ]] && echo "$CMD"
[[ $DRY_RUN == false ]] && eval "$CMD" || {
    echo "Tar command failed"
    exit 1
}

# --------------------------------------------------
# how to unarchive
# --------------------------------------------------
if [ "$DRY_RUN" = false ]; then
    echo -e "${YELLOW}Bundle created: $OUTPUT${RESET}"
    echo "Use the following command to extract:"
    echo -e "   ${YELLOW}tar -xzf $OUTPUT --one-top-level=unbundled${RESET}"
fi
