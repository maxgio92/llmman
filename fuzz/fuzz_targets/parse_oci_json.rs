#![no_main]

use libfuzzer_sys::fuzz_target;

// Fuzzes the serde parsers for every OCI JSON document the stores read
// from disk (storage::oci::{Descriptor, Manifest, CncfModelConfig} and
// hf::oci::{Descriptor, Manifest}) via the fuzzing-feature-gated wrapper
// fuzz_check_parse_oci_json. Non-UTF-8 input is fed through as well:
// serde_json rejects it and that rejection is part of the surface. The
// wrapper panics whenever a successful parse is not round-trip stable
// (to_value(parse(x)) != to_value(parse(to_string(parse(x))))), or a
// parsed digest / ref.name annotation disagrees with the split_digest,
// split_ref_digest, repo_name, tag_from_ref and default_tag helpers the
// store applies to it. See src/storage/oci.rs's
// assert_oci_json_invariants doc comment, and
// parse_oci_json_oracle_holds_on_the_seed_corpus for the same oracle
// pinned against this seed corpus.
fuzz_target!(|data: &[u8]| {
    llmman::storage::oci::fuzz_check_parse_oci_json(data);
});
