---
title: '[Good First Issue] Enhance McpDashboard UI Loading State'
labels: ['good first issue', 'management-console', 'ui']
assignees: ''
---

## Description
The `McpDashboard.tsx` in the Aiome Management Console fetches the active MCP status from `/api/skills`. Currently, while fetching this data, there is no explicit visual loading indicator for the status pills (they just appear grey or default until the fetch completes).

## Task Requirements
1. Update `apps/management-console/src/components/McpDashboard.tsx`.
2. Add a `isLoadingSkills` state boolean.
3. Show a skeleton loader or a spinner next to the "Active Skills" count while fetching.
4. Update the `McpDashboard.test.tsx` to verify that the loading state is rendered correctly before the mock API resolves.

## Why this is a Good First Issue
It provides exposure to the React frontend of Aiome and introduces our standard testing patterns using `vitest` and `@testing-library/react`.

## TDD Acceptance Criteria
- [ ] `npm run test` passes for the Management Console frontend.
- [ ] The loading indicator matches the existing design system (`tokens.css`).

## Getting Started
Please comment "I'd like to work on this!" below to get assigned. Review `CONTRIBUTING.md` for our workspace setup instructions.
