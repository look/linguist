package linguist

/*
#cgo LDFLAGS: -llinguist
#cgo CFLAGS: -I${SRCDIR}/../../include

#include "blackbird_linguist.h"
*/
import "C"

import (
	"fmt"
	"math"
	"runtime"
	"unsafe"

	// This is what makes sure that blackbird_linguist.h is included in the go module when using `go mod vendor`.
	_ "github.com/github/blackbird/crates/linguist/include"
)

func IsIndexable(path string, content []byte) (bool, string, error) {
	cPath := C.CString(path)
	defer C.free(unsafe.Pointer(cPath))

	var cContentPtr *C.uchar
	if len(content) > 0 {
		// Don't let the Go GC move content while it's memory is shared across the ffi boundary.
		contentPinner := runtime.Pinner{}
		contentPinner.Pin(&content[0])
		defer contentPinner.Unpin()

		cContentPtr = (*C.uchar)(unsafe.Pointer(&content[0]))
	}

	var is_indexable C.bool
	var reasonPtr *C.char
	code := C.linguist_is_indexable(cPath, cContentPtr, C.size_t(len(content)), &is_indexable, &reasonPtr)
	if code != C.LinguistError_None {
		return false, "", fmt.Errorf("failed to check is_indexable: %w", LinguistError{code})
	}

	if reasonPtr != nil {
		defer C.linguist_cstring_free(reasonPtr) // NOTE: Must free AFTER checking code an only if a string was set
	}

	return bool(is_indexable), C.GoString(reasonPtr), nil
}

func GetLanguageName(id uint32) (string, error) {
	var ptr *C.char
	var len C.size_t
	code := C.get_language_name(C.uint32_t(id), &ptr, &len)
	if code != C.LinguistError_None {
		return "", fmt.Errorf("failed to get language name: %w", LinguistError{code})
	}
	name := C.GoStringN(ptr, C.int(len))
	return name, nil
}

// GetLanguageColor returns the color for the given language id. If no color is
// defined, an empty string is returned.
func GetLanguageColor(id uint32) (string, error) {
	var ptr *C.char
	var len C.size_t
	code := C.get_language_color(C.uint32_t(id), &ptr, &len)
	if code != C.LinguistError_None {
		return "", fmt.Errorf("failed to get language color: %w", LinguistError{code})
	}
	color := C.GoStringN(ptr, C.int(len))
	return color, nil
}

// GetLanguageTMScope returns the TextMate scope for the given language id.
// Linguist defines "none" as the TextMate scope for languages that do not have
// one.
func GetLanguageTMScope(id uint32) (string, error) {
	var ptr *C.char
	var len C.size_t
	code := C.get_language_tm_scope(C.uint32_t(id), &ptr, &len)
	if code != C.LinguistError_None {
		return "", fmt.Errorf("failed to get language tm_scope: %w", LinguistError{code})
	}
	scope := C.GoStringN(ptr, C.int(len))
	return scope, nil
}

// GetLanguageByAlias returns the language ID for the given alias, if there is one.
func GetLanguageByAlias(alias string) (uint32, error) {
	cAlias := C.CString(alias)
	defer C.free(unsafe.Pointer(cAlias))
	var out C.uint32_t

	code := C.get_language_by_alias(cAlias, &out)
	if code != C.LinguistError_None {
		return math.MaxUint32, fmt.Errorf("failed to get language ID for alias: %w", LinguistError{code})
	}
	return uint32(out), nil
}

// Wrapper for errors returned from Linguist
type LinguistError struct {
	// CErr is the C error code (see the LinguistError enum).
	CErr C.enum_LinguistError
}

func (e LinguistError) Error() string {
	var msg string
	switch e.CErr {
	case C.LinguistError_None:
		msg = ""
	case C.LinguistError_NotFound:
		msg = "Unknown language"
	case C.LinguistError_Panic:
		msg = "Panic"
	case C.LinguistError_IllegalState:
		msg = "Illegal state"
	default:
		msg = "Unknown error"
	}
	return fmt.Sprintf("%s, code=%d", msg, e.CErr)
}
