"""
AnyContext Help Page Display & Menu Manager.
Encapsulates rich command manual formatting, interactive questionary help menus,
and CLI command interception for help requests.
"""
import sys
import questionary
from typing import Optional
from any_context.help.models import HelpPage
from any_context.help.registry import HELP_REGISTRY, get_help_page


def safe_print(msg: str):
    """Safely prints strings handling legacy Windows encoding errors."""
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))


def display_help_page(page: HelpPage):
    """Prints a beautifully formatted, rich help manual page for a command."""
    safe_print("\n================================================================================")
    safe_print(f"\033[93m{page.title}\033[0m")
    safe_print("================================================================================\n")
    
    safe_print("\033[1m📖 DESCRIPTION:\033[0m")
    safe_print(f"  {page.description}\n")

    safe_print("\033[1m⚡ SYNTAX / USAGE:\033[0m")
    safe_print(f"  \033[96m{page.syntax}\033[0m\n")

    if page.parameters:
        safe_print("\033[1m⚙️ PARAMETERS & OPTIONS:\033[0m")
        for param in page.parameters:
            safe_print(f"  • {param}")
        safe_print("")

    if page.examples:
        safe_print("\033[1m💡 EXAMPLES:\033[0m")
        for example in page.examples:
            safe_print(f"  \033[92m$ {example}\033[0m")
        safe_print("")

    if page.tips:
        safe_print("\033[1m📌 TIPS & BEST PRACTICES:\033[0m")
        for tip in page.tips:
            safe_print(f"  • {tip}")
        safe_print("")

    safe_print("================================================================================\n")


def show_interactive_help_menu():
    """Displays an interactive Questionary menu listing all command documentation topics dynamically."""
    while True:
        choices_map = {}
        for key, page in HELP_REGISTRY.items():
            label = f"{page.title} ({page.command})"
            choices_map[label] = page

        sorted_labels = sorted(list(choices_map.keys()))
        sorted_labels.append("🔙 Return to Chat")

        safe_print("\n\033[93m📖 AnyContext Interactive Manual & Documentation Index (28 Topics)\033[0m")
        choice = questionary.select(
            "Select a command topic to view detailed documentation and usage examples:",
            choices=sorted_labels
        ).ask()

        if not choice or choice.startswith("🔙"):
            break

        selected_page = choices_map.get(choice)
        if selected_page:
            display_help_page(selected_page)


def handle_command_help_interception(user_input: str) -> bool:
    """
    Intercepts user inputs for help commands in both prefix and suffix styles:
      - Prefix: '/help density', 'help switch', 'actx --help density', 'actx help mcp'
      - Suffix: '/density --help', '/density -h', 'actx serve --help', '/sync /h'
      - Exact:  '/help', 'help', '--help', '-h', '/keys', '/api-keys'
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

    # Case 2: Exact /api-keys or /keys command
    if clean_input in ["/keys", "/api-keys", "/apikeys", "actx --keys", "actx --api-keys", "--keys", "--api-keys"]:
        page = get_help_page("api-keys")
        if page:
            display_help_page(page)
            return True

    parts = clean_input.split()

    # Case 3: Leading 'actx' CLI command prefix (e.g. 'actx --help density', 'actx help sync')
    if parts[0] in ["actx", "anycontext", "any-context", "ac"] and len(parts) >= 2:
        parts = parts[1:]

    # Case 4: Prefix help syntax (e.g. '/help density', 'help switch', '--help sources', '-h model')
    if parts[0] in ["/help", "help", "--help", "-h", "/h"] and len(parts) >= 2:
        target_topic = " ".join(parts[1:]).strip()
        page = get_help_page(target_topic)
        if page:
            display_help_page(page)
            return True
        else:
            safe_print(f"\n⚠️ Command or topic '\033[93m{target_topic}\033[0m' not found in Help manual.")
            show_interactive_help_menu()
            return True

    # Case 5: Suffix help flag (e.g. '/switch --help', '/config -h', 'serve --help', '/login /h')
    if len(parts) >= 2 and parts[-1] in ["--help", "-h", "/help", "/h", "help"]:
        target_topic = " ".join(parts[:-1]).strip()
        page = get_help_page(target_topic)
        if page:
            display_help_page(page)
            return True
        else:
            page_single = get_help_page(parts[0])
            if page_single:
                display_help_page(page_single)
                return True
            show_interactive_help_menu()
            return True

    return False
