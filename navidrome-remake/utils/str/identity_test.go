package str

import "testing"

func TestNormalizeForIdentity(t *testing.T) {
	tests := []struct {
		name  string
		input string
		want  string
	}{
		{"plain", "Abbey Road", "abbey road"},
		{"accented", "Björk", "bjork"},
		{"decomposed accent", "Bjo\u0308rk", "bjork"},
		{"curly quotes and dashes", "\u201cThe\u201d \u2014 Album", "\"the\" - album"},
		{"invisible characters", "A\u200bB\ufeff", "ab"},
		{"non-breaking space", "A\u00a0B", "a b"},
		{"non-breaking space between words", "The\u00a0Beatles", "the beatles"},
		{"extra whitespace", "  The \t Beatles  ", "the beatles"},
		{"empty", "", ""},
	}
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := NormalizeForIdentity(tt.input); got != tt.want {
				t.Fatalf("NormalizeForIdentity(%q) = %q, want %q", tt.input, got, tt.want)
			}
		})
	}
}

func TestIdentityEquals(t *testing.T) {
	if !IdentityEquals("Björk", "Bjo\u0308rk") {
		t.Fatal("precomposed and decomposed spellings must share an identity")
	}
	if !IdentityEquals("The Beatles", "  the   beatles ") {
		t.Fatal("whitespace and case differences must share an identity")
	}
	if IdentityEquals("Abbey Road", "Let It Be") {
		t.Fatal("different albums must not share an identity")
	}
}

func TestTruncateRunesSafe(t *testing.T) {
	if got := TruncateRunesSafe("Björk", 3); got != "Bjö" {
		t.Fatalf("got %q", got)
	}
	if got := TruncateRunesSafe("abc", 10); got != "abc" {
		t.Fatalf("got %q", got)
	}
	if got := TruncateRunesSafe("abc", 0); got != "" {
		t.Fatalf("got %q", got)
	}
}
