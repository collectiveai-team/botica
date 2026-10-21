---
title: Editor
---

# Editor

```bash
botica server                  # the Language Server Protocol on stdio: the findings for the buffers of an
                               # editor (native and pattern rules), paths from the working directory
botica server --extensions     # and the extension rules of the catalogs of the project
```

`botica server` is a language server on stdin and stdout. It takes the paths of the buffers from
the directory it starts in: start it at the root of the project.

It publishes the findings of the active native and pattern rules for each open buffer. The
findings of a rule with `type` in its `run-on` change with each edit. Those of a rule with `save`
and without `type` are found when the buffer opens and at each save, and stay as they are between
two saves. A rule of file scope whose `run-on` is `["cli"]` does not show in the editor. Each save
also runs the native rules of project scope (`external-config`, `settings-env-schema`,
`import-layers`) over the files on disk, on a thread of their own, and their findings are published
for the files they name, open or not.

A save of `botica.toml`, of `pyproject.toml` or of a file under `.botica/`, or a
`workspace/didChangeWatchedFiles` that names one, reads the configuration again and lints the open
buffers with it. The server does not ask the editor to watch files. A configuration that does not
load is reported in a `window/showMessage` warning, and the rules stay as they were. The processes
of the extension rules are the ones the session started: a change to them needs a restart of the
server.

The editor sends each change as a range (incremental sync). A buffer in a language of the pattern
rules keeps its syntax tree, and an edit parses it again from where the text changed. Columns are
UTF-16 code units, or bytes when the editor lists `utf-8` in
`capabilities.general.positionEncodings`.

The option `{"extensions": true}` of `initialize`, or `botica server --extensions`, runs the
extension rules too ([Extensions in the editor](../extend/extensions.md#in-the-editor)).

## Neovim

Neovim 0.11 or later, in `init.lua`:

```lua
vim.lsp.config('botica', {
  cmd = { 'botica', 'server' },
  filetypes = { 'python', 'typescript', 'javascript' },
  root_markers = { 'botica.toml', 'pyproject.toml', '.git' },
  -- init_options = { extensions = true },
})
vim.lsp.enable('botica')
```

## Helix

Helix, in `languages.toml`. `language-servers` replaces the list of the language: name the servers
you keep. `config` is what Helix sends as `initializationOptions`.

```toml
[language-server.botica]
command = "botica"
args = ["server"]
# config = { extensions = true }

[[language]]
name = "python"
language-servers = ["pyright", "ruff", "botica"]
```

## Emacs

Eglot runs one server for a buffer, so this one replaces the Python server:

```elisp
(with-eval-after-load 'eglot
  (add-to-list 'eglot-server-programs
               '((python-mode python-ts-mode) . ("botica" "server"))))
```

lsp-mode runs it next to the others with `:add-on? t`:

```elisp
(with-eval-after-load 'lsp-mode
  (lsp-register-client
   (make-lsp-client :new-connection (lsp-stdio-connection '("botica" "server"))
                    :activation-fn (lsp-activate-on "python")
                    :server-id 'botica
                    :add-on? t)))
```

## Zed

Zed starts the language servers that its extensions declare
([language extensions](https://zed.dev/docs/extensions/languages)). botica has no Zed extension: one
would declare a server for the languages of the rules and return `botica server` as its command.
