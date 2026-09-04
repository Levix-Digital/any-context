"""
AnyContext Server Package - REST API & MCP Integration Engine
Provides lazy access to REST API and MCP protocol servers, avoiding heavy
FastAPI / LlamaIndex / LanceDB initialization during lightweight RPC or CLI startup.
"""

def create_app(*args, **kwargs):
    from any_context.server.api import create_app as _create_app
    return _create_app(*args, **kwargs)

def start_api_server(*args, **kwargs):
    from any_context.server.api import start_api_server as _start_api_server
    return _start_api_server(*args, **kwargs)

__all__ = ["create_app", "start_api_server"]
