package str

import "testing"

func TestNormalizeArtistName(t *testing.T) {
	tests := []struct{ in, want string }{
		{"Camellia", "camellia"},
		{"  CAMELLIA  ", "camellia"},
		{"Måneskin", "maneskin"},
		{"Maneskin.", "maneskin"},
		{"Camellia as \"Bang Riot\"", "camellia"},
		{"Camellia as Reverse of Riot", "camellia"},
		{"虎視虎子 (CV.藤田 咲)", "虎視虎子"},
		{"Björk", "bjork"},
		{"Bjo\u0308rk", "bjork"},
	}
	for _, tt := range tests {
		if got := NormalizeArtistName(tt.in); got != tt.want {
			t.Errorf("NormalizeArtistName(%q) = %q, want %q", tt.in, got, tt.want)
		}
	}
	if !ArtistNamesEqual("Camellia as \"Bang Riot\"", "CAMELLIA.") {
		t.Fatal("alias and case variants must map to one artist")
	}
}
