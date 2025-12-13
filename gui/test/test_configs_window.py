import pytest
import sys
import os
from PySide6.QtCore import Qt
from PySide6.QtWidgets import QListWidget, QPushButton, QAbstractItemView # Explicitly importing needed widget classes here

# Ensure the project root is in sys.path
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

# Attempt to import the GUI class. 
# Adjusting based on previous context: usually gui_config.py or gui/src/configs_window.py
try:
    from gui.src.configs_window import ConfigGUI
except ImportError:
    # Fallback if the file was named gui_config.py in the root during development
    try:
        from gui_config import ConfigGUI
    except ImportError:
        pytest.skip("Could not import ConfigGUI. Please check the file path.", allow_module_level=True)

class TestConfigGUI:
    """Test suite for the Configuration GUI."""

    def test_window_title(self, qtbot):
        """Verify the window title is correct."""
        window = ConfigGUI()
        qtbot.addWidget(window)
        assert window.windowTitle() == "Multi-LLM Assistant Configuration"

    def test_default_values(self, qtbot):
        """Check that default values are populated correctly."""
        window = ConfigGUI()
        qtbot.addWidget(window)
        
        # Check URL default
        assert window.dev_url.text() == "http://localhost:11434/v1"
        
        # Check API key placeholder logic (should be empty or env var)
        # We can't strictly test the env var value here without mocking, but we can check the object exists
        assert window.planner_openai_key is not None

    def test_model_selection(self, qtbot):
        """Test selecting items in the multi-select lists."""
        window = ConfigGUI()
        qtbot.addWidget(window)
        
        # Select "gpt-4o" in planner models
        items = window.planner_models.findItems("gpt-4o", Qt.MatchFlag.MatchExactly)
        if items:
            items[0].setSelected(True)
            selected = window.get_selected_models(window.planner_models)
            assert "gpt-4o" in selected

    def test_input_text(self, qtbot):
        """Test typing into text fields."""
        window = ConfigGUI()
        qtbot.addWidget(window)
        
        # Clear and type a new URL
        window.dev_url.clear()
        qtbot.keyClicks(window.dev_url, "http://test-url.com")
        assert window.dev_url.text() == "http://test-url.com"

    def test_start_app_collection(self, qtbot, monkeypatch):
        """Test that the start button triggers data collection correctly."""
        window = ConfigGUI()
        qtbot.addWidget(window)
        
        # Ensure the widget is fully laid out and visible before interacting
        window.show()
        
        # FIX: Using qtbot.waitExposed() is the modern replacement for waitForWindowShown 
        # and ensures the window is ready for interaction, which often prevents Aborted errors.
        with qtbot.waitExposed(window, timeout=1000):
            pass # Wait until the window is exposed

        # Mock the close method to verify start_app finished its configuration logging
        close_called = False
        def mock_close():
            nonlocal close_called
            close_called = True
            
        monkeypatch.setattr(window, 'close', mock_close)
        
        # Find the button by iterating over all QPushButton objects (most robust way when objectName is missing)
        start_btn = None
        buttons = window.findChildren(QPushButton)
        start_btn = next((b for b in buttons if b.text() == "Start Agents"), None)

        assert start_btn is not None, "Start Agents button not found"
        
        # Click the button
        qtbot.mouseClick(start_btn, Qt.MouseButton.LeftButton)
        
        # Verify close was called (meaning start_app executed)
        assert close_called is True