# Backend changes in the remake

Branch: `remake/album-id-and-metadata`. Upstream reference: `../navidrome`.

## 1. Album identification

### Unicode-aware identity normalization — `utils/str/identity.go`
`NormalizeForIdentity(value)` applies: NFC normalization → `Clear` (curly quotes/dashes → ASCII)
→ removal of ZWSP/ZWNJ/ZWJ/BOM and NBSP normalization → accent folding → lowercase → whitespace
collapsing. `IdentityEquals` and `TruncateRunesSafe` are helpers.

This is intentionally separate from `str.Clear`, which other code (sort fields) relies on.

**Opt-in only.** Normalization is exposed as new PID attributes (`albumartistid_norm`, `album_norm`,
`title_norm`) used by `EnhancedAlbumPID` / `EnhancedTrackPID`. The legacy attributes keep their
exact old computation, so pointing the remake at an existing Navidrome database does not invalidate
stored IDs or trigger a surprise full rescan.

### Opt-in enhanced album PID — `consts.EnhancedAlbumPID`
```
musicbrainz_releasegroupid | musicbrainz_albumid |
albumartist_mbid,albumartistid_norm,album_norm,albumversion,releasedate |
folder,album_norm,albumartistid_norm
```

Matching opt-in track spec: `EnhancedTrackPID =
"musicbrainz_trackid|albumid,discnumber,tracknumber,title_norm"`.

`computePID` keeps its "first non-empty field wins" semantics. The new order means:
1. A release-group MBID wins outright — releases of the same album stay grouped even when their
   individual release MBIDs differ.
2. Otherwise a release MBID wins (legacy behavior).
3. Otherwise the album artist's MusicBrainz ID is considered alongside normalized names, so an
   artist rename or spelling difference no longer splits an album.
4. Untagged files fall back to `(folder, album, albumartistid)` rather than hashing an empty
   string, so two unlabeled albums in different folders remain distinct.

New attributes `mbz_releasegroupid` and `albumartist_mbid` are recognized by `getPIDAttr` and
`ValidatePIDSpec`. The existing `DefaultAlbumPID` remains the default for backward compatibility;
set `PID.Album` (or a library's `pid_album`) to `consts.EnhancedAlbumPID` to adopt it. Changing a
PID spec triggers a full rescan and Navidrome's `albumIDMap` preserves annotations.

`Metadata.AlbumIDEvidence` was added for diagnostics/reporting which identity source was used.

### Various Artists normalization — `model/metadata/map_participants.go`
`isVariousArtists` recognizes the canonical MusicBrainz ID, `Various Artists` (case/accent
insensitive), `Various Artist` and `VA`. These are now normalized to the canonical entity even
when the compilation flag is absent, preventing compilation albums from splitting per track.

## 2. Metadata parsing robustness

### `adapters/gotaglib/gotaglib.go`
- `parseTuple` now understands `2/10`, `2 / 10`, `2 of 10` and `2-10` via a strict numeric regex.
- Malformed values are trimmed and kept as-is instead of producing an empty number.
- An explicit, valid `tracktotal`/`disctotal` is never overwritten by a malformed embedded total.
- Multiple tag values are handled.
- Added `isDigits` / `firstTag` helpers.

### `model/metadata/metadata.go`
- `Bool` accepts `yes`, `y`, `on`, `t`, `1` (case-insensitive) in addition to `strconv.ParseBool`.
- `Metadata.tuple` uses the same strict number/total parser.
- `filterDuplicatedOrEmptyValues` trims values and deduplicates case-insensitively while keeping
  the first display spelling.
- `parseDate` accepts dot/slash-separated dates, ISO timestamps, validates calendar values and
  truncates with runes (not bytes), falling back to the year when the full date is invalid.

## 3. Tests

New/updated test files:
- `utils/str/identity_test.go`
- `model/metadata/enhanced_ids_test.go`
- `model/metadata/parsing_robustness_test.go`
- `model/metadata/persistent_ids_test.go` (normalized-title expectation)
- `model/metadata/metadata_test.go` (dot-separated dates now parse fully)
- `adapters/gotaglib/tuple_robustness_test.go`

Verification:
```
go build -tags=netgo,sqlite_fts5 ./...
go test -count=1 -tags=netgo,sqlite_fts5 ./...
```
All packages pass, including `model`, `model/metadata`, `adapters/gotaglib`, `scanner`,
`persistence`, `server/subsonic` and `server/subsonic/e2e`.

## Known limitations / follow-up work

The upstream analysis identified further improvements that were intentionally not included in
this pass because they change public contracts or require schema work:

- Change the `local.Extractor.Parse` contract to return per-file errors (currently failed files
  are logged and skipped).
- Add raw-tag extraction for Vorbis/FLAC/Ogg/Opus and case-insensitive iTunes atom prefixes.
- Explicit audio-extension allowlist to avoid reliance on the host MIME database.
- Deterministic conflict resolution + diagnostics columns in `album` for disagreeing tags.
- Switch the default `DefaultAlbumPID` to the enhanced spec in a future major version.
