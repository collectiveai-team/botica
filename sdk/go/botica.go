// Package botica is the SDK to write a botica lint rule as an extension in Go: NDJSON JSON-RPC 2.0
// over stdio.
//
// An extension lists its rules and calls Serve. A rule of file scope receives (path, text, options)
// and returns its findings, with Start and End as byte offsets of the text. A rule with Scope
// "project" receives (root, paths, options) in CheckProject, reads the files itself and returns
// findings that carry their Path. options holds the options that the configuration of the project
// gives the rule. Serve answers initialize, lint/files, lint/project and shutdown, and runs the
// rules that each request names.
package botica

import (
	"bufio"
	"encoding/json"
	"io"
	"os"
	"path"
	"strings"
)

// ProtocolVersion is the version of the protocol of the extension host.
const ProtocolVersion = 1

// Finding is one finding: Start and End are byte offsets in the UTF-8 text of the file. Path is
// the file of a finding of a rule of project scope; a rule of file scope leaves it empty.
type Finding struct {
	Path    string
	Start   int
	End     int
	Message string
}

// Rule is a rule of the extension. Severity is "warning" when it is empty. Scope is "file" when it
// is empty; a rule of file scope has Check, and a rule of Scope "project" has CheckProject, whose
// findings without a Path are skipped.
type Rule struct {
	ID           string
	Languages    []string
	Severity     string
	Scope        string
	Check        func(path, text string, options map[string]any) []Finding
	CheckProject func(root string, paths []string, options map[string]any) []Finding
}

var languages = map[string]string{
	".py":  "python",
	".ts":  "typescript",
	".tsx": "typescript",
	".js":  "javascript",
}

type object = map[string]any

func (rule Rule) severity() string {
	if rule.Severity == "" {
		return "warning"
	}
	return rule.Severity
}

func (rule Rule) scope() string {
	if rule.Scope == "" {
		return "file"
	}
	return rule.Scope
}

func (rule Rule) diagnostic(path string, found Finding) object {
	return object{
		"path":     path,
		"rule":     rule.ID,
		"severity": rule.severity(),
		"start":    found.Start,
		"end":      found.End,
		"message":  found.Message,
	}
}

func (rule Rule) reads(language string) bool {
	for _, own := range rule.Languages {
		if own == language {
			return true
		}
	}
	return false
}

type file struct {
	Path string `json:"path"`
	Text string `json:"text"`
}

type params struct {
	Root    string          `json:"root"`
	Files   json.RawMessage `json:"files"`
	Rules   *[]string       `json:"rules"`
	Options map[string]any  `json:"options"`
}

type request struct {
	ID     json.RawMessage `json:"id"`
	Method string          `json:"method"`
	Params params          `json:"params"`
}

type namedRule struct {
	rule    Rule
	options map[string]any
}

// named are the rules of scope that the request names (every one when it names none), each with
// its options: an empty map when the request gives it none.
func named(rules []Rule, p params, scope string) []namedRule {
	var found []namedRule
	for _, rule := range rules {
		if rule.scope() != scope || (p.Rules != nil && !contains(*p.Rules, rule.ID)) {
			continue
		}
		options, _ := p.Options[rule.ID].(map[string]any)
		if options == nil {
			options = map[string]any{}
		}
		found = append(found, namedRule{rule, options})
	}
	return found
}

func contains(values []string, value string) bool {
	for _, own := range values {
		if own == value {
			return true
		}
	}
	return false
}

func lint(rules []Rule, p params) ([]object, error) {
	var files []file
	if len(p.Files) > 0 {
		if err := json.Unmarshal(p.Files, &files); err != nil {
			return nil, err
		}
	}
	diagnostics := []object{}
	run := named(rules, p, "file")
	for _, f := range files {
		language := languages[strings.ToLower(path.Ext(f.Path))]
		for _, n := range run {
			if n.rule.Check == nil || !n.rule.reads(language) {
				continue
			}
			for _, found := range n.rule.Check(f.Path, f.Text, n.options) {
				diagnostics = append(diagnostics, n.rule.diagnostic(f.Path, found))
			}
		}
	}
	return diagnostics, nil
}

func lintProject(rules []Rule, p params) ([]object, error) {
	var paths []string
	if len(p.Files) > 0 {
		if err := json.Unmarshal(p.Files, &paths); err != nil {
			return nil, err
		}
	}
	diagnostics := []object{}
	for _, n := range named(rules, p, "project") {
		if n.rule.CheckProject == nil {
			continue
		}
		for _, found := range n.rule.CheckProject(p.Root, paths, n.options) {
			if found.Path != "" {
				diagnostics = append(diagnostics, n.rule.diagnostic(found.Path, found))
			}
		}
	}
	return diagnostics, nil
}

func answer(rules []Rule, req request) object {
	reply := object{"jsonrpc": "2.0", "id": req.ID}
	switch req.Method {
	case "initialize":
		listed := []object{}
		for _, rule := range rules {
			listed = append(listed, object{"id": rule.ID, "languages": rule.Languages})
		}
		reply["result"] = object{"protocolVersion": ProtocolVersion, "rules": listed}
	case "lint/files":
		answerWith(reply, lint, rules, req.Params)
	case "lint/project":
		answerWith(reply, lintProject, rules, req.Params)
	case "shutdown":
		reply["result"] = nil
	default:
		reply["error"] = object{"code": -32601, "message": req.Method}
	}
	return reply
}

func answerWith(reply object, run func([]Rule, params) ([]object, error), rules []Rule, p params) {
	diagnostics, err := run(rules, p)
	if err != nil {
		reply["error"] = object{"code": -32602, "message": err.Error()}
		return
	}
	reply["result"] = object{"diagnostics": diagnostics}
}

// ServeOn answers the requests of in, one reply on out for each, until shutdown or the end of in.
func ServeOn(rules []Rule, in io.Reader, out io.Writer) error {
	reader := bufio.NewReader(in)
	encoder := json.NewEncoder(out)
	for {
		line, readErr := reader.ReadBytes('\n')
		if len(strings.TrimSpace(string(line))) > 0 {
			var req request
			if err := json.Unmarshal(line, &req); err != nil {
				return err
			}
			if err := encoder.Encode(answer(rules, req)); err != nil {
				return err
			}
			if req.Method == "shutdown" {
				return nil
			}
		}
		if readErr == io.EOF {
			return nil
		}
		if readErr != nil {
			return readErr
		}
	}
}

// Serve answers on stdin and stdout, and returns after shutdown.
func Serve(rules []Rule) error {
	return ServeOn(rules, os.Stdin, os.Stdout)
}
