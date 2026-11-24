package str

import (
	"regexp"
	"strings"
)

var (
	// "Camellia as \"Bang Riot\"" and "Artist as Alias" are performance aliases,
	// not separate artists. Spotify treats those as the primary artist.
	artistAliasRx = regexp.MustCompile(`(?i)\s+as\s+["'“”]?[^"']+["']?\s*$`)

	// Character/voice-actor qualifiers are metadata about the artist, not part
	// of their name ("虎視虎子 (CV.藤田 咲)" -> "虎視虎子").
	artistCVQualifierRx = regexp.MustCompile(`(?i)\s*\(cv[.:]?\s*[^)]*\)\s*$`)

	artistPunctRx = regexp.MustCompile(`[.,;:!?]+$`)
)

// NormalizeArtistName produces the key used to decide whether two artist-tag
// spellings are the same artist. It is deliberately more aggressive than
// NormalizeForIdentity: it also drops performance aliases, CV qualifiers and
// trailing punctuation, so "Camellia as \"Bang Riot\"" and "Camellia" merge.
//
// The display name is never changed — only the ID hashing uses this value.
func NormalizeArtistName(name string) string {
	name = artistAliasRx.ReplaceAllString(name, "")
	name = artistCVQualifierRx.ReplaceAllString(name, "")
	name = artistPunctRx.ReplaceAllString(name, "")
	name = NormalizeForIdentity(name)
	name = strings.TrimSpace(name)
	return name
}

// ArtistNamesEqual reports whether two tag values should map to one artist.
func ArtistNamesEqual(a, b string) bool {
	return NormalizeArtistName(a) == NormalizeArtistName(b)
}
