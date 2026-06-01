# rfl-c — C ABI binding

A minimal C binding for the RFL retarget engine: YAML in, canonical JSONL out.
It wraps the same stable `rfl-core` pipeline as the CLI and the Python binding
(`Skill::parse_yaml` → `Embodiment::parse_yaml` → `translation::retarget` →
`canonical::to_jsonl`), exposed over a tiny value-only C ABI that stays stable
regardless of how the engine's internal coverage grows.

## API

The header [`include/rfl.h`](include/rfl.h) is generated from `src/lib.rs` by
cbindgen at build time (`cargo build -p rfl-c`). Three functions:

```c
/* Retarget skill_yaml onto descriptor_yaml. Returns owned JSONL on success
 * (*ok = true) or an owned error message (*ok = false); free it with
 * rfl_string_free. Returns NULL only if an input pointer is NULL. */
char       *rfl_retarget(const char *skill_yaml, const char *descriptor_yaml, bool *ok);

/* Free a string returned by rfl_retarget. */
void        rfl_string_free(char *s);

/* The spec version this build implements (static; do NOT free). */
const char *rfl_spec_version(void);
```

## Building

```bash
cargo build -p rfl-c --release
# -> target/release/librfl.a (static) + librfl.{so,dylib,dll} (shared)
# -> bindings/c/include/rfl.h (generated)
```

## Using it from C

```c
#include <stdio.h>
#include <stdlib.h>
#include "rfl.h"

int main(void) {
    const char *skill = /* ... Skill ISA YAML ... */;
    const char *desc  = /* ... embodiment descriptor YAML ... */;
    bool ok = false;
    char *out = rfl_retarget(skill, desc, &ok);
    if (ok) fputs(out, stdout);        /* JSONL, one execute goal per line */
    else    fprintf(stderr, "%s\n", out);
    rfl_string_free(out);
    return ok ? 0 : 1;
}
```

```bash
cc demo.c -I bindings/c/include -L target/release -lrfl -o demo
```

## Memory contract

Every non-null `char*` from `rfl_retarget` is owned by the caller and must be
released exactly once with `rfl_string_free`. The `rfl_spec_version` string is
static and must not be freed. The functions never unwind across the ABI
(parse/retarget failures are returned as `*ok = false` + an error string).

## Notes

- `crate-type = ["cdylib", "staticlib", "rlib"]` — shared + static for C/C++
  consumers, plus `rlib` so the workspace can unit-test the FFI directly.
- Built and tested in-workspace (the Rust `#[test]`s exercise the ABI and assert
  byte-equality with the engine output); a C compile-link-run is verified
  manually (see the snippet above).
- The Python binding lives separately in [`bindings/python`](../python) and is
  built via maturin.
