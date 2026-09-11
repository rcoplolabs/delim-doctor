# Delim Doctor

A stdio MCP server that scans source files for unbalanced delimiters (`() [] {}`) and optionally applies conservative fixes.

## Language

**DelimProblem**:
An imbalance point found during scanning — an unexpected close or a missing close.
_Avoid_: Error, issue, finding

**ScanReport**:
The result of scanning a file — a paginated list of DelimProblems with a text summary.
_Avoid_: Result, output

**FixReport**:
The result of fixing a file — applied and skipped fixes with a text summary.
_Avoid_: Result, output

**Lexer**:
A language-specific tokenizer that emits delimiter events while skipping comments, strings, and other non-code regions.
_Avoid_: Tokenizer, parser, scanner

**Scan**:
The deep module that orchestrates the full flow from a file path to a report — validating, reading, detecting language, scanning, and reporting.
_Avoid_: Service, handler, pipeline, scanner

**Workspace Root**:
The directory that constrains which files may be scanned. Defaults to the server's working directory.
_Avoid_: Base dir, root path, project root
