"""
Universal Language-Agnostic Query Preprocessor & Temporal Engine for AnyContext.
Provides high-performance single-pass date, temporal clause, and filename extraction.
Powered by native Rust (any-context-core-rs) with a zero-dependency pure-Python fallback.
"""
from typing import List, Optional
import re

try:
    import any_context_core_rs
    _RUST_AVAILABLE = True
except ImportError:
    _RUST_AVAILABLE = False


class _PythonProcessedQuery:
    def __init__(self, temporal_clauses: List[str], expanded_query: str, filename_mentions: List[str]):
        self.temporal_clauses = temporal_clauses
        self.expanded_query = expanded_query
        self.filename_mentions = filename_mentions

    def __repr__(self) -> str:
        return f"ProcessedQuery(temporal_clauses={self.temporal_clauses}, filename_mentions={self.filename_mentions}, expanded_query={self.expanded_query!r})"


# Language-agnostic RFC / English month table for international standardization
RFC_MONTH_MAP = {
    "january": "01", "jan": "01",
    "february": "02", "feb": "02",
    "march": "03", "mar": "03",
    "april": "04", "apr": "04",
    "may": "05",
    "june": "06", "jun": "06",
    "july": "07", "jul": "07",
    "august": "08", "aug": "08",
    "september": "09", "sept": "09", "sep": "09",
    "october": "10", "oct": "10",
    "november": "11", "nov": "11",
    "december": "12", "dec": "12",
}

# Regex singletons for pure-Python fallback (same logic as native Rust engine)
_ISO_RE = re.compile(r"\b(20\d{2})[-/](0[1-9]|1[0-2])[-/](0[1-9]|[12]\d|3[01])\b")
_INTL_NUMERIC_RE = re.compile(r"\b(0[1-9]|[12]\d|3[01])[-/.](0[1-9]|1[0-2])[-/.](20\d{2})\b")
_MONTHS_PATTERN = "|".join(sorted(RFC_MONTH_MAP.keys(), key=len, reverse=True))
_EN_MDY_RE = re.compile(rf"\b({_MONTHS_PATTERN})\s+(\d{{1,2}})(?:st|nd|rd|th)?(?:,?\s+(20\d{{2}}))?\b", re.IGNORECASE)
_EN_DMY_RE = re.compile(rf"\b(\d{{1,2}})(?:st|nd|rd|th)?\s+({_MONTHS_PATTERN})(?:\s+(?:,?\s*)?(20\d{{2}}))?\b", re.IGNORECASE)
_EN_MY_RE = re.compile(rf"\b({_MONTHS_PATTERN})\s+(?:of\s+)?(20\d{{2}})\b", re.IGNORECASE)
_SHORT_NUMERIC_RE = re.compile(r"\b(0?[1-9]|[12]\d|3[01])[-/.](0?[1-9]|1[0-2])\b")
_FILENAME_RE = re.compile(r"\b([a-zA-Z0-9_\-\.]+\.(?:pdf|csv|xlsx|xls|json|xml|docx|txt|md|log|tsv))\b", re.IGNORECASE)


def _py_is_part_of_larger_date(query: str, start: int, end: int) -> bool:
    if start > 0 and query[start - 1] in "-/0123456789.":
        return True
    if end < len(query) and query[end] in "-/0123456789.":
        return True
    return False


def _py_extract_temporal_clauses(query: str) -> List[str]:
    clauses: List[str] = []
    seen = set()

    def add_clauses(yy: Optional[str], mm: str, dd: Optional[str]):
        if yy and dd:
            for c in [
                f"file_path LIKE '%{yy}/{mm}/{dd}%'",
                f"file_path LIKE '%{yy}-{mm}-{dd}%'",
                f"file_path LIKE '%/{mm}/{dd}/%'",
                f"file_path LIKE '%/{mm}/{dd}%'",
            ]:
                if c not in seen:
                    seen.add(c)
                    clauses.append(c)
        elif dd:
            for c in [
                f"file_path LIKE '%/{mm}/{dd}/%'",
                f"file_path LIKE '%/{mm}/{dd}%'",
            ]:
                if c not in seen:
                    seen.add(c)
                    clauses.append(c)
        elif yy:
            for c in [
                f"file_path LIKE '%{yy}/{mm}/%'",
                f"file_path LIKE '%{yy}/{mm}%'",
            ]:
                if c not in seen:
                    seen.add(c)
                    clauses.append(c)

    # 1. ISO format: YYYY-MM-DD or YYYY/MM/DD
    for m in _ISO_RE.finditer(query):
        y, mm, d = m.group(1), m.group(2), m.group(3)
        add_clauses(y, mm, d)

    # 2. International numeric DD/MM/YYYY
    for m in _INTL_NUMERIC_RE.finditer(query):
        d, mm, y = m.group(1), m.group(2), m.group(3)
        add_clauses(y, mm, d)

    # 3. English Month Day Year
    for m in _EN_MDY_RE.finditer(query):
        m_name, d_str, y_str = m.group(1).lower(), m.group(2), m.group(3)
        mm = RFC_MONTH_MAP.get(m_name)
        if mm:
            dd = f"{int(d_str):02d}"
            add_clauses(y_str, mm, dd)

    # 4. English Day Month Year
    for m in _EN_DMY_RE.finditer(query):
        d_str, m_name, y_str = m.group(1), m.group(2).lower(), m.group(3)
        mm = RFC_MONTH_MAP.get(m_name)
        if mm:
            dd = f"{int(d_str):02d}"
            add_clauses(y_str, mm, dd)

    # 4B. English Month Year
    for m in _EN_MY_RE.finditer(query):
        m_name, y_str = m.group(1).lower(), m.group(2)
        mm = RFC_MONTH_MAP.get(m_name)
        if mm:
            add_clauses(y_str, mm, None)

    # 5. Short numeric dates without year
    for m in _SHORT_NUMERIC_RE.finditer(query):
        if _py_is_part_of_larger_date(query, m.start(), m.end()):
            continue
        v1, v2 = int(m.group(1)), int(m.group(2))
        pad1, pad2 = f"{v1:02d}", f"{v2:02d}"
        if v1 > 12:
            items = [
                f"file_path LIKE '%/{pad2}/{pad1}/%'",
                f"file_path LIKE '%/{pad2}/{pad1}%'",
                f"file_path LIKE '%{pad2}-{pad1}%'",
                f"file_path LIKE '%{pad1}-{pad2}%'",
            ]
        elif v2 > 12:
            items = [
                f"file_path LIKE '%/{pad1}/{pad2}/%'",
                f"file_path LIKE '%/{pad1}/{pad2}%'",
                f"file_path LIKE '%{pad1}-{pad2}%'",
                f"file_path LIKE '%{pad2}-{pad1}%'",
            ]
        else:
            items = [
                f"file_path LIKE '%/{pad2}/{pad1}/%'",
                f"file_path LIKE '%/{pad2}/{pad1}%'",
                f"file_path LIKE '%/{pad1}/{pad2}/%'",
                f"file_path LIKE '%/{pad1}/{pad2}%'",
                f"file_path LIKE '%{pad2}-{pad1}%'",
                f"file_path LIKE '%{pad1}-{pad2}%'",
            ]
        for it in items:
            if it not in seen:
                seen.add(it)
                clauses.append(it)

    return clauses


def _py_expand_query_temporal(query: str) -> str:
    expanded_tokens: List[str] = []
    seen = set()

    def add_token(t: str):
        if t not in seen:
            seen.add(t)
            expanded_tokens.append(t)

    # 1. ISO format
    for m in _ISO_RE.finditer(query):
        y, mm, d = m.group(1), m.group(2), m.group(3)
        for t in [f"{y}-{mm}-{d}", f"{y}/{mm}/{d}", f"{d}/{mm}/{y}", f"{mm}/{d}"]:
            add_token(t)

    # 2. International numeric format
    for m in _INTL_NUMERIC_RE.finditer(query):
        d, mm, y = m.group(1), m.group(2), m.group(3)
        for t in [f"{y}-{mm}-{d}", f"{y}/{mm}/{d}", f"{d}/{mm}/{y}", f"{mm}/{d}"]:
            add_token(t)

    # 3. English Month Day Year
    for m in _EN_MDY_RE.finditer(query):
        m_name, d_str, y_str = m.group(1).lower(), m.group(2), m.group(3)
        mm = RFC_MONTH_MAP.get(m_name)
        if mm:
            dd = f"{int(d_str):02d}"
            if y_str:
                for t in [f"{y_str}-{mm}-{dd}", f"{y_str}/{mm}/{dd}", f"{dd}/{mm}/{y_str}", f"{mm}/{dd}"]:
                    add_token(t)
            else:
                for t in [f"{mm}-{dd}", f"{mm}/{dd}", f"{dd}/{mm}"]:
                    add_token(t)

    # 4. English Day Month Year
    for m in _EN_DMY_RE.finditer(query):
        d_str, m_name, y_str = m.group(1), m.group(2).lower(), m.group(3)
        mm = RFC_MONTH_MAP.get(m_name)
        if mm:
            dd = f"{int(d_str):02d}"
            if y_str:
                for t in [f"{y_str}-{mm}-{dd}", f"{y_str}/{mm}/{dd}", f"{dd}/{mm}/{y_str}", f"{mm}/{dd}"]:
                    add_token(t)
            else:
                for t in [f"{mm}-{dd}", f"{mm}/{dd}", f"{dd}/{mm}"]:
                    add_token(t)

    # 4B. English Month Year
    for m in _EN_MY_RE.finditer(query):
        m_name, y_str = m.group(1).lower(), m.group(2)
        mm = RFC_MONTH_MAP.get(m_name)
        if mm:
            for t in [f"{y_str}-{mm}", f"{y_str}/{mm}"]:
                add_token(t)

    # 5. Short numeric dates without year
    for m in _SHORT_NUMERIC_RE.finditer(query):
        if _py_is_part_of_larger_date(query, m.start(), m.end()):
            continue
        v1, v2 = int(m.group(1)), int(m.group(2))
        pad1, pad2 = f"{v1:02d}", f"{v2:02d}"
        for t in [f"{pad1}/{pad2}", f"{pad2}/{pad1}", f"{pad1}-{pad2}", f"{pad2}-{pad1}"]:
            add_token(t)

    if not expanded_tokens:
        return query
    return f"{query} {' '.join(expanded_tokens)}"


def _py_extract_filename_mentions(query: str) -> List[str]:
    seen = set()
    filenames = []
    for m in _FILENAME_RE.finditer(query):
        fn = m.group(1)
        lower = fn.lower()
        if lower not in seen:
            seen.add(lower)
            filenames.append(fn)
    return filenames


class _PythonQueryPreprocessor:
    @classmethod
    def process(cls, query: str) -> _PythonProcessedQuery:
        return _PythonProcessedQuery(
            temporal_clauses=_py_extract_temporal_clauses(query),
            expanded_query=_py_expand_query_temporal(query),
            filename_mentions=_py_extract_filename_mentions(query),
        )


if _RUST_AVAILABLE and hasattr(any_context_core_rs, "QueryPreprocessor"):
    QueryPreprocessor = any_context_core_rs.QueryPreprocessor
    ProcessedQuery = any_context_core_rs.ProcessedQuery
    extract_temporal_clauses = any_context_core_rs.extract_temporal_clauses
    expand_query_temporal = any_context_core_rs.expand_query_temporal
    extract_filename_mentions = any_context_core_rs.extract_filename_mentions
else:
    QueryPreprocessor = _PythonQueryPreprocessor
    ProcessedQuery = _PythonProcessedQuery
    extract_temporal_clauses = _py_extract_temporal_clauses
    expand_query_temporal = _py_expand_query_temporal
    extract_filename_mentions = _py_extract_filename_mentions

__all__ = [
    "QueryPreprocessor",
    "ProcessedQuery",
    "extract_temporal_clauses",
    "expand_query_temporal",
    "extract_filename_mentions",
]
