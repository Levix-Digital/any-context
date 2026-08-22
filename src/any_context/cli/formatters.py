"""
AnyContext CLI Presentation Layer & Formatters (v0.24.1).
Encapsulates all terminal-specific visual formatting, ANSI color palettes, ASCII/Unicode cards,
live terminal progress tickers, help page renderers, and interactive questionary wizards.
Part of the Hexagonal Architecture CLI Adapter layer.
"""
import os
import sys
import urllib.parse
from typing import Dict, Any, List, Optional
import questionary

from any_context.cli.spinner import Spinner
from any_context.ingestion.web_crawler import discover_site_urls, crawl_website, crawl_and_index_urls
from any_context.ingestion.web_scheduler import WebSchedulerStore
from any_context.billing.models import PlanTier
from any_context.help.models import HelpPage
from any_context.help.registry import HELP_REGISTRY, get_help_page


def safe_stdout_write(msg: str):
    """Safely writes strings to stdout handling legacy Windows encoding errors."""
    try:
        sys.stdout.write(msg)
        sys.stdout.flush()
    except (UnicodeEncodeError, Exception):
        try:
            clean_msg = msg.encode("ascii", errors="ignore").decode("ascii")
            sys.stdout.write(clean_msg)
            sys.stdout.flush()
        except Exception:
            pass


def safe_print(msg: str):
    """Safely prints strings handling legacy Windows encoding errors."""
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))


def format_sync_status_box(diff: Dict[str, Any]) -> str:
    """Formats a modern, comprehensive multi-source sync status card with ANSI colors for the CLI."""
    ws_name = diff.get("workspace_name", "Default")
    total_sources = diff.get("total_sources", 0)
    src_label = f" ({total_sources} source{'s' if total_sources != 1 else ''})" if total_sources > 0 else " (Empty)"

    lines = [f"┌ 🔍 \033[1mWorkspace Sync Status: {ws_name}{src_label}\033[0m"]

    # 1. Local Folders
    folders = diff.get("folders", [])
    disk_files = diff.get("total_disk_files", 0)
    cached_files = diff.get("total_cached_files", 0)
    if folders:
        lines.append(f"│ ├─ 📂 Local Folders : {len(folders)} folder{'s' if len(folders) != 1 else ''} ({disk_files} files on disk, {cached_files} cached)")
        for f in folders[:3]:
            lines.append(f"│ │    • [Folder] {f}")
        if len(folders) > 3:
            lines.append(f"│ │    • ... (+ {len(folders) - 3} more folders)")
    else:
        lines.append(f"│ ├─ 📂 Local Folders : 0 folders (0 files on disk, 0 cached)")

    # 2. Web Sources
    web_sources = diff.get("web_sources", [])
    web_pages = diff.get("web_pages_count", 0)
    if web_sources:
        lines.append(f"│ ├─ 🌐 Web Sources   : {len(web_sources)} portal{'s' if len(web_sources) != 1 else ''} ({web_pages} pages indexed)")
        for w in web_sources[:3]:
            title = w.get("title") or w.get("url")
            p_cnt = w.get("page_count", 1) or 1
            lines.append(f"│ │    • [Web] {w.get('url')} ({title} • {p_cnt} pages)")
        if len(web_sources) > 3:
            lines.append(f"│ │    • ... (+ {len(web_sources) - 3} more portals)")
    else:
        lines.append(f"│ ├─ 🌐 Web Sources   : 0 portals")

    # 3. Cloud Drives
    cloud_drives = diff.get("cloud_drives", [])
    if cloud_drives:
        lines.append(f"│ ├─ ☁️ Cloud Drives  : {len(cloud_drives)} connected")
        for cd in cloud_drives[:3]:
            dtype = (cd.get("drive_type") or "drive").capitalize()
            dname = cd.get("folder_name") or cd.get("folder_id") or "Drive Folder"
            lines.append(f"│ │    • [{dtype}] {dname}")
        if len(cloud_drives) > 3:
            lines.append(f"│ │    • ... (+ {len(cloud_drives) - 3} more drives)")
    else:
        lines.append(f"│ ├─ ☁️ Cloud Drives  : 0 connected")

    # 4. Pending Status & Up to Date
    lines.append(f"│ ├─ 📦 Pending Status: {diff.get('summary', 'Up to date')}")
    status_str = "Yes (0 changes)" if diff.get("is_up_to_date") else "No (Changes detected - run '/sync' to update)"
    lines.append(f"│ └─ ⚡ Up to Date   : {status_str}")
    lines.append("└─────────────────────────────────────────────────────────────")
    return "\n".join(lines)


def format_pricing_plans_cli(plans: List[PlanTier], current_tier: str) -> str:
    """Formats the pricing and plan capability matrix with ANSI colors for CLI presentation."""
    lines = []
    lines.append("=" * 80)
    lines.append("                    ANYCONTEXT PLANS & CAPABILITY MATRIX                    ")
    lines.append("=" * 80)

    for idx, p in enumerate(plans, 1):
        is_active = (p.tier_id == current_tier)
        active_badge = " [PLANO ATIVO]" if is_active else ""

        if p.tier_id == "community":
            m_price = "Grátis ($0 / sempre)"
            a_price = "Grátis ($0 / ano)"
        elif p.tier_id == "team":
            m_price = f"${p.monthly_price_usd:.0f}/mês (5 seats inclusos) + ${p.extra_seat_price_usd:.0f}/seat extra"
            a_price = f"${p.annual_price_usd:.0f}/ano base (~$65/mês - 20% OFF)"
        else:
            m_price = f"${p.monthly_price_usd:.0f}/mês"
            a_price = f"${p.annual_price_usd:.0f}/ano (~${p.annual_price_usd/12:.0f}/mês - 20% OFF)" if p.annual_price_usd else "-"

        lines.append(f"\n  [{idx}] \033[1;97m{p.name}\033[0m\033[92m{active_badge}\033[0m")
        lines.append("  " + "-" * 74)
        lines.append(f"  • \033[1mPreço Mensal\033[0m   : \033[92m{m_price}\033[0m")
        lines.append(f"  • \033[1mPreço Anual\033[0m    : \033[96m{a_price}\033[0m")
        lines.append(f"  • \033[1mEscopo & RAG\033[0m   : {p.ingestion_scope}")
        lines.append(f"  • \033[1mPúblico-Alvo\033[0m   : \033[90m{p.target_audience}\033[0m")

    lines.append("\n" + "=" * 80)
    return "\n".join(lines)


def format_crawler_discovery_report(
    title: str,
    start_url: str,
    section_count: int,
    domain_count: int,
    already_indexed_count: int,
    new_section_count: int,
    new_domain_count: int,
    has_sitemap: bool
) -> str:
    """Formats the website discovery report with ANSI colors for CLI presentation."""
    lines = [
        "\n================================================================================",
        f"🌐 \033[93mWebsite Discovery Report:\033[0m \033[1m{title}\033[0m",
        f"🔗 \033[96m{start_url}\033[0m",
        "================================================================================",
        f"  • 📄 Section Pages (matching path prefix) : \033[92m{section_count}\033[0m pages",
        f"  • 🌐 Total Internal Domain URLs Found    : \033[92m{domain_count}\033[0m pages",
    ]
    if already_indexed_count > 0:
        lines.append(f"  • 📦 Already Indexed in this Workspace   : \033[96m{already_indexed_count}\033[0m pages (Cached in Vector DB)")
    else:
        lines.append(f"  • 📦 Already Indexed in this Workspace   : \033[90m0 pages (First time indexing)\033[0m")
    lines.append(f"  • ✨ New Unindexed Pages Available       : \033[93m{new_section_count}\033[0m section / \033[93m{new_domain_count}\033[0m domain pages")
    lines.append(f"  • 🗺️ XML Sitemap Detected                : \033[95m{'Yes (Structured XML)' if has_sitemap else 'No (Fast Recursive Link Scan)'}\033[0m")
    lines.append("================================================================================\n")
    return "\n".join(lines)


def run_interactive_web_crawler(workspace_name: str, start_url: Optional[str] = None) -> bool:
    """
    CLI Wizard: Guides the user through interactive website discovery, scope selection, and concurrent crawling.
    """
    if not start_url:
        start_url = questionary.text(
            "Enter website URL or documentation portal (e.g. https://docs.python.org/3/):"
        ).ask()
        if not start_url or not start_url.strip():
            return False

    start_url = start_url.strip()
    if not start_url.startswith("http://") and not start_url.startswith("https://"):
        start_url = f"https://{start_url}"

    # 1. Discovery Phase with clean Spinner
    with Spinner(f"Mapping site structure, internal links & sitemaps for '{start_url}'..."):
        disc = discover_site_urls(start_url)

    title = disc.get("title") or start_url
    section_count = disc.get("section_count", 1)
    domain_count = disc.get("domain_count", 1)
    has_sitemap = disc.get("has_sitemap", False)
    domain = disc.get("domain") or urllib.parse.urlparse(start_url).netloc.lower()

    # Query already indexed pages in this workspace for this domain
    store = WebSchedulerStore()
    indexed_map = store.get_indexed_pages_map(workspace_name, domain_or_prefix=domain)
    already_indexed_urls = set(indexed_map.keys())
    already_indexed_count = len(already_indexed_urls)

    # Classify unindexed pages
    new_section_urls = [u for u in disc.get("section_urls", []) if u not in already_indexed_urls]
    new_domain_urls = [u for u in disc.get("domain_urls", []) if u not in already_indexed_urls]
    new_section_count = len(new_section_urls)
    new_domain_count = len(new_domain_urls)

    report = format_crawler_discovery_report(
        title=title,
        start_url=start_url,
        section_count=section_count,
        domain_count=domain_count,
        already_indexed_count=already_indexed_count,
        new_section_count=new_section_count,
        new_domain_count=new_domain_count,
        has_sitemap=has_sitemap
    )
    print(report)

    scope_name = "Single Page"
    force_refresh = False

    if domain_count == 1:
        chosen_urls = [start_url]
    else:
        choices = []

        if already_indexed_count > 0:
            if new_section_count > 0:
                choices.append(f"1. ⚡ Incremental Section Ingestion ({new_section_count} NEW pages) [Recommended]")
            if new_domain_count > 0:
                choices.append(f"2. 🚀 Quick Incremental Crawl (Next {min(50, new_domain_count)} NEW pages) ~ 5s")
                if new_domain_count > 50:
                    choices.append(f"3. 🌐 Deep Incremental Crawl (Next {min(250, new_domain_count)} NEW pages) ~ 20s")
                if new_domain_count > 250:
                    choices.append(f"4. 📦 Extensive Incremental Crawl (Next {min(500, new_domain_count)} NEW pages) ~ 45s")
                choices.append(f"5. 🌌 Ingest All Remaining Domain Pages ({new_domain_count} NEW pages)")
            choices.append(f"6. 🔄 Full Re-Sync & Refresh (Re-check all {domain_count} pages with SHA-256)")
            choices.append(f"7. 📄 Ingest / Refresh Landing Page Only (1 page) ~ 1s")
            choices.append("❌ Cancel")
        else:
            if section_count > 1 and section_count != domain_count:
                choices.append(f"1. 📄 Current Section Only ({section_count} pages) [Recommended]")

            if domain_count > 50:
                choices.append(f"2. ⚡ Fast Crawl Limit (Top 50 pages) ~ 5s")
            if domain_count > 250:
                choices.append(f"3. 🚀 Deep Crawl Limit (Top 250 pages) ~ 20s")
            if domain_count > 500:
                choices.append(f"4. 📦 Extensive Crawl Limit (Top 500 pages) ~ 45s")

            choices.append(f"5. 🌐 Entire Discovered Domain ({domain_count} pages)")
            choices.append(f"6. 📄 Single Start Page Only (1 page) ~ 1s")
            choices.append("❌ Cancel")

        choice = questionary.select(
            f"Select indexing scope for workspace '{workspace_name}':",
            choices=choices
        ).ask()

        if not choice or choice.startswith("❌"):
            print("Operation cancelled.\n")
            return False

        if "Incremental Section Ingestion" in choice:
            chosen_urls = new_section_urls
            scope_name = f"Incremental Section (+{len(chosen_urls)} pages)"
        elif "Quick Incremental Crawl" in choice:
            chosen_urls = new_domain_urls[:50]
            scope_name = f"Incremental Top 50 (+{len(chosen_urls)} pages)"
        elif "Deep Incremental Crawl" in choice:
            chosen_urls = new_domain_urls[:250]
            scope_name = f"Incremental Top 250 (+{len(chosen_urls)} pages)"
        elif "Extensive Incremental Crawl" in choice:
            chosen_urls = new_domain_urls[:500]
            scope_name = f"Incremental Top 500 (+{len(chosen_urls)} pages)"
        elif "Ingest All Remaining" in choice:
            chosen_urls = new_domain_urls
            scope_name = f"Remaining Domain (+{len(chosen_urls)} pages)"
        elif "Full Re-Sync" in choice:
            chosen_urls = disc["domain_urls"]
            force_refresh = True
            scope_name = f"Full Re-Sync ({len(chosen_urls)} pages)"
        elif "Current Section Only" in choice:
            chosen_urls = disc["section_urls"]
            scope_name = f"Section ({len(chosen_urls)} pages)"
        elif "Fast Crawl Limit" in choice or "Top 50 pages" in choice:
            chosen_urls = disc["domain_urls"][:50]
            scope_name = "Top 50 pages"
        elif "Deep Crawl Limit" in choice or "Top 250 pages" in choice:
            chosen_urls = disc["domain_urls"][:250]
            scope_name = "Top 250 pages"
        elif "Extensive Crawl Limit" in choice or "Top 500 pages" in choice:
            chosen_urls = disc["domain_urls"][:500]
            scope_name = "Top 500 pages"
        elif "Entire Discovered Domain" in choice:
            chosen_urls = disc["domain_urls"]
            scope_name = f"Domain ({len(chosen_urls)} pages)"
        else:
            chosen_urls = [start_url]

    total_target = len(chosen_urls)
    print(f"\n🚀 Processing and indexing \033[92m{total_target}\033[0m web pages into workspace '\033[93m{workspace_name}\033[0m'...")

    SPINNER_FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]

    def _render_crawl_progress(current: int, total: int, indexed: int, skipped: int, latest_url: str = "", latest_title: str = ""):
        pct = int((current / total) * 100) if total else 100
        bar_len = 14
        filled = int((pct / 100) * bar_len)
        bar = "█" * filled + "░" * (bar_len - filled)
        frame = SPINNER_FRAMES[current % len(SPINNER_FRAMES)]

        display_url = latest_url
        if len(display_url) > 30:
            display_url = display_url[:12] + "..." + display_url[-15:]

        status_text = f"{indexed} new"
        if skipped > 0:
            status_text += f", {skipped} cached"

        safe_stdout_write(f"\r\033[K\033[96m{frame}\033[0m [1/2 Crawling] [{bar}] {current}/{total} ({pct}%) • \033[93m{status_text}\033[0m • \033[90m{display_url}\033[0m")

    def _render_embed_progress(current: int, total: int):
        pct = int((current / total) * 100) if total else 100
        bar_len = 14
        filled = int((pct / 100) * bar_len)
        bar = "█" * filled + "░" * (bar_len - filled)
        frame = SPINNER_FRAMES[current % len(SPINNER_FRAMES)]

        safe_stdout_write(f"\r\033[K\033[95m{frame}\033[0m [2/2 Embedding] [{bar}] {current}/{total} pages ({pct}%) • \033[92mVector Knowledge Base\033[0m")

    # Hide terminal cursor during active live progress ticks
    safe_stdout_write("\033[?25l")
    try:
        res = crawl_and_index_urls(
            workspace_name=workspace_name,
            urls=chosen_urls,
            root_url=start_url,
            root_title=title,
            scope=scope_name,
            force_refresh=force_refresh,
            max_workers=20,
            progress_callback=_render_crawl_progress,
            embed_progress_callback=_render_embed_progress,
            sitemap_lastmods=disc.get("sitemap_lastmods")
        )
    finally:
        # Restore terminal cursor visibility
        safe_stdout_write("\033[?25h")

    # Completely clear the live ticker line and print a clean final summary
    safe_stdout_write("\r\033[K")
    indexed_cnt = res.get("indexed_count", 0)
    skipped_cnt = res.get("skipped_count", 0)
    total_distinct = res.get("total_distinct_indexed", indexed_cnt + skipped_cnt)
    total_chars = res.get("total_chars", 0)

    if res.get("status") == "partial_error":
        safe_stdout_write(f"⚠️ Partial indexing completed: \033[92m{indexed_cnt}\033[0m pages indexed ({skipped_cnt} cached), but encountered error: \033[91m{res.get('error')}\033[0m\n\n")
    elif indexed_cnt == 0 and skipped_cnt > 0:
        safe_stdout_write(f"✔ All \033[96m{skipped_cnt}\033[0m web pages from \033[96m{start_url}\033[0m are already up-to-date in workspace '\033[93m{workspace_name}\033[0m' (SHA-256 verified, 0 embeddings consumed). Total in knowledge base: \033[92m{total_distinct}\033[0m pages.\n\n")
    elif indexed_cnt > 0 and skipped_cnt > 0:
        safe_stdout_write(f"✔ Successfully ingested \033[92m{indexed_cnt}\033[0m new/updated web pages ({total_chars:,} chars) from \033[96m{start_url}\033[0m into workspace '\033[93m{workspace_name}\033[0m' (\033[90m{skipped_cnt} unchanged pages cached\033[0m). Total in knowledge base: \033[92m{total_distinct}\033[0m pages!\n\n")
    else:
        safe_stdout_write(f"✔ Successfully ingested and indexed \033[92m{indexed_cnt}\033[0m web pages ({total_chars:,} chars) from \033[96m{start_url}\033[0m into workspace '\033[93m{workspace_name}\033[0m'! Total in knowledge base: \033[92m{total_distinct}\033[0m pages.\n\n")

    if res.get("is_dynamic_spa"):
        safe_stdout_write(
            f"⚠️ \033[1;93mImportante:\033[0m\n"
            f"Este site carrega seu conteúdo de forma dinâmica no navegador. Apenas a estrutura\n"
            f"estática foi capturada. Para consultar detalhes específicos, adicione o link\n"
            f"direto da página via '\033[96m/web add <url>\033[0m'.\n"
            f"\033[90m[Nota técnica: Client-Side Rendering (CSR / SPA) detectado no domínio {domain}]\033[0m\n\n"
        )

    return True


from any_context.help.manager import (
    display_help_page,
    show_interactive_help_menu,
    handle_command_help_interception
)

