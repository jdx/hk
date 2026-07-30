#!/usr/bin/env bash
# Build hk with rustc PGO followed by a BOLT post-link layout rewrite.
#
# The instrumented binary is trained against a hermetic workload that covers
# clap startup, Pkl evaluation and caching, Git discovery/status, file
# filtering, batching, lock planning, and subprocess execution.
#
# Environment:
#   HK_PGO_PROFILE       Cargo profile (default: serious-pgo)
#   HK_PGO_TARGET        Optional target triple. The resulting binary must run
#                        on this host so the training phases can execute it.
#   HK_PGO_BUILD_TOOL    cargo or cross (default: cargo)
#   HK_PGO_BOLT          Set to 0 to stop after PGO (default: 1)
#   HK_PGO_FEATURES      Cargo features used by release builds
#                        (default: git2/vendored-libgit2,git2/vendored-openssl)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

PGO_PROFILE="${HK_PGO_PROFILE:-serious-pgo}"
PGO_TARGET="${HK_PGO_TARGET:-}"
PGO_BUILD_TOOL="${HK_PGO_BUILD_TOOL:-cargo}"
PGO_BOLT="${HK_PGO_BOLT:-1}"
PGO_FEATURES="${HK_PGO_FEATURES:-git2/vendored-libgit2,git2/vendored-openssl}"

PGO_DATA_DIR="$REPO_ROOT/target/pgo-data"
PGO_PROFRAW_DIR="$PGO_DATA_DIR/profraw"
PGO_MERGED="$PGO_DATA_DIR/merged.profdata"
BOLT_INSTR_BIN="$PGO_DATA_DIR/hk.instr"
BOLT_FDATA_PREFIX="$PGO_DATA_DIR/bolt"
BOLT_FDATA="$PGO_DATA_DIR/hk.fdata"
TRAIN_CONFIG="$REPO_ROOT/benchmark/pgo/hk.pkl"

target_arg=""
target_dir_part=""
if [ -n "$PGO_TARGET" ]; then
	target_arg="--target=$PGO_TARGET"
	target_dir_part="$PGO_TARGET/"
fi

RUSTC_HOST="$(rustc -vV | sed -n 's|^host: ||p')"
RUSTC_SYSROOT="$(rustc --print sysroot)"
LLVM_PROFDATA="$RUSTC_SYSROOT/lib/rustlib/$RUSTC_HOST/bin/llvm-profdata"
if [ ! -x "$LLVM_PROFDATA" ]; then
	echo "ERROR: llvm-profdata not found at $LLVM_PROFDATA" >&2
	echo "  Install with: rustup component add llvm-tools" >&2
	exit 1
fi

LLVM_BOLT=""
MERGE_FDATA=""
if [ "$PGO_BOLT" != "0" ]; then
	for candidate in \
		/usr/lib/llvm-18/bin/llvm-bolt \
		/usr/lib/llvm-19/bin/llvm-bolt \
		/usr/lib/llvm-20/bin/llvm-bolt; do
		if [ -x "$candidate" ]; then
			LLVM_BOLT="$candidate"
			break
		fi
	done
	if [ -z "$LLVM_BOLT" ]; then
		LLVM_BOLT="$(command -v llvm-bolt || true)"
	fi
	if [ -z "$LLVM_BOLT" ]; then
		echo "ERROR: HK_PGO_BOLT=1 but llvm-bolt is not installed" >&2
		exit 1
	fi
	bolt_bindir="$(dirname "$LLVM_BOLT")"
	if [ -x "$bolt_bindir/merge-fdata" ]; then
		MERGE_FDATA="$bolt_bindir/merge-fdata"
	else
		MERGE_FDATA="$(command -v merge-fdata || true)"
	fi
	if [ -z "$MERGE_FDATA" ]; then
		echo "ERROR: merge-fdata was not found next to llvm-bolt or on PATH" >&2
		exit 1
	fi
fi

mkdir -p "$PGO_PROFRAW_DIR"
rm -f "$PGO_PROFRAW_DIR"/*.profraw "$PGO_MERGED"
rm -f "$BOLT_INSTR_BIN" "$BOLT_FDATA"
find "$PGO_DATA_DIR" -maxdepth 1 -name 'bolt.*.fdata' -delete 2>/dev/null || true

# Cross mounts the repository at a container-specific path. Bind the profile
# directory at its host path as well so phase 3's -Cprofile-use path resolves.
if [ "$PGO_BUILD_TOOL" = "cross" ]; then
	export CROSS_CONTAINER_OPTS="${CROSS_CONTAINER_OPTS:-} -v $PGO_DATA_DIR:$PGO_DATA_DIR:rw"
fi

build() {
	local rustflags=$1
	# target_arg is intentionally word-split: an empty value must disappear.
	# shellcheck disable=SC2086
	RUSTFLAGS="$rustflags" "$PGO_BUILD_TOOL" build \
		--profile="$PGO_PROFILE" \
		$target_arg \
		--bin hk \
		--features "$PGO_FEATURES"
}

FINAL_BIN="$REPO_ROOT/target/${target_dir_part}${PGO_PROFILE}/hk"

echo ">>> [1/4] Building the PGO-instrumented binary"
build "-Cprofile-generate=$PGO_PROFRAW_DIR"
if [ ! -x "$FINAL_BIN" ]; then
	echo "ERROR: instrumented binary missing at $FINAL_BIN" >&2
	exit 1
fi

train() {
	local bin=$1
	local label=$2
	local state_root="$PGO_DATA_DIR/state-$label"
	rm -rf "$state_root"
	mkdir -p "$state_root/cache" "$state_root/config" "$state_root/home" "$state_root/state"

	for pass in 1 2 3; do
		echo "  train: $label pass $pass"
		env -i \
			PATH="$PATH" \
			HOME="$state_root/home" \
			HK_CACHE_DIR="$state_root/cache" \
			HK_CONFIG_DIR="$state_root/config" \
			HK_FILE="$TRAIN_CONFIG" \
			HK_STATE_DIR="$state_root/state" \
			HK_STASH=false \
			HK_STASH_UNTRACKED=false \
			LLVM_PROFILE_FILE="${LLVM_PROFILE_FILE:-}" \
			"$bin" builtins >/dev/null
		env -i \
			PATH="$PATH" \
			HOME="$state_root/home" \
			HK_CACHE_DIR="$state_root/cache" \
			HK_CONFIG_DIR="$state_root/config" \
			HK_FILE="$TRAIN_CONFIG" \
			HK_STATE_DIR="$state_root/state" \
			HK_STASH=false \
			HK_STASH_UNTRACKED=false \
			LLVM_PROFILE_FILE="${LLVM_PROFILE_FILE:-}" \
			"$bin" validate --quiet
		env -i \
			PATH="$PATH" \
			HOME="$state_root/home" \
			HK_CACHE_DIR="$state_root/cache" \
			HK_CONFIG_DIR="$state_root/config" \
			HK_FILE="$TRAIN_CONFIG" \
			HK_STATE_DIR="$state_root/state" \
			HK_STASH=false \
			HK_STASH_UNTRACKED=false \
			LLVM_PROFILE_FILE="${LLVM_PROFILE_FILE:-}" \
			"$bin" check --all --quiet
	done
}

echo ">>> [2/4] Training the PGO-instrumented binary"
export LLVM_PROFILE_FILE="$PGO_PROFRAW_DIR/hk-%m-%p.profraw"
train "$FINAL_BIN" pgo
unset LLVM_PROFILE_FILE

profraw_count="$(find "$PGO_PROFRAW_DIR" -maxdepth 1 -name '*.profraw' -type f | wc -l | tr -d ' ')"
if [ "$profraw_count" -eq 0 ]; then
	echo "ERROR: PGO training produced no .profraw files" >&2
	exit 1
fi

echo ">>> [3/4] Merging PGO data and rebuilding"
"$LLVM_PROFDATA" merge -o "$PGO_MERGED" "$PGO_PROFRAW_DIR"
if [ ! -s "$PGO_MERGED" ]; then
	echo "ERROR: llvm-profdata did not produce $PGO_MERGED" >&2
	exit 1
fi

phase3_flags="-Cprofile-use=$PGO_MERGED -Cllvm-args=-pgo-warn-missing-function=false"
if [ "$PGO_BOLT" != "0" ]; then
	phase3_flags="$phase3_flags -Clink-arg=-Wl,--emit-relocs -Clink-arg=-Wl,-q"
fi
build "$phase3_flags"

if [ "$PGO_BOLT" = "0" ]; then
	echo ">>> PGO build complete: $FINAL_BIN"
	exit 0
fi

echo ">>> [4/4] Instrumenting, training, and applying BOLT"
"$LLVM_BOLT" "$FINAL_BIN" \
	--instrument \
	--instrumentation-file="$BOLT_FDATA_PREFIX" \
	--instrumentation-file-append-pid \
	-o "$BOLT_INSTR_BIN"

train "$BOLT_INSTR_BIN" bolt

fdata_files=("$PGO_DATA_DIR"/bolt.*.fdata)
if [ ! -e "${fdata_files[0]}" ]; then
	echo "ERROR: BOLT training produced no fdata files" >&2
	exit 1
fi
"$MERGE_FDATA" "${fdata_files[@]}" -o "$BOLT_FDATA"

"$LLVM_BOLT" "$FINAL_BIN" \
	-o "$FINAL_BIN.bolt" \
	-data="$BOLT_FDATA" \
	-reorder-blocks=ext-tsp \
	-reorder-functions=cdsort \
	-split-functions \
	-split-all-cold \
	-split-eh \
	-use-gnu-stack
mv -f "$FINAL_BIN.bolt" "$FINAL_BIN"
strip --strip-all "$FINAL_BIN"

# Release-pipeline smoke checks use both configuration-free and representative
# project paths after the final rewrite and strip.
"$FINAL_BIN" builtins >/dev/null
"$FINAL_BIN" usage >/dev/null
HK_FILE="$TRAIN_CONFIG" HK_STASH=false "$FINAL_BIN" validate --quiet
HK_FILE="$TRAIN_CONFIG" HK_STASH=false "$FINAL_BIN" check --all --quiet

echo ">>> PGO+BOLT build complete: $FINAL_BIN"
ls -lh "$FINAL_BIN"
