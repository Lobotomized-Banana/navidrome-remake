package str

import (
	"strings"
	"unicode/utf8"

	"github.com/deluan/sanitize"
	"golang.org/x/text/unicode/norm"
)

// identityReplacer removes invisible characters that must never influence the
// identity of an artist, album or track. Two files that differ only by a
// zero-width space or a UTF-8 BOM must hash to the same entity.
var identityReplacer = strings.NewReplacer(
	"\u200b", "", // zero-width space
	"\u200c", "", // zero-width non-joiner
	"\u200d", "", // zero-width joiner
	"\ufeff", "", // byte order mark
	"\u00a0", " ", // non-breaking space
	"\u202f", " ", // narrow non-breaking space
)

// NormalizeForIdentity canonicalizes a value before it is used to compute a
// persistent ID. It is deliberately more aggressive than Clear: it folds
// Unicode to NFC (so precomposed and decomposed spellings match), strips
// accents and invisible characters, unifies quotes/dashes, lowercases, and
// collapses whitespace.
//
// Do not use this for display or sort values: it throws away information that
// is meaningful to a human (accent marks and capitalization).
func NormalizeForIdentity(value string) string {
	if value == "" {
		return ""
	}
	value = norm.NFC.String(value)
	value = Clear(value)
	value = identityReplacer.Replace(value)
	value = sanitize.Accents(value)
	value = strings.ToLower(value)
	return strings.Join(strings.Fields(value), " ")
}

// IdentityEquals reports whether two values normalize to the same identity.
func IdentityEquals(a, b string) bool {
	return NormalizeForIdentity(a) == NormalizeForIdentity(b)
}

// TruncateRunesSafe truncates s to at most maxRunes, never leaving a partial
// UTF-8 rune behind. It is used for values that may contain arbitrary Unicode.
func TruncateRunesSafe(s string, maxRunes int) string {
	if maxRunes <= 0 {
		return ""
	}
	if utf8.RuneCountInString(s) <= maxRunes {
		return s
	}
	return string([]rune(s)[:maxRunes])
}
