package gotaglib

import (
	. "github.com/onsi/ginkgo/v2"
	. "github.com/onsi/gomega"
)

var _ = Describe("parseTuple (robust parsing)", func() {
	DescribeTable("parses number/total spellings",
		func(input, wantNum string, wantTotal string, totalPresent bool) {
			tags := map[string][]string{"tracknumber": {input}}
			parseTuple(tags, "track")
			Expect(tags["tracknumber"][0]).To(Equal(wantNum))
			if totalPresent {
				Expect(tags["tracktotal"][0]).To(Equal(wantTotal))
			} else {
				Expect(tags).ToNot(HaveKey("tracktotal"))
			}
		},
		Entry("slash", "2/10", "2", "10", true),
		Entry("spaced slash", "2 / 10", "2", "10", true),
		Entry("of", "2 of 10", "2", "10", true),
		Entry("dash", "2-10", "2", "10", true),
		Entry("junk is kept but does not create a total", "side A", "side A", "", false),
		Entry("three-part value is left alone", "1/10/2", "1/10/2", "", false),
	)

	It("does not overwrite a valid explicit total", func() {
		tags := map[string][]string{"tracknumber": {"2/10"}, "tracktotal": {"12"}}
		parseTuple(tags, "track")
		Expect(tags["tracknumber"][0]).To(Equal("2"))
		Expect(tags["tracktotal"][0]).To(Equal("12"))
	})

	It("replaces an invalid explicit total", func() {
		tags := map[string][]string{"tracknumber": {"2/10"}, "tracktotal": {"many"}}
		parseTuple(tags, "track")
		Expect(tags["tracktotal"][0]).To(Equal("10"))
	})
})
