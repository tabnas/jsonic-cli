module github.com/tabnas/jsonic-cli/go

go 1.24.7

require (
	github.com/tabnas/debug/go v0.0.0-00010101000000-000000000000
	github.com/tabnas/jsonic/go v0.0.0-00010101000000-000000000000
	github.com/tabnas/parser/go v0.0.0
)

require github.com/tabnas/json/go v0.0.0-00010101000000-000000000000 // indirect

// jsonic-cli is the `jsonic` command-line tool: it parses relaxed-JSON
// (from args, --file, or STDIN) with @tabnas/jsonic and prints standard
// JSON. It wraps three unpublished @tabnas siblings at runtime — jsonic
// (the parser), parser (the engine type it configures) and debug (the
// --debug tracer). Until those publish tagged Go modules, depend on
// sibling checkouts, the same development model the TypeScript package
// uses (file:../../jsonic/ts, file:../../parser/ts, file:../../debug/ts).
// Clone tabnas/jsonic, tabnas/parser, tabnas/json and tabnas/debug as
// siblings of this repo.
replace github.com/tabnas/parser/go => ../../parser/go

replace github.com/tabnas/json/go => ../../json/go

replace github.com/tabnas/jsonic/go => ../../jsonic/go

replace github.com/tabnas/debug/go => ../../debug/go
