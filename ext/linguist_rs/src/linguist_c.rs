use std::ffi::{CStr, CString, c_char};
use std::panic::AssertUnwindSafe;
use std::str::from_utf8;

use crate::LanguageId;

/// Check if the path and content is considered indexable for code search.
///
/// If successful the out-parameter `indexable` will be set. If `indexable` is `false`, a pointer to a new
/// null-terminated UTF-8 string is stored in the out-parameter `*reason`. The caller is responsible
/// for freeing the string by calling [`linguist_cstring_free`].
///
/// # Safety
///
/// `path` must be a valid pointer to UTF-8 encoded, null-terminated string. `content` must be a
/// valid pointer to a UTF-8 encoded string of length: `content_len`.
#[unsafe(no_mangle)] // SAFETY: there is no other global function of this name
pub unsafe extern "C" fn linguist_is_indexable(
    path: *const c_char,
    content: *const u8,
    content_len: usize,
    indexable: &mut bool,
    reason: &mut *mut c_char,
) -> LinguistError {
    std::panic::catch_unwind(AssertUnwindSafe(|| {
        let Ok(path) = unsafe { CStr::from_ptr(path) }.to_str() else {
            return LinguistError::InvalidUtf8;
        };
        let content = if content_len == 0 {
            &[]
        } else {
            unsafe { std::slice::from_raw_parts(content, content_len) }
        };
        let Ok(content) = from_utf8(content) else {
            return LinguistError::InvalidUtf8;
        };

        let skip_indexing = crate::indexable::skip_indexing(path, content);
        *indexable = skip_indexing.is_none();
        if let Some(r) = skip_indexing {
            *reason = CString::new(format!("{r:?}"))
                .expect("reason must not have NULL characters")
                .into_raw();
        }
        LinguistError::None
    }))
    .unwrap_or(LinguistError::Panic)
}

/// Free a CString returned by [`linguist_is_indexable`].
///
/// # Safety
///
/// `ptr` must be a pointer produced by a successful call to `linguist_is_indexable` and not already freed.
#[unsafe(no_mangle)] // SAFETY: there is no other global function of this name
pub unsafe extern "C" fn linguist_cstring_free(ptr: *mut c_char) -> LinguistError {
    std::panic::catch_unwind(|| {
        drop(unsafe { CString::from_raw(ptr) });
        LinguistError::None
    })
    .unwrap_or(LinguistError::Panic)
}

/// Convert a Linguist language ID to a language name, if possible.
///
/// If successful, set `language_name` and `language_name_len` with bytes
/// representing the name as a valid UTF-8 encoded string.
///
/// If the ID cannot be translated to a name, an error will be returned and the
/// out parameters will be left unmodified.
///
/// # Safety
///
/// The pointers must be valid and non-null.
#[unsafe(no_mangle)] // SAFETY: there is no other global function of this name
pub unsafe extern "C" fn get_language_name(
    id: u32,
    language_name: &mut *const c_char,
    language_name_len: &mut usize,
) -> LinguistError {
    std::panic::catch_unwind(AssertUnwindSafe(|| {
        if let Some(language) = crate::get_language_by_id(id as LanguageId) {
            // Note: this is safe because we know that the language name is owned by a struct with a 'static lifetime.
            *language_name = language.name.as_ptr() as *const c_char;
            *language_name_len = language.name.len();
            LinguistError::None
        } else {
            LinguistError::NotFound
        }
    }))
    .unwrap_or(LinguistError::Panic)
}

/// Convert a Linguist language alias to a language ID, if possible.
///
/// If successful, set `id` to the language ID.
///
/// If the alias cannot be translated to a name, an error will be returned.
///
/// # Safety
///
/// `alias` must be a valid pointer to a UTF-8 encoded, null-terminated string.
#[unsafe(no_mangle)] // SAFETY: there is no other global function of this name
pub unsafe extern "C" fn get_language_by_alias(
    alias: *const c_char,
    id: &mut u32,
) -> LinguistError {
    std::panic::catch_unwind(AssertUnwindSafe(|| {
        // The caller is responsible for keeping the string allocated until we have completed.
        let alias = unsafe {
            CStr::from_ptr(alias)
                .to_str()
                .expect("invalid UTF-8 in alias name")
        };
        if let Some(language) = crate::get_language_by_alias(alias) {
            *id = language.language_id;
            LinguistError::None
        } else {
            LinguistError::NotFound
        }
    }))
    .unwrap_or(LinguistError::Panic)
}

/// Convert a Linguist language ID to a language color, if possible.
///
/// If successful, set `language_color` and `language_color_len` with bytes
/// representing the color as a valid UTF-8 encoded string. If no color is
/// defined, `language_color` and `language_color_len` will not be modified.
///
/// If the ID cannot be translated to a color, an error will be returned.
///
/// # Safety
///
/// The pointers must be valid and non-null.
#[unsafe(no_mangle)] // SAFETY: there is no other global function of this name
pub unsafe extern "C" fn get_language_color(
    id: u32,
    language_color: &mut *const c_char,
    language_color_len: &mut usize,
) -> LinguistError {
    std::panic::catch_unwind(AssertUnwindSafe(|| {
        if let Some(language) = crate::get_language_by_id(id as LanguageId) {
            if let Some(color) = &language.color {
                // Note: this is safe because we know that the color is owned by a struct with a 'static lifetime.
                *language_color = color.as_ptr() as *const c_char;
                *language_color_len = color.len();
            }
            LinguistError::None
        } else {
            LinguistError::NotFound
        }
    }))
    .unwrap_or(LinguistError::Panic)
}

/// Convert a Linguist language ID to a TextMate scope, if possible.
///
/// If successful, set `language_tm_scope` and `language_tm_scope_len` with
/// bytes representing the TextMate scope as a valid UTF-8 encoded string. If
/// unsuccessful, the pointers will not be modified.
///
/// Note that Linguist's data files define `"none"` as the `tm_scope` value when
/// none is defined for a language.
///
/// If the ID cannot be translated to a TextMate scope, an error will be
/// returned.
///
/// # Safety
///
/// The pointers must be valid and non-null.
#[unsafe(no_mangle)] // SAFETY: there is no other global function of this name
pub unsafe extern "C" fn get_language_tm_scope(
    id: u32,
    language_tm_scope: &mut *const c_char,
    language_tm_scope_len: &mut usize,
) -> LinguistError {
    std::panic::catch_unwind(AssertUnwindSafe(|| {
        if let Some(language) = crate::get_language_by_id(id as LanguageId) {
            // Note: this is safe because we know that the color is owned by a struct with a 'static lifetime.
            *language_tm_scope = language.tm_scope.as_ptr() as *const c_char;
            *language_tm_scope_len = language.tm_scope.len();
            LinguistError::None
        } else {
            LinguistError::NotFound
        }
    }))
    .unwrap_or(LinguistError::Panic)
}

/// Potential errors returned from the Linguist C-API.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinguistError {
    None,
    NotFound,
    Panic,
    IllegalState,
    InvalidUtf8,
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;

    use super::*;

    #[test]
    fn test_is_indexable() {
        let path = CString::new("foo/bar.rs").unwrap();
        let content = "fn main() {}";
        let mut indexable = false;
        let mut reason = std::ptr::null_mut();
        let res = unsafe {
            linguist_is_indexable(
                path.as_ptr(),
                content.as_ptr(),
                content.len(),
                &mut indexable,
                &mut reason,
            )
        };
        assert_eq!(res, LinguistError::None);
        assert!(indexable)
    }

    #[test]
    fn test_skip_indexing() {
        let path = CString::new(".gitignore").unwrap();
        let content = "";
        let mut indexable = true;
        let mut reason = std::ptr::null_mut();
        let res = unsafe {
            linguist_is_indexable(
                path.as_ptr(),
                content.as_ptr(),
                content.len(),
                &mut indexable,
                &mut reason,
            )
        };
        assert_eq!(res, LinguistError::None);
        assert!(!indexable);
        let reason = unsafe { CString::from_raw(reason) };
        assert_eq!("FilePath(GitIgnore)", reason.to_str().unwrap())
        // NB: Can't free here because CString::from_raw takes ownership.
    }

    #[test]
    fn test_skip_indexing_free() {
        let path = CString::new(".gitignore").unwrap();
        let content = "";
        let mut indexable = true;
        let mut reason = std::ptr::null_mut();
        let res = unsafe {
            linguist_is_indexable(
                path.as_ptr(),
                content.as_ptr(),
                content.len(),
                &mut indexable,
                &mut reason,
            )
        };
        assert_eq!(res, LinguistError::None);
        assert!(!indexable);

        // Make sure we can free the string
        let res = unsafe { linguist_cstring_free(reason) };
        assert_eq!(res, LinguistError::None);
    }
}
