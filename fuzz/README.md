# Fuzzing

This crate holds cargo-fuzz targets for the parsers that read untrusted
bytes: src/shortnames.rs's model-reference parsing (parse_registry_ref,
validate_reference, resolve_ollama_api), src/gguf.rs's GGUF header reader
(read_gguf_info, the parser behind `llmman show` and /api/show), and the
serde types for the OCI JSON documents the stores read back from disk
(parse_oci_json: storage::oci Descriptor, Manifest, CncfModelConfig and
hf::oci Descriptor, Manifest). Each target's oracle lives next to the code
it fuzzes, behind the non-default fuzzing Cargo feature (fuzz_check_* in
src/shortnames.rs, src/gguf.rs, src/storage/oci.rs). A signature change
that breaks a target shows up as a compile error, not a silent gap.

| Target | Input | Oracle on a successful parse |
| --- | --- | --- |
| parse_registry_ref | UTF-8 reference | every field holds its own grammar |
| validate_reference | UTF-8 reference | accept implies parse_registry_ref accepts, with the same grammar |
| resolve_ollama_api | UTF-8 reference | Ok/Err agrees with validate_reference |
| read_gguf_info | raw GGUF bytes, parsed in memory | metadata.len() <= declared kv count, tensor_count matches entries read, parameter_count equals a saturating recomputation, quantization names a ggml_type, string (decoded, 3 * 64 MiB after lossy UTF-8), array and rank bounds hold |
| parse_oci_json | raw JSON bytes | second-generation round trip is stable; digest passes split_digest iff it has a `:`; ref.name is default_tag-idempotent, keeps its repo_name, and a digest-free one matches its own descriptor |

read_gguf_info asserts `<=` on the metadata count, not `==`: the reader
stores metadata in a HashMap, so a file repeating a key collapses the
duplicates. parse_oci_json does not assert an `algo:hex` digest shape:
the read path only splits on the first `:` today, so a stricter oracle
would report a missing validation feature as a crash. Adding that
validation to the store read path is a separate change.

fuzzing only widens visibility of those wrapper functions. It does not
change parsing behavior. Root CI compiles this crate
(cargo check --manifest-path fuzz/Cargo.toml) and the fuzzing feature
(cargo clippy --features fuzzing --lib) on every push, but does not run the
fuzzer: that needs a nightly toolchain. .github/workflows/fuzz-nightly.yml
installs one and runs the fuzzer on a schedule, separate from main CI.

## Running locally

Needs a nightly toolchain:

```
rustup install nightly
cargo install cargo-fuzz
cargo +nightly fuzz run parse_registry_ref -- -max_total_time=300
cargo +nightly fuzz run validate_reference -- -max_total_time=300
cargo +nightly fuzz run resolve_ollama_api -- -max_total_time=300
cargo +nightly fuzz run read_gguf_info -- -max_total_time=300
cargo +nightly fuzz run parse_oci_json -- -max_total_time=300
```

Each target reads seeds from fuzz/corpus/<target>/. A unit test replays
those same seeds on every cargo test (see e.g.
parse_registry_ref_oracle_holds_on_the_seed_corpus in src/shortnames.rs),
so a regression pinned by a seed fails in normal CI too, with no fuzzer run
needed.

A crash writes a file under fuzz/artifacts/<target>/. Minimize it and copy
it into fuzz/corpus/<target>/ under a descriptive name before fixing the
bug, so the regression stays pinned:

```
cargo +nightly fuzz tmin parse_registry_ref fuzz/artifacts/parse_registry_ref/<crash-file>
cp <minimized-file> fuzz/corpus/parse_registry_ref/<descriptive-name>
```

fuzz/.gitignore tracks only the hand-curated corpus directories: corpus/*
is ignored by default, with one `!corpus/<target>/` exception per target.
A new target's corpus directory needs its own exception line added there,
or git add silently skips its seeds.

## Lockfile drift

fuzz/Cargo.lock is a separate lockfile from the root Cargo.lock: cargo-fuzz
makes the fuzz crate a standalone workspace member on purpose, so a change
to its dependency tree can't affect the root build. This repo has no
Dependabot config at all (no .github/dependabot.yml), so a Dependabot entry
scoped only to fuzz/ would be inconsistent, not a fix. Until this repo
adopts Dependabot generally, bump both lockfiles by hand together and land
both diffs in the same commit:

1. `cargo update` at the root, as usual.
2. `cp Cargo.lock fuzz/Cargo.lock`, so the fuzz crate resolves the shared
   dependency tree to the same versions as the root instead of running an
   independent `cargo update`.
3. `cargo check --manifest-path fuzz/Cargo.toml` once, without `--locked`:
   Cargo adds the entries the root lockfile lacks (libfuzzer-sys and its
   tree, plus the llmman-fuzz package itself) and leaves everything else
   as copied.
4. `cargo check --manifest-path fuzz/Cargo.toml --locked` to confirm the
   result is stable.

CI runs step 4. `--locked` fails whenever Cargo would need to rewrite
fuzz/Cargo.lock: a dependency changed in fuzz/Cargo.toml, or the root
crate's version (packaging/version.sh stamps it into both lockfiles) not
matching the `llmman` entry. It does not compare fuzz/Cargo.lock's pins
against the root Cargo.lock, so skipping step 2 leaves the two lockfiles
resolving shared dependencies differently without any CI failure; the
copy is what keeps them aligned.
