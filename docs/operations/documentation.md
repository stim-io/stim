# Documentation Method

This file defines the documentation method for `stim/docs`.

## Goal

Keep the docs surface small, methodological, and durable.

The target is not a complete repo encyclopedia.
The target is a few files that still teach the right boundary method after implementation details change.

## Keep rule

Keep a doc only when it provides one of these values:

- a durable ownership or structure method
- a stable contract that code and operators should obey
- a maintenance rule for keeping the docs surface small and clean
- a small amount of high-value supporting material that makes the method easier to apply, such as a necessary index, anti-pattern set, boundary check, or positive/negative examples

If a file mainly describes the current system, phase, or implementation shape, delete it or merge only the durable rule into a stronger canonical file.

Methodology must remain the main body of the docs surface.

Supporting material is allowed only when it clearly serves the method rather than replacing it.

## Organizing principle

Organize docs by method and contract, not by narrative description.

- `architecture/`: how to place responsibility and structure new work
- `contracts/`: stable contracts and operating boundaries
- `operations/`: methods for keeping the docs surface and local operator flow clean

## Canonical-source rule

Each important fact should have one canonical home.

- define a concept once
- in other docs, link to it instead of redefining it
- short navigational summaries are fine
- parallel full definitions are not

If two docs explain the same rule in full, the structure is wrong and should be corrected.

## Merge / delete rule

Prefer merging over multiplying files.

Split only when one file can no longer answer one primary method question cleanly.

Keep a short supporting index or example section when it materially lowers ambiguity for the method.

Do not keep standalone descriptive files just to preserve examples or navigation.

Delete files that exist mainly as:

- docs maps
- overview prose
- current-state description
- historical or transitional narrative

## Durable-content rule

Keep durable information. Remove descriptive residue.

Keep:

- stable boundaries
- client/host ownership methods
- API and host-control contracts
- operational methods that remain useful across changes

Do not keep active docs for:

- overview maps
- repo tours
- current-state descriptions
- migration plans
- phase notes
- worklists
- baselines
- historical cleanup notes
- transitional compatibility narratives

If something is only useful as history, delete it and rely on git history.

## Wording rule

Write docs as durable method or durable contract, not as a transition diary.

Avoid writing files whose main job is to say what the repo currently looks like.

Prefer explicit rules, criteria, anti-patterns, and decision tests.

Brief positive/negative examples are good when they help a reader apply the rule correctly.

Do not let examples grow into a second descriptive documentation layer.

## Document-boundary rule

Each document should answer one primary method or contract question.

If a document starts answering multiple primary questions, split it.

## Update process

When changing docs:

1. identify the canonical home for the fact you are changing
2. update that canonical document first
3. trim or relink any duplicated explanation elsewhere
4. if no good home exists, create one only if it carries a durable method or contract
5. prefer renaming, merging, or deleting docs over layering on another overlapping file
6. if the content is only descriptive or historical, delete it instead of archiving it
7. if you keep an index or examples, make sure they are short and clearly subordinate to the method

## Directory evolution rule

Directory structure may change when it helps collapse low-value description into fewer stronger method files.

Do not preserve a weak structure just because files already exist.

## Quality check before finishing

Before considering a docs change done, ask:

1. Is there exactly one canonical place for each important fact I touched?
2. Did I remove repeated explanations instead of adding another one?
3. Did I keep only durable, high-value information?
4. Does the file location match the method or contract question the document answers?
5. Could this file still matter after the current implementation details change?
