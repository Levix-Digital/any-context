from any_context import __version__

def safe_print(msg: str):
    try:
        print(msg)
    except UnicodeEncodeError:
        print(msg.encode("ascii", errors="ignore").decode("ascii"))

def print_banner():
    """
    Displays the signature ASCII art banner and branding info for AnyContext CLI
    """
    banner_art = r"""
  ___              ____ ___  _   _ _____ _____ _  _______ 
 / _ \ _ __  _   _/ ___/ _ \| \ | |_   _| ____\ \/ /_   _|
| |_| | '_ \| | | | |  | | | |  \| | | | |  _|  \  /  | |  
|  _  | | | | |_| | |__| |_| | |\  | | | | |___ /  \  | |  
|_| |_|_| |_|\__, |\____\___/|_| \_| |_| |_____/_/\_\ |_|  
             |___/                                          
"""
    cyan = "\033[96m"
    yellow = "\033[93m"
    magenta = "\033[95m"
    bold = "\033[1m"
    gray = "\033[90m"
    reset = "\033[0m"

    safe_print(f"{cyan}{banner_art}{reset}")
    safe_print(f"{bold}{yellow}  🚀 AnyContext {gray}(actx){yellow} v{__version__}{reset}  |  {bold}{magenta}Levix Digital{reset}")
    safe_print(f"{gray}  ⚡ Transform any file, drive, or folder into a living, real-time AI context.{reset}")
    safe_print(f"{gray}  🔒 100% Local & Offline-First Privacy{reset}\n")
