#!/bin/bash
# Author: aav
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
read -r BLUE YELLOW RESET <<<"$(bash $UTILS_DIR/colors.sh blue yellow reset)"

# --------------------------------------------------
# clean before
# --------------------------------------------------
# CMD="cargo clean"
# [[ $PRINT_CMD == true ]] && echo $CMD
# [[ $DRY_RUN == false ]] && eval "$cMD"

# --------------------------------------------------
# start bundle command
# --------------------------------------------------
DX_TARGET="web"
PKG_NAME=$(basename "$CRATE_DIR")
CMD="dx bundle --$DX_TARGET --package $PKG_NAME"

# --------------------------------------------------
# is release
# --------------------------------------------------
if [ "$IS_RELEASE" = true ]; then
    # maybe? want to investigate this:
    # so this is fine for non-reactive elements
    # but the moment reactive elements are included, then
    # it explodes in size (even if small)
    #
    # need to investiage this more, and how to
    # do more wasm splitting
    # CMD+=" --inject-loading-scripts false"
    # # maybe? on multiple pages?
    # CMD+=" --wasm-split"
    # yes 100%
    CMD+=" --debug-symbols false --release"
fi
[[ $VERBOSE == true ]] && CMD+=" --verbose"

# # --------------------------------------------------
# # after verbose, add client cargo/rustc args
# # --------------------------------------------------
# if [ "$IS_RELEASE" = true ]; then
#     CMD+=" @client \
#     	--cargo-args ' \
#     	\\-\\-config profile.wasm-release.strip=true
#     	\\-\\-config profile.wasm-release.inherits=\\\"release\\\"
#     	\\-\\-config profile.wasm-release.opt-level=\\\"z\\\"
#     	\\-\\-config profile.wasm-release.lto=true
#     	\\-\\-config profile.wasm-release.codegen-units=1
#     '"
#     # --rustc-args "
#     # 	-Zunstable-options
#     # 	-Cpanic=immediate-abort
#     # " \
# fi
# [[ $PRINT_CMD == true ]] && echo $CMD

# --------------------------------------------------
# THIS COMES RIGHT BEFORE EVAL since there is a @server
# in the command
# still issues with mold + wasm, for both client and server
# but would be nice to uncomment:
# # --------------------------------------------------
# # rust flags: check if `mold` linker exists, for faster building
# # --------------------------------------------------
# RUSTC_ARGS=""
# if command -v mold >/dev/null 2>&1; then
#     RUSTC_ARGS="-Clink-arg=-fuse-ld=mold"
# fi
# CMD+=" @server --rustc-args \"$RUSTC_ARGS\""
# --------------------------------------------------

# --------------------------------------------------
# bundle
# --------------------------------------------------
[[ $DRY_RUN == false ]] && eval "$CMD" || {
    echo "Cargo command failed"
    exit 1
}

# --------------------------------------------------
# find the wasm file inside of target dir, and optimize
# for size using `wasm-opt`
# --------------------------------------------------
# TARGET_DIR="$(pwd)/target/dx/$PKG_NAME/release/$DX_TARGET"
# WASM_FILE=$(find "$TARGET_DIR" -name "*.wasm")
# if [ -n "$WASM_FILE" ]; then
#     wasm-opt "$WASM_FILE" -o "$WASM_FILE" -Oz
# fi

# --------------------------------------------------
# start tar command
# --------------------------------------------------
VERBOSE_FLAG=$([[ $VERBOSE == true ]] && echo "v")
tar_args=(
    # --------------------------------------------------
    # create gzip compressed archive
    # --------------------------------------------------
    -${VERBOSE_FLAG}czf "$OUTPUT"

    # --------------------------------------------------
    # global exclude
    # --------------------------------------------------
    --exclude '*.sh'
    --exclude '*.md'

    # --------------------------------------------------
    # ch dir to web? bundle
    # * include all
    # * exclude manifest
    # --------------------------------------------------
    -C "$(pwd)/target/dx/$PKG_NAME/release/$DX_TARGET"
    --exclude .manifest.json
    .

    # # --------------------------------------------------
    # # ch dir to container
    # # * include all
    # # --------------------------------------------------
    # -C "$(pwd)/container"
    # .

    # --------------------------------------------------
    # ch dir to root
    # * include docker-compose.yml
    # --------------------------------------------------
    -C "$(pwd)"
    ./docker-compose.yml
)

# --------------------------------------------------
# tar
# --------------------------------------------------
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
    echo "Use the following command extract:"
    echo -e "   ${YELLOW}tar -xzf $OUTPUT --one-top-level=unbundled$RESET"
fi
