import os
import pytest
from any_context.config.db_store import ConfigDBStore
from any_context.core.services.model_service import ModelService
from any_context.commands.dispatcher import CommandDispatcher

@pytest.fixture
def test_db_store(tmp_path):
    db_file = str(tmp_path / "test_settings.db")
    store = ConfigDBStore(db_path=db_file)
    return store

def test_default_workspaces_have_gpt_4o_mini(test_db_store):
    assert test_db_store.get_workspace_model("Default") == "gpt-4o-mini"
    assert test_db_store.get_workspace_model("Shared Sources") == "gpt-4o-mini"

def test_new_workspace_creation_strictly_defaults_to_gpt_4o_mini(test_db_store):
    test_db_store.add_workspace(name="ProjectAlpha", paths=[])
    assert test_db_store.get_workspace_model("ProjectAlpha") == "gpt-4o-mini"
    
    svc = ModelService(store=test_db_store)
    assert svc.get_current_model(workspace_name="ProjectAlpha") == "gpt-4o-mini"

def test_workspace_model_isolation_prevents_contamination(test_db_store):
    svc = ModelService(store=test_db_store)
    
    # 1. Create Workspace A and set model to gpt-4o
    test_db_store.add_workspace(name="WorkspaceA", paths=[])
    svc.set_model("gpt-4o", workspace_name="WorkspaceA")
    assert svc.get_current_model(workspace_name="WorkspaceA") == "gpt-4o"
    
    # 2. Create Workspace B -> must strictly be factory default gpt-4o-mini
    test_db_store.add_workspace(name="WorkspaceB", paths=[])
    assert svc.get_current_model(workspace_name="WorkspaceB") == "gpt-4o-mini"
    
    # 3. Switching with dispatcher returns gpt-4o-mini for new workspace
    dispatcher = CommandDispatcher(store=test_db_store)
    res = dispatcher.dispatch("/switch WorkspaceB", active_workspace="WorkspaceA")
    assert res.success is True
    assert res.state_updates.get("model") == "gpt-4o-mini"
