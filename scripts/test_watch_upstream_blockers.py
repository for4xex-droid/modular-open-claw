import unittest
from unittest.mock import patch, MagicMock
import sys
import os

# To import the script we will create
sys.path.append(os.path.dirname(os.path.abspath(__file__)))

try:
    import watch_upstream_blockers
except ImportError:
    watch_upstream_blockers = None

class TestWatchUpstreamBlockers(unittest.TestCase):
    def test_module_exists(self):
        """Test that the script exists and can be imported."""
        self.assertIsNotNone(watch_upstream_blockers, "watch_upstream_blockers.py does not exist")

    @patch('watch_upstream_blockers.urllib.request.urlopen')
    def test_check_crate_version_blocked(self, mock_urlopen):
        """Test when the crate has NOT reached the target version (still blocked)."""
        if not watch_upstream_blockers:
            self.skipTest("Module not implemented yet")
            
        mock_response = MagicMock()
        mock_response.read.return_value = b'{"crate": {"max_stable_version": "0.12.5"}}'
        mock_response.__enter__.return_value = mock_response
        mock_urlopen.return_value = mock_response
        
        result = watch_upstream_blockers.check_crate_target("serenity", "0.13.0")
        self.assertFalse(result["reached"], "Should return false for 0.12.5 < 0.13.0")
        self.assertEqual(result["current_version"], "0.12.5")

    @patch('watch_upstream_blockers.urllib.request.urlopen')
    def test_check_crate_version_released(self, mock_urlopen):
        """Test when the crate has reached or exceeded the target version (unblocked)."""
        if not watch_upstream_blockers:
            self.skipTest("Module not implemented yet")
            
        mock_response = MagicMock()
        mock_response.read.return_value = b'{"crate": {"max_stable_version": "0.13.1"}}'
        mock_response.__enter__.return_value = mock_response
        mock_urlopen.return_value = mock_response
        
        result = watch_upstream_blockers.check_crate_target("serenity", "0.13.0")
        self.assertTrue(result["reached"], "Should return true for 0.13.1 >= 0.13.0")
        self.assertEqual(result["current_version"], "0.13.1")

if __name__ == '__main__':
    unittest.main()
