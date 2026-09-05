import sys


def safe_stdout_write(msg: str):
    """Writes message to stdout with fallback for Windows CP1252 / Charmap encoding."""
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


def format_session_error(error: Exception) -> str:
    """
    Translates raw runtime exceptions into a user-friendly, reassuring message,
    while discretely displaying technical details for troubleshooting.
    """
    err_type = type(error).__name__
    err_msg = str(error).strip()

    if isinstance(error, FileNotFoundError) or "no such file" in err_msg.lower():
        friendly_desc = "O arquivo ou diretório solicitado não foi encontrado."
        tip = "Verifique o caminho informado e tente novamente."
    elif isinstance(error, PermissionError) or "permission denied" in err_msg.lower():
        friendly_desc = "Permissão de acesso negada pelo sistema operacional."
        tip = "Verifique as permissões de leitura/escrita ou execute o terminal como administrador."
    elif "connection" in err_msg.lower() or "timeout" in err_msg.lower():
        friendly_desc = "Houve uma instabilidade temporária na conexão de rede."
        tip = "Verifique sua conexão com a internet e tente novamente."
    elif "decompress" in err_msg.lower() or "truncated stream" in err_msg.lower() or "error -5" in err_msg.lower() or "zlib" in err_msg.lower():
        friendly_desc = "Houve uma oscilação na resposta comprimida da rede ou no stream da API."
        tip = "O AnyContext recuperou e estabilizou sua sessão automaticamente. Basta reenviar sua mensagem."
    elif isinstance(error, (UnboundLocalError, NameError, AttributeError, TypeError, ValueError)):
        friendly_desc = "Ocorreu uma falha interna temporária ao processar esta ação."
        tip = "Sua sessão e seus dados continuam intactos. Tente executar o comando novamente ou digite '/help'."
    else:
        friendly_desc = "Ocorreu um erro inesperado ao executar esta ação."
        tip = "Sua sessão permanece ativa. Tente executar o comando novamente ou digite '/help'."

    return (
        f"\n\033[93m⚠️ Ops! Não foi possível concluir a ação:\033[0m\n"
        f"  • {friendly_desc}\n"
        f"  • \033[96mDica:\033[0m {tip}\n"
        f"  \033[90m[Nota técnica: {err_type} - {err_msg}]\033[0m\n\n"
    )
