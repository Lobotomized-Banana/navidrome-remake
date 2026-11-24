package metadata

import (
	"strings"
	"testing"

	"github.com/navidrome/navidrome/consts"
	"github.com/navidrome/navidrome/model"
	"github.com/navidrome/navidrome/tests"
)

// testHash keeps the PID input readable so identity tests assert on the actual
// attributes that were selected, not on an opaque digest.
func testHash(values ...string) string { return strings.Join(values, "\\") }

func TestIdentityNormalizationStabilizesAlbumID(t *testing.T) {
	tests.Init(t, true)
	md1 := Metadata{tags: map[model.TagName][]string{model.TagAlbum: {"Début"}}}
	mf1 := model.MediaFile{AlbumArtist: "Björk", Title: "Debut"}
	md2 := Metadata{tags: map[model.TagName][]string{model.TagAlbum: {"Debut"}}}
	mf2 := model.MediaFile{AlbumArtist: "Bjo\u0308rk", Title: "Debut"}

	id1 := computePID(mf1, md1, consts.DefaultAlbumPID, consts.DefaultAlbumPID, false, testHash)
	id2 := computePID(mf2, md2, consts.DefaultAlbumPID, consts.DefaultAlbumPID, false, testHash)

	if id1 != id2 {
		t.Fatalf("accented/decomposed names must share an album ID: %q != %q", id1, id2)
	}
	if !strings.Contains(id1, "bjork\\debut") {
		t.Fatalf("expected normalized attributes in PID, got %q", id1)
	}
}

func TestEnhancedAlbumPIDUsesReleaseGroupBeforeReleaseID(t *testing.T) {
	tests.Init(t, true)
	md1 := Metadata{tags: map[model.TagName][]string{
		model.TagMusicBrainzAlbumID:        {"release-1"},
		model.TagMusicBrainzReleaseGroupID: {"group-1"},
	}}
	md2 := Metadata{tags: map[model.TagName][]string{
		model.TagMusicBrainzAlbumID:        {"release-2"},
		model.TagMusicBrainzReleaseGroupID: {"group-1"},
	}}

	id1 := computePID(model.MediaFile{}, md1, consts.EnhancedAlbumPID, consts.EnhancedAlbumPID, false, testHash)
	id2 := computePID(model.MediaFile{}, md2, consts.EnhancedAlbumPID, consts.EnhancedAlbumPID, false, testHash)

	if id1 != id2 {
		t.Fatalf("releases in one release group must share an album ID: %q != %q", id1, id2)
	}
	if id1 != "group-1" {
		t.Fatalf("release group MBID should be the sole lead attribute, got %q", id1)
	}
}

func TestEnhancedAlbumPIDUsesAlbumArtistMBID(t *testing.T) {
	tests.Init(t, true)
	mk := func(mbid string) model.MediaFile {
		return model.MediaFile{
			AlbumArtist: "The Artist",
			Participants: model.Participants{
				model.RoleAlbumArtist: []model.Participant{{Artist: model.Artist{Name: "The Artist", MbzArtistID: mbid}}},
			},
		}
	}
	md := Metadata{tags: map[model.TagName][]string{model.TagAlbum: {"Album"}}}

	legacy1 := computePID(mk("mbid-a"), md, consts.DefaultAlbumPID, consts.DefaultAlbumPID, false, testHash)
	legacy2 := computePID(mk("mbid-b"), md, consts.DefaultAlbumPID, consts.DefaultAlbumPID, false, testHash)
	if legacy1 != legacy2 {
		t.Fatal("legacy spec is not expected to distinguish album artist MBIDs")
	}

	enhanced1 := computePID(mk("mbid-a"), md, consts.EnhancedAlbumPID, consts.EnhancedAlbumPID, false, testHash)
	enhanced2 := computePID(mk("mbid-b"), md, consts.EnhancedAlbumPID, consts.EnhancedAlbumPID, false, testHash)
	if enhanced1 == enhanced2 {
		t.Fatalf("enhanced spec must distinguish album artist MBIDs: %q", enhanced1)
	}
}

func TestEnhancedAlbumPIDFallsBackToFolder(t *testing.T) {
	tests.Init(t, true)
	md := Metadata{tags: map[model.TagName][]string{}}
	legacy1 := computePID(model.MediaFile{Path: "/library/a/01.mp3"}, md, consts.DefaultAlbumPID, consts.DefaultAlbumPID, false, testHash)
	legacy2 := computePID(model.MediaFile{Path: "/library/b/01.mp3"}, md, consts.DefaultAlbumPID, consts.DefaultAlbumPID, false, testHash)
	if legacy1 != legacy2 {
		t.Fatal("legacy spec hashes empty tags to the same value")
	}

	enhanced1 := computePID(model.MediaFile{Path: "/library/a/01.mp3"}, md, consts.EnhancedAlbumPID, consts.EnhancedAlbumPID, false, testHash)
	enhanced2 := computePID(model.MediaFile{Path: "/library/b/01.mp3"}, md, consts.EnhancedAlbumPID, consts.EnhancedAlbumPID, false, testHash)
	if enhanced1 == enhanced2 {
		t.Fatalf("untagged files in different folders must not merge: %q", enhanced1)
	}
	if !strings.Contains(enhanced1, "/library/a") {
		t.Fatalf("folder should be part of the fallback identity, got %q", enhanced1)
	}
}

func TestEnhancedAlbumPIDSpecValidates(t *testing.T) {
	tests.Init(t, true)
	if err := ValidatePIDSpec(consts.EnhancedAlbumPID, true); err != nil {
		t.Fatalf("enhanced album PID must be a valid spec: %v", err)
	}
}
