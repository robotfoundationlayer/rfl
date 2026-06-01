// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Minimal C ABI binding for the RFL retarget engine.
//!
//! Exposes the stable `rfl-core` pipeline (`Skill::parse_yaml` ->
//! `Embodiment::parse_yaml` -> `translation::retarget` -> `canonical::to_jsonl`)
//! over a string-in / string-out C ABI — the C counterpart of the Python
//! binding's `rfl.retarget`. The API surface is intentionally tiny and value-only
//! (YAML in, JSONL out), so it stays stable regardless of how the engine's
//! internal coverage grows.
//!
//! Memory contract: every non-null `char*` returned by [`rfl_retarget`] is owned
//! by the caller and MUST be released with [`rfl_string_free`]. The static string
//! from [`rfl_spec_version`] must NOT be freed.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use rfl_core::{canonical, embodiment::Embodiment, skill_isa::Skill, translation};

/// Box a Rust `String` into an owned C string. Interior NULs (which valid JSONL
/// and error messages never contain) collapse to a short sentinel rather than
/// returning null, so the caller always gets a freeable pointer on the non-null
/// path.
fn into_c_string(s: String) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("error: output contained an interior NUL").unwrap())
        .into_raw()
}

/// Retarget a Skill ISA composition onto an embodiment descriptor.
///
/// `skill_yaml` and `descriptor_yaml` are NUL-terminated UTF-8 strings. On
/// success `*ok` is set to `true` and the return value is the canonical JSONL
/// (one `execute` message per line). On a parse/retarget failure `*ok` is set to
/// `false` and the return value is a human-readable error message. The returned
/// pointer is owned by the caller and must be freed with [`rfl_string_free`].
///
/// Returns null only if `skill_yaml` or `descriptor_yaml` is null.
///
/// # Safety
/// `skill_yaml` and `descriptor_yaml` must each be either null or a valid pointer
/// to a NUL-terminated string that stays valid for the duration of the call.
/// `ok` must be either null or a valid pointer to a writable `bool`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rfl_retarget(
    skill_yaml: *const c_char,
    descriptor_yaml: *const c_char,
    ok: *mut bool,
) -> *mut c_char {
    if skill_yaml.is_null() || descriptor_yaml.is_null() {
        return std::ptr::null_mut();
    }
    let set_ok = |v: bool| {
        if !ok.is_null() {
            // SAFETY: caller's contract guarantees `ok` is writable when non-null.
            unsafe { *ok = v };
        }
    };
    // SAFETY: non-null checked above; caller guarantees NUL-terminated validity.
    let skill_s = unsafe { CStr::from_ptr(skill_yaml) };
    let desc_s = unsafe { CStr::from_ptr(descriptor_yaml) };

    let result = (|| -> Result<String, String> {
        let s = skill_s
            .to_str()
            .map_err(|e| format!("skill not UTF-8: {e}"))?;
        let d = desc_s
            .to_str()
            .map_err(|e| format!("descriptor not UTF-8: {e}"))?;
        let skill = Skill::parse_yaml(s).map_err(|e| e.to_string())?;
        let emb = Embodiment::parse_yaml(d).map_err(|e| e.to_string())?;
        let out = translation::retarget(&skill, &emb).map_err(|e| e.to_string())?;
        Ok(canonical::to_jsonl(
            &skill.skill,
            &emb.id,
            &out.actions,
            &out.suffixes,
        ))
    })();

    match result {
        Ok(jsonl) => {
            set_ok(true);
            into_c_string(jsonl)
        }
        Err(e) => {
            set_ok(false);
            into_c_string(e)
        }
    }
}

/// Free a string returned by [`rfl_retarget`].
///
/// # Safety
/// `s` must be either null or a pointer previously returned by [`rfl_retarget`]
/// and not yet freed. Passing any other pointer, or freeing twice, is undefined
/// behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rfl_string_free(s: *mut c_char) {
    if !s.is_null() {
        // SAFETY: reclaims a CString this library allocated via into_raw.
        drop(unsafe { CString::from_raw(s) });
    }
}

/// The specification version this build implements (e.g. `v0.1-draft`).
///
/// Returns a pointer to a static NUL-terminated string; do NOT free it.
#[unsafe(no_mangle)]
pub extern "C" fn rfl_spec_version() -> *const c_char {
    c"v0.1-draft".as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn cstr(s: &str) -> CString {
        CString::new(s).unwrap()
    }

    #[test]
    fn spec_version_matches_core_and_is_freeing_safe() {
        // SAFETY: the returned pointer is a 'static C string.
        let v = unsafe { CStr::from_ptr(rfl_spec_version()) }
            .to_str()
            .unwrap();
        assert_eq!(
            v,
            rfl_core::SPEC_VERSION,
            "C spec_version drifted from core"
        );
    }

    #[test]
    fn retarget_round_trips_the_cable_example() {
        let skill = include_str!("../../../examples/01-cable-insertion/skill.yaml");
        let desc = include_str!("../../../examples/01-cable-insertion/embodiments/allegro.yaml");
        let skill_c = cstr(skill);
        let desc_c = cstr(desc);
        let mut ok = false;
        // SAFETY: both pointers are valid NUL-terminated strings; ok is writable.
        let out = unsafe { rfl_retarget(skill_c.as_ptr(), desc_c.as_ptr(), &mut ok) };
        assert!(!out.is_null());
        assert!(ok, "valid inputs should succeed");
        // SAFETY: out is the just-returned owned string.
        let jsonl = unsafe { CStr::from_ptr(out) }.to_str().unwrap().to_owned();
        // Identical to the engine's own output (the binding adds no behavior).
        let s = Skill::parse_yaml(skill).unwrap();
        let e = Embodiment::parse_yaml(desc).unwrap();
        let r = translation::retarget(&s, &e).unwrap();
        let expected = canonical::to_jsonl(&s.skill, &e.id, &r.actions, &r.suffixes);
        assert_eq!(jsonl, expected);
        assert!(jsonl.contains("\"message\":\"execute\""));
        // SAFETY: free the owned string exactly once.
        unsafe { rfl_string_free(out) };
    }

    #[test]
    fn invalid_skill_reports_error_not_crash() {
        let bad = cstr("not: a valid skill");
        let desc = cstr(include_str!(
            "../../../examples/01-cable-insertion/embodiments/allegro.yaml"
        ));
        let mut ok = true;
        // SAFETY: valid pointers; ok writable.
        let out = unsafe { rfl_retarget(bad.as_ptr(), desc.as_ptr(), &mut ok) };
        assert!(!out.is_null());
        assert!(!ok, "an invalid skill must set ok=false");
        // SAFETY: free the owned error string.
        unsafe { rfl_string_free(out) };
    }

    #[test]
    fn null_input_returns_null() {
        let mut ok = true;
        // SAFETY: passing null is explicitly handled.
        let out = unsafe { rfl_retarget(std::ptr::null(), std::ptr::null(), &mut ok) };
        assert!(out.is_null());
    }
}
