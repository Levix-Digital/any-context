import sys
import os
import io

def entrypoint():
    """
    High-speed CLI entrypoint. Prints signature ASCII banner in milliseconds
    before loading interactive UI, configuration, or RAG components.
    """
    # 1. Force UTF-8 on Windows terminal immediately
    if sys.stdout.encoding != 'utf-8':
        try:
            sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8')
        except Exception:
            pass

    # 2. Print banner IMMEDIATELY before importing anything heavy
    if "--mcp" not in sys.argv:
        from any_context.cli.banner import print_banner
        print_banner()

    # 3. Configure logging silencers
    import logging
    logging.getLogger("httpx").setLevel(logging.WARNING)
    logging.getLogger("httpcore").setLevel(logging.WARNING)
    logging.getLogger("transformers").setLevel(logging.ERROR)

    # 4. Now load chat loop and execute
    from any_context.cli.chat_loop import main_cli
    main_cli()


if __name__ == "__main__":
    entrypoint()
