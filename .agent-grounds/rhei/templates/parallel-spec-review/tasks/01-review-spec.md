### Task review-spec: Parallel spec review and fix loop for {{spec}}
**State:** review

Review and update `{{spec}}`.

The reviewer fan-out should inspect only the target specification and files it
explicitly references unless the focus text below requires adjacent context.

{% if review_focus %}
Additional focus:

{{review_focus}}
{% else %}
Use the default review focus: consistency, completeness, correctness,
implementability, edge cases, error handling, and clarity.
{% endif %}
