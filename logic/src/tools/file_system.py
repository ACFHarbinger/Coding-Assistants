import os


class FileTools:
    """
    A toolset for LLM agents to interact with the local file system.
    This allows agents to explore the codebase, read files, and write changes.
    """

    def __init__(self, root_dir: str = "."):
        self.root_dir = os.path.abspath(root_dir)

    def _is_safe_path(self, filepath: str) -> bool:
        """Ensure the path is within the root directory to prevent directory traversal."""
        # Normalize the path and ensure it starts with the root directory path
        abs_path = os.path.abspath(os.path.join(self.root_dir, filepath))
        return abs_path.startswith(self.root_dir)

    def list_files(self, directory: str = ".", recursive: bool = False) -> str:
        """
        List files in a directory.
        
        Args:
            directory: The subdirectory to list (default is root).
            recursive: If true, lists files recursively (useful for overview).
        """
        if not self._is_safe_path(directory):
            return "Error: Access denied. Path is outside the project root."

        target_dir = os.path.abspath(os.path.join(self.root_dir, directory))
        if not os.path.exists(target_dir):
            return f"Error: Directory '{directory}' does not exist."

        file_list = []
        
        if recursive:
            # Use os.walk for recursive listing
            for root, dirs, files in os.walk(target_dir):
                # Ignore hidden directories like .git, .pytest_cache
                dirs[:] = [d for d in dirs if not d.startswith('.')]
                
                for file in files:
                    if file.startswith('.'): continue # Ignore hidden files
                    
                    full_path = os.path.join(root, file)
                    # Use relpath to get the path relative to the root_dir (or target_dir if preferred)
                    # Here we list relative to the requested directory to match "ls" behavior essentially
                    # But usually relative to project root is safer/clearer for agents.
                    # The test expects simple filenames for root files, so relative to target_dir is best if target_dir != root
                    
                    # Logic: If I ask to list ".", I want "file1.txt". 
                    # If I ask to list "subdir", I want "file3.md" (if recursive within subdir).
                    
                    rel_path = os.path.relpath(full_path, target_dir)
                    if rel_path == ".": continue 
                    file_list.append(rel_path)
        else:
            # Use os.listdir for flat listing
            try:
                for item in os.listdir(target_dir):
                    item_path = os.path.join(target_dir, item)
                    if os.path.isfile(item_path) or os.path.isdir(item_path):
                        if not item.startswith('.'):
                            file_list.append(item)
            except Exception as e:
                return f"Error listing directory: {str(e)}"

        # Sort the list for consistent output in tests
        file_list.sort()
        return "\n".join(file_list) if file_list else "(No files found)"

    def read_file(self, filepath: str) -> str:
        """Read the contents of a specific file."""
        if not self._is_safe_path(filepath):
            return "Error: Access denied. Path is outside the project root."

        full_path = os.path.join(self.root_dir, filepath)
        if not os.path.exists(full_path):
            return f"Error: File '{filepath}' does not exist."

        try:
            with open(full_path, 'r', encoding='utf-8') as f:
                content = f.read()
            return content
        except Exception as e:
            return f"Error reading file: {str(e)}"

    def write_file(self, filepath: str, content: str) -> str:
        """
        Write content to a file. Overwrites if it exists, creates if it doesn't.
        """
        if not self._is_safe_path(filepath):
            return "Error: Access denied. Path is outside the project root."

        full_path = os.path.join(self.root_dir, filepath)
        
        try:
            # Create necessary directories
            os.makedirs(os.path.dirname(full_path), exist_ok=True)
            with open(full_path, 'w', encoding='utf-8') as f:
                f.write(content)
            return f"Success: File '{filepath}' written."
        except Exception as e:
            return f"Error writing file: {str(e)}"

    def search_files(self, keyword: str, directory: str = ".") -> str:
        """
        Search for a keyword in all files within a directory (grep-style).
        """
        if not self._is_safe_path(directory):
            return "Error: Access denied."
            
        target_dir = os.path.abspath(os.path.join(self.root_dir, directory))
        results = []
        
        for root, _, files in os.walk(target_dir):
            for file in files:
                if file.startswith('.'): continue
                if file.endswith(('.pyc', '.git', '.png', '.jpg')): continue # Skip binary/system files
                
                full_path = os.path.join(root, file)
                rel_path = os.path.relpath(full_path, self.root_dir)
                
                try:
                    with open(full_path, 'r', encoding='utf-8', errors='ignore') as f:
                        for i, line in enumerate(f, 1):
                            if keyword in line:
                                results.append(f"{rel_path}:{i}: {line.strip()[:100]}")
                except Exception:
                    continue
                    
        return "\n".join(results) if results else "No matches found."