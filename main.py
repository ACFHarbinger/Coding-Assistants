import sys
from logic.src.environment import init_model_env
from logic.src.config.arg_parser import parse_params


# Placeholder for GUI import to avoid immediate dependency error if PySide6 isn't installed
# In a real scenario, ensure gui_config is in the python path or strictly imported inside the 'if' block
def run_gui(config):
    try:
        from gui.src.configs_window import ConfigGUI 
        from PySide6.QtWidgets import QApplication
        
        # Initialize the Qt Application
        app = QApplication(sys.argv)
        
        # Apply style if provided in args
        if config.get('app_style'):
            app.setStyle(config['app_style'])
            
        window = ConfigGUI()
        # Pre-fill GUI with CLI args if passed
        # (This assumes ConfigGUI has methods to set these values)
        # window.set_defaults(config) 
        
        window.show()
        sys.exit(app.exec())
    except ImportError as e:
        print("Error: PySide6 is not installed or GUI module not found.")
        print(f"Details: {e}")
        sys.exit(1)


if __name__ == "__main__":
    # 1. Parse Command Line Arguments
    command, config = parse_params()

    print(f"Starting Multi-LLM Assistant in [{command.upper()}] mode...")

    # 2. Dispatch based on command
    if command == "gui":
        run_gui(config)
        
    elif command == "run":
        # Initialize the backend environment with the parsed config
        # We pass the config dictionary to the init function
        init_model_env(config)