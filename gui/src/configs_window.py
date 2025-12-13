import sys
import os

from PySide6.QtWidgets import (
    QApplication, QWidget, QVBoxLayout, QHBoxLayout, 
    QLabel, QLineEdit, QPushButton, QComboBox, 
    QFileDialog, QTextEdit, QGroupBox, QFormLayout
)
from PySide6.QtCore import Qt


class ConfigGUI(QWidget):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Multi-LLM Assistant Configuration")
        self.setMinimumWidth(600)
        
        self.layout = QVBoxLayout()
        self.setLayout(self.layout)

        # Planner Config Group
        self.create_planner_group()
        
        # Developer Config Group
        self.create_developer_group()
        
        # Workspace Config Group
        self.create_workspace_group()

        # Task Input
        self.create_task_group()

        # Action Buttons
        self.create_buttons()

    def create_planner_group(self):
        group = QGroupBox("Planner Agent (Architecture)")
        layout = QFormLayout()
        
        self.planner_model = QComboBox()
        self.planner_model.addItems(["gpt-4o", "gpt-4-turbo", "gpt-3.5-turbo", "claude-3-opus", "claude-3-sonnet"])
        self.planner_model.setEditable(True) # Allow custom input
        
        self.planner_api_key = QLineEdit()
        self.planner_api_key.setEchoMode(QLineEdit.EchoMode.Password)
        self.planner_api_key.setPlaceholderText("sk-...")
        if os.environ.get("OPENAI_API_KEY"):
            self.planner_api_key.setText(os.environ.get("OPENAI_API_KEY"))
            
        layout.addRow("Model:", self.planner_model)
        layout.addRow("API Key:", self.planner_api_key)
        group.setLayout(layout)
        self.layout.addWidget(group)

    def create_developer_group(self):
        group = QGroupBox("Developer Agent (Coding - Local/API)")
        layout = QFormLayout()
        
        self.dev_model = QComboBox()
        self.dev_model.addItems(["llama3.1", "qwen2.5-coder", "mistral", "deepseek-coder", "gpt-3.5-turbo"])
        self.dev_model.setEditable(True)
        
        self.dev_url = QLineEdit()
        self.dev_url.setText("http://localhost:11434/v1")
        self.dev_url.setPlaceholderText("http://localhost:11434/v1")
        
        self.dev_api_key = QLineEdit()
        self.dev_api_key.setText("ollama")
        self.dev_api_key.setPlaceholderText("API Key (or 'ollama')")
        
        layout.addRow("Model:", self.dev_model)
        layout.addRow("Base URL:", self.dev_url)
        layout.addRow("API Key:", self.dev_api_key)
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
        config = {
            "planner_model": self.planner_model.currentText(),
            "planner_key": self.planner_api_key.text(),
            "dev_model": self.dev_model.currentText(),
            "dev_url": self.dev_url.text(),
            "dev_key": self.dev_api_key.text(),
            "work_dir": self.work_dir_input.text(),
            "task": self.task_input.toPlainText()
        }
        
        print("\n--- Starting Configuration ---")
        for k, v in config.items():
            print(f"{k}: {v}")
        print("------------------------------\n")
        
        # Here you would typically import your main app logic and run it
        # For example: 
        # from app import run_agents
        # run_agents(config)
        
        self.close()

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = ConfigGUI()
    window.show()
    sys.exit(app.exec())