package metadata_test

import (
	"github.com/navidrome/navidrome/model"
	"github.com/navidrome/navidrome/model/metadata"
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
)

var _ = Describe("Robust metadata parsing (remake)", func() {
	newMD := func(tags map[string][]string) metadata.Metadata {
		raw := model.RawTags{}
		for k, v := range tags {
			raw[k] = v
		}
		return metadata.New("test.mp3", metadata.Info{Tags: raw})
	}

	DescribeTable("accepts real-world boolean spellings",
		func(value string, want bool) {
			md := newMD(map[string][]string{"compilation": {value}})
			Expect(md.Bool(model.TagCompilation)).To(Equal(want))
		},
		Entry("true", "true", true),
		Entry("yes", "Yes", true),
		Entry("y", "Y", true),
		Entry("on", "on", true),
		Entry("false", "false", false),
		Entry("empty", "", false),
	)

	DescribeTable("parses track numbers with totals",
		func(value string, wantNum, wantTotal int) {
			md := newMD(map[string][]string{"track": {value}})
			num, total := md.NumAndTotal(model.TagTrackNumber)
			Expect(num).To(Equal(wantNum))
			Expect(total).To(Equal(wantTotal))
		},
		Entry("slash", "2/10", 2, 10),
		Entry("of", "2 of 10", 2, 10),
		Entry("dash", "2-10", 2, 10),
		Entry("plain", "3", 3, 0),
	)

	DescribeTable("parses dates more defensively",
		func(value, want string) {
			md := newMD(map[string][]string{"releasedate": {value}})
			Expect(string(md.Date(model.TagReleaseDate))).To(Equal(want))
		},
		Entry("dots", "2014.05.21", "2014-05-21"),
		Entry("slashes", "2014/05/21", "2014-05-21"),
		Entry("ISO timestamp", "2016-12-31T12:15:00Z", "2016-12-31"),
		Entry("invalid month falls back to year", "2014-99-99", "2014"),
	)

	It("deduplicates tags ignoring case and surrounding whitespace", func() {
		md := newMD(map[string][]string{"album": {" Abbey Road ", "abbey road", "ABBEY ROAD"}})
		Expect(md.Strings(model.TagAlbum)).To(HaveLen(1))
		Expect(md.String(model.TagAlbum)).To(Equal("Abbey Road"))
	})
})
