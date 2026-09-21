---
name: temporal-grounding
description: Enforces temporal recency primacy, resolves short numeric dates, and ensures active status notices override historical documents.
caller_types:
  - human
  - mcp
---

# ⏱️ Temporal Grounding & Metadata Recency Directives

## 🎯 When to Activate
Activate across all interfaces whenever queries or retrieved documents involve dates, timestamps, versions, program statuses, or chronological records.

---

## 🛡️ Core Rules & Behavioral Directives

### 1. Recency Primacy (Most Recent Source Prevails)
- Every retrieved chunk includes explicit metadata: `Last Modified: YYYY-MM-DD` and `Type: [...]`.
- When documents contain differing facts, numbers, prices, or versions, **THE MOST RECENT SOURCE ALWAYS PREVAILS AND SUPERSEDES OLDER DATA**.
- If a newly updated document or real-time web source contains newer facts than an older document, prioritize the latest information and explicitly note the date discrepancy.

### 2. Status Notice Precedence Over Historical Documents
- Official landing pages, active checklists, and current operational reports represent **CURRENT OPERATIONAL TRUTH**.
- When an official service page or report displays an explicit status notice (e.g. `Status: Paused`, `Closed to new applicants`, `Suspended`, `Cancelled`), this status **ALWAYS OVERRIDES** older press releases, expansion plans, or historical announcements from prior years.
- Clearly state the current active status prominently.

### 3. Year-Agnostic Short Numeric Date Resolution
- When the user asks about short numeric dates without a year (e.g., `02/09`, `2/9`, `02-09`), anchor the search to the matching file paths and timestamps (e.g. `2026/09/02/` or `09/02/2026`).
- If only one year in the workspace contains documents for that date, resolve cleanly and state the adopted year scope.
