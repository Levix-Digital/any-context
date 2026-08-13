import sys
import questionary
from typing import Optional
from any_context.help.models import HelpPage
from any_context.help.registry import HELP_REGISTRY, get_help_page

def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))

def display_help_page(page: HelpPage):
    """Prints a beautifully formatted, rich help manual page for a command."""
    safe_print(f"\n================================================================================")
    safe_print(f"\033[93m{page.title}\033[0m")
    safe_print(f"================================================================================\n")
    
    safe_print(f"\033[1m📖 DESCRIPTION:\033[0m")
    safe_print(f"  {page.description}\n")

    safe_print(f"\033[1m⚡ SYNTAX / USAGE:\033[0m")
    safe_print(f"  \033[96m{page.syntax}\033[0m\n")

    if page.parameters:
        safe_print(f"\033[1m⚙️ PARAMETERS & OPTIONS:\033[0m")
        for param in page.parameters:
            safe_print(f"  • {param}")
        safe_print("")

    if page.examples:
        safe_print(f"\033[1m💡 EXAMPLES:\033[0m")
        for example in page.examples:
            safe_print(f"  \033[92m$ {example}\033[0m")
        safe_print("")

    if page.tips:
        safe_print(f"\033[1m📌 TIPS & BEST PRACTICES:\033[0m")
        for tip in page.tips:
            safe_print(f"  • {tip}")
        safe_print("")

    safe_print(f"================================================================================\n")

def show_interactive_help_menu():
    """Displays an interactive Questionary menu listing all command documentation topics."""
    while True:
        choices = [
            "📂 /switch (Workspace Switching & DB Synchronization)",
            "⚙️ /config (Configuration Menu, AI Models & RBAC)",
            "🔐 /auth & /login (User Accounts, Access Control & Bearer Tokens)",
            "🌐 --serve (REST API Server & Enterprise VPC Deploy)",
            "🔌 --mcp (Model Context Protocol for Claude & Cursor)",
            "🔄 /update (Auto-Updater Engine)",
            "🧹 /reset-memory (Purge Long-Term Vector Memory)",
            "💥 /factory-reset (Complete Factory Reset)",
            "🔙 Return to Chat"
        ]

        safe_print("\n\033[93m📖 AnyContext Interactive Manual & Documentation Index\033[0m")
        choice = questionary.select(
            "Select a command topic to view detailed documentation and usage examples:",
            choices=choices
        ).ask()

        if not choice or choice.startswith("🔙"):
            break

        if choice.startswith("📂"):
            page = get_help_page("switch")
        elif choice.startswith("⚙️"):
            page = get_help_page("config")
        elif choice.startswith("🔐"):
            page = get_help_page("auth")
        elif choice.startswith("🌐"):
            page = get_help_page("serve")
        elif choice.startswith("🔌"):
            page = get_help_page("mcp")
        elif choice.startswith("🔄"):
            page = get_help_page("update")
        elif choice.startswith("🧹"):
            page = get_help_page("reset-memory")
        elif choice.startswith("💥"):
            page = get_help_page("factory-reset")
        else:
            page = None

        if page:
            display_help_page(page)

def handle_command_help_interception(user_input: str) -> bool:
    """
    Intercepts user inputs ending with --help, -h, /help, or /h, or exact /help commands.
    Displays dedicated help page or opens interactive help index. Returns True if intercepted.
    """
    raw_input = user_input.strip()
    clean_input = raw_input.lower()

    if not clean_input:
        return False

    # Case 1: Exact /help or /h or help
    if clean_input in ["/help", "/h", "help", "--help", "-h"]:
        show_interactive_help_menu()
        return True

    # Case 2: Subcommand help requested (e.g. '/switch --help', '/config -h', 'actx serve --help', '/login /h')
    parts = clean_input.split()
    if len(parts) >= 2:
        last_arg = parts[-1]
        if last_arg in ["--help", "-h", "/help", "/h", "help"]:
            target_cmd = parts[0]
            page = get_help_page(target_cmd)
            if page:
                display_help_page(page)
                return True
            else:
                show_interactive_help_menu()
                return True

    return False
