# Rhei: {{plan_title}}
**States:** parallel-spec-review

## Overview
This workspace reviews `{{spec}}` with a configurable parallel reviewer set,
summarizes their discussion into weighted findings, gates conflicts to a
human, and applies fixes for up to {{iterations}} iteration(s).

## Inputs

- Specification: `{{spec}}`
- Review iterations: {{iterations}}
- Review notes: `{{review_output_dir}}/`
- Summaries: `{{summary_output_dir}}/`
- Fix notes: `{{fix_output_dir}}/`
