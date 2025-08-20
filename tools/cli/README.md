# Forgepoint CLI

A comprehensive linter and validator for Forgepoint AsciiDoc documents.

## Installation

```bash
npm install -g @forgepoint/cli
```

## Usage

### Lint Documents

Validate all AsciiDoc files in your project:

```bash
forgepoint lint
```

Validate specific files:

```bash
forgepoint lint docs/**/*.adoc src/**/*.adoc
```

### Output Formats

- **Text** (default): Human-readable colored output
- **JSON**: Machine-readable JSON output
- **JUnit**: XML format for CI/CD integration

```bash
forgepoint lint --format json --output results.json
forgepoint lint --format junit --output results.xml
```

### Create New Documents

Create a new document from a template:

```bash
forgepoint create story user-authentication --title "User Authentication Story"
forgepoint create prd payment-system --author "Product Team"
```

### Check Single File

Validate a single file with detailed output:

```bash
forgepoint check ./docs/story/auth-login.adoc
```

### List Available Document Types

See all supported document types:

```bash
forgepoint list-types
```

## Configuration

Create a `.forgepointrc.json` file in your project root:

```json
{
  "schemaPath": "./schema",
  "excludePatterns": [
    "node_modules/**",
    "*.tmp.adoc"
  ],
  "rules": {
    "requireId": true,
    "enforceStructure": true,
    "validateReferences": true,
    "checkIdUniqueness": true
  },
  "output": {
    "format": "text",
    "verbose": false
  }
}
```

## Validation Rules

### Schema Validation
- Validates document attributes against JSON schemas
- Checks required fields and data types
- Validates enum values and patterns

### Structural Validation
- Ensures required sections are present
- Validates document title format
- Checks for proper AsciiDoc structure

### Reference Validation
- Validates cross-references between documents
- Checks `xref:type:id` syntax
- Reports broken internal references

### ID Uniqueness
- Ensures all document IDs are unique within a repository
- Validates ID format (lowercase, alphanumeric, hyphens only)
- Prevents duplicate IDs across different document types

## Document Types

Forgepoint supports 37 document types across the entire SDLC:

### Discovery (12 types)
- `story` - User Story
- `prd` - Product Requirements Document
- `okr` - Objectives and Key Results
- `prfaq` - Press Release + FAQ
- `mrd` - Market Requirements Document
- `brd` - Business Requirements Document
- `one-pager` - Executive Summary
- `opportunity-assessment` - Opportunity Evaluation
- `product-brief` - Product Specification
- `vision-strategy` - Vision & Strategy
- `product-roadmap` - Product Roadmap
- `shape-up-pitch` - Shape Up Pitch

### Design (5 types)
- `use-case` - Use Case
- `jtbd` - Jobs-to-be-Done
- `user-journey` - User Journey
- `technical-spec` - Technical Specification
- `api-spec` - API Specification

### Development (8 types)
- `epic` - Epic
- `task` - Task/Subtask
- `sprint-plan` - Sprint Plan
- `adr` - Architecture Decision Record
- `rfc` - Request for Comments
- `design-doc` - Design Document
- `risk-register` - Risk Register
- `safe-feature` - SAFe Feature

### Testing (5 types)
- `scenario` - BDD Scenario
- `test-case` - Test Case
- `test-plan` - Test Plan
- `test-results` - Test Results
- `bug-report` - Bug Report

### Release (6 types)
- `release-notes` - Release Notes
- `changelog` - Changelog
- `deployment-plan` - Deployment Plan
- `runbook` - Operational Runbook
- `postmortem` - Post-mortem
- `retrospective` - Retrospective
- `feature-flag` - Feature Flag

## Cross-References

Link between documents using the `xref` syntax:

```asciidoc
// Internal references
xref:story:user-authentication[]
xref:prd:payment-system[]

// External references
xref:github.com/acme/specs#story:user-auth@v1.0[]

// With custom text
xref:story:user-authentication[User Authentication Story]
```

## Document Structure

All Forgepoint documents must include these attributes:

```asciidoc
= Document Title
:forgepoint-type: story
:id: unique-document-id
:status: draft
:schema-version: 1.0
```

## Exit Codes

- `0` - All documents are valid
- `1` - Validation errors found or CLI error occurred

## Examples

### Basic Validation
```bash
# Validate all documents
forgepoint lint

# Validate with verbose output
forgepoint lint --verbose

# Exclude certain patterns
forgepoint lint --exclude "archive/**,*.draft.adoc"
```

### CI/CD Integration
```bash
# Generate JUnit XML for CI systems
forgepoint lint --format junit --output test-results.xml

# Fail on warnings in CI
forgepoint lint --fail-on-warnings
```

### Document Creation
```bash
# Create a user story
forgepoint create story auth-system --title "Authentication System"

# Create a PRD with author
forgepoint create prd mobile-app --title "Mobile App PRD" --author "Product Team"
```

## API Usage

The CLI can also be used as a library:

```typescript
import { ForgepointLinter, LinterConfig } from '@forgepoint/cli';

const config: LinterConfig = {
  schemaPath: './schema',
  excludePatterns: ['node_modules/**'],
  rules: {
    requireId: true,
    enforceStructure: true,
    validateReferences: true,
    checkIdUniqueness: true
  },
  output: {
    format: 'text',
    verbose: false
  }
};

const linter = new ForgepointLinter(config);
await linter.initialize();

const results = await linter.lintFiles(['**/*.adoc']);
console.log(results);
```