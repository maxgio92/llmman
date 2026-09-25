#![no_main]

use libfuzzer_sys::fuzz_target;

// Fuzzes gguf::read_header (private to the parent crate; the parser behind
// gguf::read_info, which feeds `llmman show` and /api/show) via the
// fuzzing-feature-gated wrapper fuzz_check_read_gguf_info. The input is
// parsed in memory, no temp file per run. The wrapper panics whenever a
// successful parse violates the header invariants: metadata.len() above
// the declared kv count, tensor_count disagreeing with the entries read,
// a parameter_count that differs from an independent saturating
// recomputation, a quantization name outside the ggml_type table, or a
// string/array/rank past the reader's allocation bounds. See
// src/gguf.rs's assert_gguf_header_invariants doc comment, and
// read_gguf_info_oracle_holds_on_the_seed_corpus for the same oracle
// pinned against this seed corpus.
fuzz_target!(|data: &[u8]| {
    llmman::gguf::fuzz_check_read_gguf_info(data);
});
