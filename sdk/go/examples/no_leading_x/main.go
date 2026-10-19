// A rule that flags each line that starts with `x`. Build it with `go build` and name the binary
// in the `command` of an extension rule.
package main

import (
	"log"
	"strings"

	botica "github.com/collectiveai-team/botica/sdk/go"
)

func main() {
	rules := []botica.Rule{{
		ID:        "acme/no-leading-x",
		Languages: []string{"python"},
		Severity:  "error",
		Check: func(path, text string, options map[string]any) []botica.Finding {
			var found []botica.Finding
			offset := 0
			for _, line := range strings.SplitAfter(text, "\n") {
				if strings.HasPrefix(line, "x") {
					found = append(found, botica.Finding{Start: offset, End: offset + 1, Message: "a line starts with x"})
				}
				offset += len(line)
			}
			return found
		},
	}}
	if err := botica.Serve(rules); err != nil {
		log.Fatal(err)
	}
}
