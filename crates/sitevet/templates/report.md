# Audit report for {{ v.start_url }}

Overall score: **{{ v.score }}/100** ({{ v.band_label }}). {{ v.pages_count }} {% if v.pages_count == 1 %}page{% else %}pages{% endif %} audited on {{ v.generated_at }} in {{ v.duration }}.
{%- if v.final_url != v.start_url %}

The start URL redirected to {{ v.final_url }}.
{%- endif %}

## Category scores
{% if v.categories.is_empty() %}
No checks ran.
{% else %}
| Category | Score | Fail | Warn | Pass |
| --- | --- | --- | --- | --- |
{%- for c in v.categories %}
| {{ c.name }} | {{ c.score }} | {{ c.fail }} | {{ c.warn }} | {{ c.pass }} |
{%- endfor %}
{% endif %}
## Pages
{% for p in v.pages %}
### {{ p.url }} (score {{ p.score }}/100)
{% if let Some(f) = p.final_url %}
Final URL: {{ f }}
{% endif -%}
{% if let Some(e) = p.error %}
Could not fetch this page: {{ e }}
{% else if p.status >= 400 %}
HTTP status {{ p.status }}.
{% endif -%}
{% for g in p.groups %}
#### {{ g.name }}
{% if g.issues.is_empty() %}
All {{ g.passed }} checks passed.
{% else %}
| Status | Rule | Message | Evidence |
| --- | --- | --- | --- |
{%- for f in g.issues %}
| {{ f.word }} | {{ f.rule_id }} | {{ f.message }} | {% if let Some(e) = f.evidence %}{{ e }}{% endif %} |
{%- endfor %}
{% if g.passed > 0 %}
{{ g.passed }} other {% if g.passed == 1 %}check{% else %}checks{% endif %} passed.
{% endif -%}
{% endif -%}
{% endfor -%}
{% if !p.metric_groups.is_empty() %}
#### Measurements
{% for g in p.metric_groups %}
##### {{ g.name }}
{% if !g.values.is_empty() %}
| Metric | Value | Status | Note |
| --- | --- | --- | --- |
{%- for m in g.values %}
| {{ m.label }} | {{ m.value }} | {% if m.status.is_empty() %}-{% else %}{{ m.status }}{% endif %} | {{ m.note }} |
{%- endfor %}
{% endif -%}
{% for t in g.tables %}
**{{ t.label }}**{% if !t.caption.is_empty() %} ({{ t.caption }}){% endif %}
{% if t.rows.is_empty() %}
None found.
{% else %}
|{% for c in t.columns %} {{ c }} |{% endfor %}
|{% for c in t.columns %} --- |{% endfor %}
{%- for r in t.rows %}
|{% for cell in r %} {{ cell }} |{% endfor %}
{%- endfor %}
{% endif -%}
{% endfor -%}
{% endfor -%}
{% endif -%}
{% endfor -%}
{% if !v.blocked.is_empty() %}
## Blocked by robots.txt

| URL | Rule |
| --- | --- |
{%- for (url, rule) in v.blocked %}
| {{ url }} | {{ rule }} |
{%- endfor %}
{% endif %}
---

sitevet {{ v.version }}, report schema {{ v.schema_version }}.
