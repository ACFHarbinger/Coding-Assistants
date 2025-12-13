import sys
import pytest

from pathlib import Path

# The project root is THREE levels up from conftest.py:
project_root = Path(__file__).resolve().parent.parent.parent

# Add the project root to sys.path. This allows 'import logic.src...' 
sys.path.insert(0, str(project_root))

from logic.src.tools.file_system import FileTools


@pytest.fixture
def workspace(tmp_path):
    """
    Creates a temporary directory for file operations.
    Returns the FileTools instance initialized with this directory.
    """
    # tmp_path is a built-in pytest fixture that returns a pathlib.Path object
    temp_dir = tmp_path / "test_workspace"
    temp_dir.mkdir()
    
    tools = FileTools(root_dir=str(temp_dir))
    return tools, temp_dir