// Package botica is the SDK to write a botica lint rule as an extension in Go: NDJSON JSON-RPC 2.0
// over stdio.
//
// An extension lists its rules and calls Serve. Each rule receives (path, text) and returns its
// findings, with Start and End as byte offsets of the text. Serve answers initialize, lint/files
// and shutdown.
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

// Finding is one finding: Start and End are byte offsets in the UTF-8 text of the file.
type Finding struct {
	Start   int
	End     int
	Message string
}

// Rule is a rule of the extension. Severity is "warning" when it is empty.
type Rule struct {
	ID        string
	Languages []string
	Severity  string
	Check     func(path, text string) []Finding
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

type request struct {
	ID     json.RawMessage `json:"id"`
	Method string          `json:"method"`
	Params struct {
		Files []file `json:"files"`
	} `json:"params"`
}

func lint(rules []Rule, files []file) []object {
	diagnostics := []object{}
	for _, f := range files {
		language := languages[strings.ToLower(path.Ext(f.Path))]
		for _, rule := range rules {
			if !rule.reads(language) {
				continue
			}
			for _, found := range rule.Check(f.Path, f.Text) {
				diagnostics = append(diagnostics, object{
					"path":     f.Path,
					"rule":     rule.ID,
					"severity": rule.severity(),
					"start":    found.Start,
					"end":      found.End,
					"message":  found.Message,
				})
			}
		}
	}
	return diagnostics
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
		reply["result"] = object{"diagnostics": lint(rules, req.Params.Files)}
	case "shutdown":
		reply["result"] = nil
	default:
		reply["error"] = object{"code": -32601, "message": req.Method}
	}
	return reply
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
