// Copyright (c) 2020-2026 Richard Rodger, Oliver Sturm, and other
// contributors, MIT License

// Command jsonic is a JSON parser that isn't strict. It reads relaxed-JSON
// source (from command arguments, --file, or STDIN), parses it with
// github.com/tabnas/jsonic/go, merges the results in precedence, and prints
// standard JSON.
//
// It is a thin wrapper around the cli package, which holds the Go port of
// ts/src/jsonic-cli.ts. The dynamic plugin loader (`require`) that the
// TypeScript CLI uses has no Go equivalent, so -p/--plugin resolves against
// the cli package's compiled-in plugin registry (debug, jsonic, and json
// are built in). To ship more plugins, build a custom binary that calls
// cli.RegisterPlugin before cli.Run — see the cli package documentation.
package main

import (
	"os"

	cli "github.com/tabnas/jsonic-cli/go/cli"
)

// Version is the current version of the jsonic-cli Go module. Kept in sync
// with ts/package.json by the release orchestrator (admin/publish.sh).
//
// This const sits under go/cmd/ rather than in a package root, which the
// orchestrator's file discovery used to miss — so it silently stayed at 0.4.0
// while the package shipped 0.4.1 and 0.4.2. Discovery now searches the whole
// module, so future releases rewrite it. If you move this const, keep it the
// only `^const Version =` in the module.
const Version = "0.4.3"

func main() {
	// os.Args[1:] mirrors the TypeScript run()'s argv slice from index 2
	// (it skips node and the script path; we skip the binary path).
	code := cli.Run(os.Args[1:], cli.ReadStdin(), os.Stdout, nil)
	os.Exit(code)
}
