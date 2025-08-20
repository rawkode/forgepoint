# Forgepoint Document Structure Conventions

## General Conventions

### Document Attributes
All Forgepoint documents MUST include these attributes in the header:
- `:forgepoint-type:` - Document type (story, prd, okr, etc.)
- `:id:` - Unique identifier within the repository
- `:schema-version:` - Schema version (currently 1.0)

### ID Format
- Lowercase letters, numbers, and hyphens only
- Pattern: `^[a-z0-9-]+$`
- Examples: `auth-login`, `payment-system`, `2024-q1-growth`

### Cross-References
Use AsciiDoc's xref syntax with type prefixes:
- `xref:story:auth-login[]` - Reference to story
- `xref:prd:payment-system[]` - Reference to PRD
- `xref:github.com/org/repo#story:feature@v1.0[]` - External reference with version

## Document Type Structures

### User Story (`story`)

**Required Attributes:**
- `:forgepoint-type: story`
- `:id:` - Unique story identifier
- `:status:` - One of: draft, ready, in-progress, done, blocked
- `:schema-version: 1.0`

**Required Sections:**
- `== Acceptance Criteria` - Bulleted list with checkboxes

**Optional Sections:**
- `[abstract]` - Brief story description
- `== Scenarios` - BDD scenarios in Gherkin format
- `== Technical Notes` - Implementation notes
- `== Dependencies` - Prerequisites and blockers
- `== Related Items` - Cross-references

**Structure:**
```asciidoc
= Story Title
:forgepoint-type: story
:id: unique-story-id
:status: in-progress
:schema-version: 1.0

[abstract]
Brief description of the story.

== Acceptance Criteria
* [ ] Criterion 1
* [ ] Criterion 2

== Scenarios
[source,gherkin]
----
Feature: Feature Name
  Scenario: Scenario name
    Given precondition
    When action
    Then expected result
----
```

### Product Requirements Document (`prd`)

**Required Attributes:**
- `:forgepoint-type: prd`
- `:id:` - Unique PRD identifier
- `:status:` - One of: draft, review, approved, archived
- `:version:` - Semantic version (e.g., 1.0, 2.1.3)
- `:product:` - Product name
- `:owner:` - Product owner
- `:schema-version: 1.0`

**Required Sections:**
- `[abstract]` - Executive summary
- `== Problem Statement` - Problem being solved
- `== Goals` - Primary and secondary objectives
- `== Success Metrics` - Measurable success criteria
- `== Requirements` - Functional and non-functional requirements

**Optional Sections:**
- `== Background` - Context and history
- `== User Stories` - Related user stories
- `== Technical Requirements` - Technical specifications
- `== Dependencies` - Prerequisites and dependencies
- `== Timeline` - Project phases and milestones
- `== Risks` - Risk assessment and mitigation
- `== Open Questions` - Unresolved questions

### OKR (`okr`)

**Required Attributes:**
- `:forgepoint-type: okr`
- `:id:` - Unique OKR identifier
- `:status:` - One of: draft, active, completed, cancelled
- `:period:` - Time period (2024-Q1, 2024-H1, 2024-annual)
- `:level:` - One of: company, team, individual
- `:owner:` - OKR owner
- `:schema-version: 1.0`

**Required Sections:**
- `== Key Results` - Measurable outcomes (1-5 items)

**Optional Sections:**
- `== Context` - Background and rationale
- `== Initiatives` - Key projects/activities
- `== Dependencies` - Prerequisites
- `== Updates` - Progress updates

**Key Results Format:**
```asciidoc
== Key Results
* [ ] Increase user engagement by 25% (current: 60%, target: 75%)
* [ ] Reduce page load time to under 2 seconds (current: 3.2s)
* [ ] Achieve 95% uptime for core services
```

### BDD Scenario (`scenario`)

**Required Attributes:**
- `:forgepoint-type: scenario`
- `:id:` - Unique scenario identifier
- `:status:` - One of: draft, ready, automated, passing, failing, deprecated
- `:feature:` - Feature name
- `:schema-version: 1.0`

**Required Sections:**
- `== Scenarios` - Gherkin-formatted scenarios

**Gherkin Format:**
```asciidoc
== Scenarios
[source,gherkin]
----
Feature: Feature Name

  Background:
    Given common preconditions

  Scenario: Happy path
    Given user is logged in
    When they perform an action
    Then they see expected result

  Scenario Outline: Multiple inputs
    Given user enters "<input>"
    When they submit
    Then they see "<result>"

    Examples:
      | input | result |
      | valid | success |
      | invalid | error |
----
```

## Formatting Guidelines

### Admonitions
Use AsciiDoc's built-in admonitions:
```asciidoc
NOTE: Additional information

TIP: Helpful suggestion  

IMPORTANT: Critical information

WARNING: Potential issue

CAUTION: Risk or danger
```

### Tables
Use AsciiDoc table syntax for structured data:
```asciidoc
[cols="2,2,1,3"]
|===
|Column 1 |Column 2 |Column 3 |Column 4

|Row 1 Col 1
|Row 1 Col 2
|Row 1 Col 3
|Row 1 Col 4
|===
```

### Code Blocks
Specify language for syntax highlighting:
```asciidoc
[source,javascript]
----
function authenticate(user) {
  return jwt.sign(user, secret);
}
----
```

### Links
- External: `https://example.com[Link text]`
- Internal references: `xref:story:auth-login[]`
- With custom text: `xref:story:auth-login[Authentication Story]`
- With anchors: `xref:story:auth-login#acceptance-criteria[]`

## Validation Rules

1. **ID Uniqueness**: No duplicate IDs within a repository
2. **Required Sections**: Each document type must include required sections
3. **Attribute Validation**: All required attributes must be present and valid
4. **Reference Integrity**: All xref links should resolve to existing documents
5. **Schema Compliance**: Documents must validate against their JSON schemas