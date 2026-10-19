package botica_test

import (
	"encoding/json"
	"fmt"
	"reflect"
	"strings"
	"testing"

	botica "github.com/collectiveai-team/botica/sdk/go"
)

func rules() []botica.Rule {
	return []botica.Rule{
		{
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
		},
		{
			ID:        "acme/js-anything",
			Languages: []string{"javascript"},
			Check: func(path, text string, options map[string]any) []botica.Finding {
				return []botica.Finding{{Start: 0, End: 1, Message: "js"}}
			},
		},
		{
			ID:        "acme/ts-options",
			Languages: []string{"typescript"},
			Check: func(path, text string, options map[string]any) []botica.Finding {
				encoded, _ := json.Marshal(options)
				return []botica.Finding{{Start: 0, End: 1, Message: string(encoded)}}
			},
		},
		{
			ID:        "acme/project-files",
			Languages: []string{"python"},
			Severity:  "info",
			Scope:     "project",
			CheckProject: func(root string, paths []string, options map[string]any) []botica.Finding {
				encoded, _ := json.Marshal(options)
				var found []botica.Finding
				for _, path := range paths {
					found = append(found, botica.Finding{Path: path, Start: 0, End: 1, Message: fmt.Sprintf("%s %s", root, encoded)})
				}
				return append(found, botica.Finding{Start: 0, End: 1, Message: "a finding with no path"})
			},
		},
	}
}

const initialize = `{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":1,"client":"botica"}}`
const shutdown = `{"jsonrpc":"2.0","id":3,"method":"shutdown"}`

// replies are the lines that the SDK writes when it is given the requests, one on each line.
func replies(t *testing.T, requests ...string) []any {
	t.Helper()
	var out strings.Builder
	if err := botica.ServeOn(rules(), strings.NewReader(strings.Join(requests, "\n")+"\n"), &out); err != nil {
		t.Fatalf("the SDK serves: %v", err)
	}
	var lines []any
	for _, line := range strings.Split(strings.TrimSuffix(out.String(), "\n"), "\n") {
		var value any
		if err := json.Unmarshal([]byte(line), &value); err != nil {
			t.Fatalf("a line of JSON: %q: %v", line, err)
		}
		lines = append(lines, value)
	}
	return lines
}

func decode(t *testing.T, text string) any {
	t.Helper()
	var value any
	if err := json.Unmarshal([]byte(text), &value); err != nil {
		t.Fatal(err)
	}
	return value
}

func TestInitializeListsTheRulesWithTheirLanguages(t *testing.T) {
	got := replies(t, initialize, shutdown)[0]

	want := decode(t, `{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":1,"rules":[
		{"id":"acme/no-leading-x","languages":["python"]},
		{"id":"acme/js-anything","languages":["javascript"]},
		{"id":"acme/ts-options","languages":["typescript"]},
		{"id":"acme/project-files","languages":["python"]}]}}`)
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestLintFilesAnswersTheFindingsOfTheRulesOfTheLanguageOfEachFile(t *testing.T) {
	files := `{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"ok = 1\nx = 2\n"},{"path":"b.js","text":"x\n"}]}}`

	got := replies(t, files, shutdown)[0]

	want := decode(t, `{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[
		{"path":"a.py","rule":"acme/no-leading-x","severity":"error","start":7,"end":8,"message":"a line starts with x"},
		{"path":"b.js","rule":"acme/js-anything","severity":"warning","start":0,"end":1,"message":"js"}]}}`)
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestAnUnknownMethodGetsAnErrorAndTheServingGoesOn(t *testing.T) {
	got := replies(t, `{"jsonrpc":"2.0","id":7,"method":"nope"}`, shutdown)

	want := []any{
		decode(t, `{"jsonrpc":"2.0","id":7,"error":{"code":-32601,"message":"nope"}}`),
		decode(t, `{"jsonrpc":"2.0","id":3,"result":null}`),
	}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestServingEndsAfterShutdownAndLeavesTheRestOfTheInputUnread(t *testing.T) {
	got := replies(t, shutdown, initialize)

	want := []any{decode(t, `{"jsonrpc":"2.0","id":3,"result":null}`)}
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func TestAFileWithNoFindingsGetsAnEmptyListAndNotNull(t *testing.T) {
	files := `{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"ok = 1\n"}]}}`

	got := replies(t, files, shutdown)[0]

	want := decode(t, `{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[]}}`)
	if !reflect.DeepEqual(got, want) {
		t.Fatalf("got %v, want %v", got, want)
	}
}

func check(t *testing.T, got any, want string) {
	t.Helper()
	if expected := decode(t, want); !reflect.DeepEqual(got, expected) {
		t.Fatalf("got %v, want %v", got, expected)
	}
}

func TestLintFilesRunsOnlyTheRulesThatTheRequestNames(t *testing.T) {
	files := `{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"a.py","text":"x\n"},{"path":"b.js","text":"x\n"}],"rules":["acme/js-anything","acme/project-files"]}}`

	got := replies(t, files, shutdown)[0]

	check(t, got, `{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[
		{"path":"b.js","rule":"acme/js-anything","severity":"warning","start":0,"end":1,"message":"js"}]}}`)
}

func TestLintFilesGivesEachRuleItsOwnOptionsAndAnEmptyObjectWhenItHasNone(t *testing.T) {
	with := `{"jsonrpc":"2.0","id":2,"method":"lint/files","params":{"files":[{"path":"c.ts","text":"x\n"}],"options":{"acme/ts-options":{"max":3},"acme/js-anything":{"max":9}}}}`
	without := `{"jsonrpc":"2.0","id":4,"method":"lint/files","params":{"files":[{"path":"c.ts","text":"x\n"}]}}`

	got := replies(t, with, without, shutdown)

	check(t, got[0], `{"jsonrpc":"2.0","id":2,"result":{"diagnostics":[
		{"path":"c.ts","rule":"acme/ts-options","severity":"warning","start":0,"end":1,"message":"{\"max\":3}"}]}}`)
	check(t, got[1], `{"jsonrpc":"2.0","id":4,"result":{"diagnostics":[
		{"path":"c.ts","rule":"acme/ts-options","severity":"warning","start":0,"end":1,"message":"{}"}]}}`)
}

func TestLintProjectAnswersTheFindingsOfTheProjectRulesInTheFilesTheyName(t *testing.T) {
	project := `{"jsonrpc":"2.0","id":5,"method":"lint/project","params":{"root":"/repo","files":["a.py","b/c.py"],"options":{"acme/project-files":{"depth":2}}}}`

	got := replies(t, project, shutdown)[0]

	check(t, got, `{"jsonrpc":"2.0","id":5,"result":{"diagnostics":[
		{"path":"a.py","rule":"acme/project-files","severity":"info","start":0,"end":1,"message":"/repo {\"depth\":2}"},
		{"path":"b/c.py","rule":"acme/project-files","severity":"info","start":0,"end":1,"message":"/repo {\"depth\":2}"}]}}`)
}

func TestLintProjectRunsOnlyTheProjectRulesThatTheRequestNames(t *testing.T) {
	named := `{"jsonrpc":"2.0","id":5,"method":"lint/project","params":{"root":"/repo","files":["a.py"],"rules":["acme/no-leading-x"]}}`
	unnamed := `{"jsonrpc":"2.0","id":6,"method":"lint/project","params":{"root":"/repo","files":["a.py"]}}`

	got := replies(t, named, unnamed, shutdown)

	check(t, got[0], `{"jsonrpc":"2.0","id":5,"result":{"diagnostics":[]}}`)
	check(t, got[1], `{"jsonrpc":"2.0","id":6,"result":{"diagnostics":[
		{"path":"a.py","rule":"acme/project-files","severity":"info","start":0,"end":1,"message":"/repo {}"}]}}`)
}
