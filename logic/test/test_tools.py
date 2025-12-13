import os


class TestFileTools:
    def test_write_and_read_file(self, workspace):
        tools, _ = workspace
        filename = "test_file.txt"
        content = "Hello, World!"
        
        # 1. Test Write
        result_write = tools.write_file(filename, content)
        assert "Success" in result_write
        
        # 2. Test Read
        result_read = tools.read_file(filename)
        assert result_read == content

    def test_list_files(self, workspace):
        tools, temp_dir = workspace
        
        # Create dummy files manually to test listing
        (temp_dir / "file1.txt").touch()
        (temp_dir / "file2.py").touch()
        os.makedirs(temp_dir / "subdir")
        (temp_dir / "subdir" / "file3.md").touch()
        
        # Test flat list
        files = tools.list_files()
        assert "file1.txt" in files
        assert "file2.py" in files
        
        # Test recursive list
        files_recursive = tools.list_files(recursive=True)
        assert "file1.txt" in files_recursive
        assert os.path.join("subdir", "file3.md") in files_recursive

    def test_search_files(self, workspace):
        tools, _ = workspace
        
        # Write files with specific content
        tools.write_file("script.py", "def my_func():\n    print('magic_keyword')")
        tools.write_file("notes.txt", "This is just a note.")
        
        # Test Search
        result = tools.search_files("magic_keyword")
        assert "script.py" in result
        assert "magic_keyword" in result
        
        # Test Search No Match
        result_empty = tools.search_files("nonexistent_term")
        assert "No matches found" in result_empty

    def test_security_directory_traversal(self, workspace):
        """
        Critical: Ensure agents cannot access files outside the root directory.
        """
        tools, _ = workspace
        
        # Attempt to read a system file or file outside root
        # Note: We use ../ to try and escape the temp dir
        unsafe_path = "../../../../../etc/passwd" 
        result = tools.read_file(unsafe_path)
        assert "Error: Access denied" in result
        
        # Attempt to list files outside root
        result_list = tools.list_files("../")
        assert "Error: Access denied" in result_list

    def test_overwrite_protection_behavior(self, workspace):
        """
        Test that write_file overwrites existing content (standard behavior for this tool).
        """
        tools, _ = workspace
        filename = "overwrite_test.txt"
        
        tools.write_file(filename, "Original Content")
        tools.write_file(filename, "New Content")
        
        content = tools.read_file(filename)
        assert content == "New Content"

    def test_nonexistent_file(self, workspace):
        tools, _ = workspace
        result = tools.read_file("fake_ghost_file.txt")
        assert "Error" in result
        assert "does not exist" in result