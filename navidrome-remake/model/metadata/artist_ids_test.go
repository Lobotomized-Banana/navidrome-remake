package metadata

import (
	"testing"

	"github.com/navidrome/navidrome/model"
	"github.com/navidrome/navidrome/tests"
)

func TestArtistIDsMergeCloseVariants(t *testing.T) {
	tests.Init(t, true)
	md := Metadata{}
	variants := []string{
		"Camellia", "CAMELLIA", "Camellia as \"Bang Riot\"", "Camellia.",
		"Camellia  ", "Camellia as Reverse of Riot",
	}
	want := md.artistID("Camellia")
	for _, name := range variants {
		if got := md.artistID(name); got != want {
			t.Errorf("artist ID for %q should merge with Camellia: %s != %s", name, got, want)
		}
	}
}

func TestCollaborationSeparators(t *testing.T) {
	tests.Init(t, true)
	md := Metadata{tags: map[model.TagName][]string{
		model.TagTrackArtist: {"Camellia vs PHQUASE"},
	}}
	p := md.mapParticipants()
	artists := p[model.RoleArtist]
	if len(artists) != 2 {
		t.Fatalf("expected two artists for a collaboration, got %d: %+v", len(artists), artists)
	}
	names := map[string]bool{}
	for _, a := range artists {
		names[a.Name] = true
	}
	if !names["Camellia"] || !names["PHQUASE"] {
		t.Fatalf("unexpected split: %+v", names)
	}
}
