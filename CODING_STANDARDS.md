# Coding Standards

## Testing

Tautological tests considered harmful.

A test is tautological when it can only fail if someone edits the line it restates. It re-asserts a declaration instead of exercising behavior. Don't write these, and delete them when you find them:

- Asserting a literal the code returns verbatim: an endpoint string, a hand-built JSON body, a constant (`assert_eq!(BATCH_SIZE, 50)`).
- Asserting serde attributes: that `#[serde(rename = "userId")]` maps `userId`, or that a derive deserializes a sample.
- Asserting clap declarations: `--help` lists a subcommand, a `required`/`requires`/`conflicts_with`/`ValueEnum` arg exits 2.
- Asserting a mock's own response comes back unchanged from a pass-through call (mock returns X, assert the tool returned X).
- Asserting only that a call succeeded, with nothing about what it sent or returned.

Test behavior instead: logic that branches, transforms, resolves, validates, or chooses. That includes defaults, partial updates, name resolution, date math, batching, error mapping, and which requests are (or are not) sent. Check the result against an independent source of truth: the API's documented shape, the Python server's contract (`tests/fixtures/mcp-contract.json`), or a value computed by hand.
