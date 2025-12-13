import sys
import os
from PySide6.QtWidgets import (
    QApplication, 
    QWidget, 
    QVBoxLayout, 
    QHBoxLayout, 
    QLabel, 
    QLineEdit, 
    QPushButton, 
    QListWidget, # Ensuring QListWidget is clearly imported
    QFileDialog, 
    QTextEdit, 
    QGroupBox, 
    QFormLayout,
    QAbstractItemView
)
from PySide6.QtCore import Qt

class ConfigGUI(QWidget):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Multi-LLM Assistant Configuration")
        self.setMinimumWidth(700)
        self.setMinimumHeight(800)
        
        self.layout = QVBoxLayout()
        self.setLayout(self.layout)

        # Planner Config Group
        self.create_planner_group()
        
        # Developer Config Group
        self.create_developer_group()

        # Reviewer Config Group (NEW)
        self.create_reviewer_group()
        
        # Workspace Config Group
        self.create_workspace_group()

        # Task Input
        self.create_task_group()

        # Action Buttons
        self.create_buttons()

    def create_model_selector(self, models):
        """Helper to create a multi-select list widget"""
        list_widget = QListWidget()
        list_widget.addItems(models)
        list_widget.setSelectionMode(QAbstractItemView.SelectionMode.MultiSelection)
        list_widget.setMaximumHeight(80) # Keep it compact
        return list_widget

    def get_selected_models(self, list_widget):
        """Helper to get selected items from the list"""
        return [item.text() for item in list_widget.selectedItems()]

    def create_planner_group(self):
        group = QGroupBox("Planner Agent (Architecture)")
        layout = QFormLayout()
        
        # --- Model Selection (Multi-Select) ---
        self.planner_models = self.create_model_selector([
            "gpt-4o", "gpt-4-turbo", "claude-3-opus", 
            "gemini-2.5-pro", "gemini-2.5-flash", "gpt-3.5-turbo"
        ])

        # --- OpenAI Key ---
        self.planner_openai_key = QLineEdit()
        self.planner_openai_key.setEchoMode(QLineEdit.EchoMode.Password)
        self.planner_openai_key.setPlaceholderText("sk-...")
        if os.environ.get("OPENAI_API_KEY"):
            self.planner_openai_key.setText(os.environ.get("OPENAI_API_KEY"))

        # --- Gemini Key ---
        self.planner_gemini_key = QLineEdit()
        self.planner_gemini_key.setEchoMode(QLineEdit.EchoMode.Password)
        self.planner_gemini_key.setPlaceholderText("AIza...")
        if os.environ.get("GEMINI_API_KEY"):
            self.planner_gemini_key.setText(os.environ.get("GEMINI_API_KEY"))
            
        layout.addRow("Models (Select Multiple):", self.planner_models)
        layout.addRow("OpenAI Key:", self.planner_openai_key)
        layout.addRow("Gemini Key:", self.planner_gemini_key)
        group.setLayout(layout)
        self.layout.addWidget(group)

    def create_developer_group(self):
        group = QGroupBox("Developer Agent (Coding - Local/API)")
        layout = QFormLayout()
        
        # --- Model Selection (Multi-Select) ---
        self.dev_models = self.create_model_selector([
            "llama3.1", "qwen2.5-coder", "gemini-2.5-flash", "gpt-3.5-turbo", 
            "mistral", "deepseek-coder"
        ])
        
        # --- Local/Ollama URL ---
        self.dev_url = QLineEdit()
        self.dev_url.setText("http://localhost:11434/v1")
        self.dev_url.setPlaceholderText("http://localhost:11434/v1")
        
        # --- Local/Ollama API Key (generic) ---
        self.dev_api_key = QLineEdit()
        self.dev_api_key.setText("ollama")
        self.dev_api_key.setPlaceholderText("API Key (or 'ollama' for local)")
        
        # --- Dedicated Gemini Key for Developer ---
        self.dev_gemini_key = QLineEdit()
        self.dev_gemini_key.setEchoMode(QLineEdit.EchoMode.Password)
        self.dev_gemini_key.setPlaceholderText("AIza...")
        
        layout.addRow("Models (Select Multiple):", self.dev_models)
        layout.addRow("Local Base URL:", self.dev_url)
        layout.addRow("Local/OpenAI Key:", self.dev_api_key) 
        layout.addRow("Gemini Key:", self.dev_gemini_key) 
        group.setLayout(layout)
        self.layout.addWidget(group)

    def create_reviewer_group(self):
        """Creates the configuration group for the Reviewer role."""
        group = QGroupBox("Reviewer Agent (Code Review and Quality Assurance)")
        layout = QFormLayout()
        
        # --- Model Selection (Multi-Select) ---
        # Reviewers often need strong reasoning (like Planners) or specialized coding knowledge
        self.reviewer_models = self.create_model_selector([
            "gpt-4o", "gemini-2.5-pro", "llama3.1", "claude-3-opus",
            "deepseek-coder", "gpt-4-turbo"
        ])
        
        # --- OpenAI Key ---
        self.reviewer_openai_key = QLineEdit()
        self.reviewer_openai_key.setEchoMode(QLineEdit.EchoMode.Password)
        self.reviewer_openai_key.setPlaceholderText("sk-...") # Default empty, user can reuse planner key logic if needed
        
        # --- Gemini Key ---
        self.reviewer_gemini_key = QLineEdit()
        self.reviewer_gemini_key.setEchoMode(QLineEdit.EchoMode.Password)
        self.reviewer_gemini_key.setPlaceholderText("AIza...")

        layout.addRow("Models (Select Multiple):", self.reviewer_models)
        layout.addRow("OpenAI Key:", self.reviewer_openai_key)
        layout.addRow("Gemini Key:", self.reviewer_gemini_key)
        group.setLayout(layout)
        self.layout.addWidget(group)

    def create_workspace_group(self):
        group = QGroupBox("Workspace")
        layout = QHBoxLayout()
        
        self.work_dir_input = QLineEdit()
        self.work_dir_input.setText(os.getcwd())
        
        browse_btn = QPushButton("Browse...")
        browse_btn.clicked.connect(self.browse_folder)
        
        layout.addWidget(QLabel("Project Path:"))
        layout.addWidget(self.work_dir_input)
        layout.addWidget(browse_btn)
        
        group.setLayout(layout)
        self.layout.addWidget(group)

    def create_task_group(self):
        group = QGroupBox("Task Description")
        layout = QVBoxLayout()
        
        self.task_input = QTextEdit()
        self.task_input.setPlaceholderText("Describe what you want the agents to do with your codebase...")
        self.task_input.setMaximumHeight(100)
        
        layout.addWidget(self.task_input)
        group.setLayout(layout)
        self.layout.addWidget(group)

    def create_buttons(self):
        layout = QHBoxLayout()
        
        start_btn = QPushButton("Start Agents")
        start_btn.clicked.connect(self.start_app)
        start_btn.setStyleSheet("background-color: #4CAF50; color: white; font-weight: bold; padding: 8px;")
        
        cancel_btn = QPushButton("Cancel")
        cancel_btn.clicked.connect(self.close)
        
        layout.addStretch()
        layout.addWidget(cancel_btn)
        layout.addWidget(start_btn)
        
        self.layout.addLayout(layout)

    def browse_folder(self):
        folder = QFileDialog.getExistingDirectory(self, "Select Project Root")
        if folder:
            self.work_dir_input.setText(folder)

    def start_app(self):
        # Collecting all fields
        config = {
            # Planner
            "planner_models": self.get_selected_models(self.planner_models),
            "planner_openai_key": self.planner_openai_key.text(), 
            "planner_gemini_key": self.planner_gemini_key.text(), 
            
            # Developer
            "developer_models": self.get_selected_models(self.dev_models),
            "developer_base_url": self.dev_url.text(),
            "developer_api_key": self.dev_api_key.text(), 
            "developer_gemini_key": self.dev_gemini_key.text(), 

            # Reviewer (NEW)
            "reviewer_models": self.get_selected_models(self.reviewer_models),
            "reviewer_openai_key": self.reviewer_openai_key.text(),
            "reviewer_gemini_key": self.reviewer_gemini_key.text(),

            # Workspace/Task
            "work_dir": self.work_dir_input.text(),
            "task": self.task_input.toPlainText()
        }
        
        print("\n--- Starting Configuration ---")
        for k, v in config.items():
            # Mask keys for console output
            if 'key' in k.lower() and isinstance(v, str) and v and not v.lower() == 'ollama':
                print(f"{k}: {'*' * (len(v) - 4)}{v[-4:]}")
            else:
                print(f"{k}: {v}")
        print("------------------------------\n")
        
        self.close()

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = ConfigGUI()
    window.show()
    sys.exit(app.exec())