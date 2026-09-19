"""
Unit tests for /inspect --full and chunk size reporting.
"""
from any_context.commands.dispatcher import dispatch_command
from unittest.mock import patch, MagicMock


def test_inspect_full_command_output():
    # Mocking store so test runs isolated from local database state
    with patch("any_context.vector_engine.store.LanceDBStore.get_instance") as mock_get_store:
        mock_store = MagicMock()
        mock_get_store.return_value = mock_store
        mock_store.count_records.return_value = 1
        
        mock_tbl = MagicMock()
        mock_store.get_table.return_value = mock_tbl
        
        # Mock Arrow select for taxonomy
        mock_search_tax = MagicMock()
        mock_search_tax.where.return_value = mock_search_tax
        mock_search_tax.select.return_value = mock_search_tax
        mock_search_tax.limit.return_value = mock_search_tax
        mock_arrow = MagicMock()
        mock_arrow.num_rows = 1
        mock_col = MagicMock()
        mock_col.to_pylist.return_value = ["PDF Document (Digital Layout)"]
        mock_arrow.column.return_value = mock_col
        mock_search_tax.to_arrow.return_value = mock_arrow
        
        # Mock search for raw_samples
        mock_search_samples = MagicMock()
        mock_search_samples.where.return_value = mock_search_samples
        mock_search_samples.limit.return_value = mock_search_samples
        mock_search_samples.to_list.return_value = [
            {
                "file_name": "I.CMR_ONE_PICKUP.pdf",
                "content_type": "PDF Document (Digital Layout)",
                "text": "// Context: I.CMR_ONE_PICKUP.pdf > Page 1\n| IKEA CALGARY | BISON TRANSPORT INC. |"
            }
        ]
        
        mock_tbl.search.side_effect = [mock_search_tax, mock_search_samples]
        
        # 1. Test Standard view
        res = dispatch_command("/inspect CMR 1", active_workspace="TestWS")
        assert res.success is True
        assert "Size:" in res.message
        assert "characters" in res.message
        assert "Preview:" in res.message
        assert "Run `/inspect <filter> --full`" in res.message
        
        # Reset side effect for second call
        mock_tbl.search.side_effect = [mock_search_tax, mock_search_samples]
        
        # 2. Test Full view
        res_full = dispatch_command("/inspect CMR 1 --full", active_workspace="TestWS")
        assert res_full.success is True
        assert "Size:" in res_full.message
        assert "Full Content:" in res_full.message
        assert "| IKEA CALGARY | BISON TRANSPORT INC. |" in res_full.message
