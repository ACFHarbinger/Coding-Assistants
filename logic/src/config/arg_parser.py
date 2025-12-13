import argparse
import sys
import os

# ==============================================================================
# 
# ARGUMENT BUILDER FUNCTIONS
#
# ==============================================================================
def add_agent_args(parser):
    """
    Adds arguments for configuring the LLM agents (Planner, Developer, Reviewer).
    All model arguments now accept multiple space-separated values.
    """
    # --- Planner Configuration (High Reasoning) ---
    parser.add_argument('--planner_models', nargs='+', type=str, default=["gpt-4o"], 
                        help="Models to use for the Planner agent (e.g., gpt-4o gemini-2.5-pro)")
    
    parser.add_argument('--planner_openai_key', type=str, default=os.environ.get("OPENAI_API_KEY"),
                        help="OpenAI API Key for the Planner (defaults to OPENAI_API_KEY env var)")
    parser.add_argument('--planner_gemini_key', type=str, default=os.environ.get("GEMINI_API_KEY"),
                        help="Gemini API Key for the Planner (defaults to GEMINI_API_KEY env var)")

    parser.add_argument('--planner_temp', type=float, default=0.2,
                        help="Temperature setting for the Planner agent")

    # --- Developer Configuration (Coding/Execution) ---
    parser.add_argument('--developer_models', nargs='+', type=str, default=["llama3"],
                        help="Models to use for the Developer agent (e.g., llama3 qwen2.5-coder)")
    
    parser.add_argument('--developer_openai_key', type=str, default=None,
                        help="OpenAI API Key for the Developer.")
    parser.add_argument('--developer_gemini_key', type=str, default=None,
                        help="Gemini API Key for the Developer.")
    
    # Ollama/Local Configuration
    parser.add_argument('--developer_base_url', type=str, default="http://localhost:11434/v1",
                        help="Base URL for local/compatible API (e.g., Ollama, use 'ollama' for api_key)")
    parser.add_argument('--developer_api_key', type=str, default="ollama",
                        help="API Key for the Developer model (use 'ollama' for local Ollama instances)")
    
    parser.add_argument('--developer_temp', type=float, default=0.5,
                        help="Temperature setting for the Developer agent")
    parser.add_argument('--developer_timeout', type=int, default=120,
                        help="Timeout in seconds for developer model generation")

    # --- Reviewer Configuration (NEW Role) ---
    parser.add_argument('--reviewer_models', nargs='+', type=str, default=["gpt-4o"],
                        help="Models to use for the Reviewer agent (e.g., gpt-4o gemini-2.5-pro)")
    
    parser.add_argument('--reviewer_openai_key', type=str, default=None,
                        help="OpenAI API Key for the Reviewer.")
    parser.add_argument('--reviewer_gemini_key', type=str, default=None,
                        help="Gemini API Key for the Reviewer.")
                        
    parser.add_argument('--reviewer_temp', type=float, default=0.2,
                        help="Temperature setting for the Reviewer agent")

    return parser

def add_workspace_args(parser):
    """
    Adds arguments related to the project workspace and tasks.
    """
    parser.add_argument('--work_dir', type=str, default=".",
                        help="Root directory of the codebase to be edited")
    parser.add_argument('--task', type=str, default=None,
                        help="The initial task description (if empty, interactive mode is used)")
    parser.add_argument('--max_rounds', type=int, default=20,
                        help="Maximum number of chat rounds between agents")
    return parser

def add_gui_args(parser):
    """
    Adds arguments specific to the GUI application.
    """
    parser.add_argument('--app_style', type=str, default="Fusion", 
                        help="Visual style for the PySide6 application")
    parser.add_argument('--debug_gui', action='store_true',
                        help="Enable verbose logging for GUI events")
    return parser

def get_main_parser():
    """
    Builds the main parser with sub-commands for 'run' (CLI) and 'gui'.
    """
    parser = argparse.ArgumentParser(
        description="Multi-LLM Coding Assistant",
        formatter_class=argparse.RawTextHelpFormatter
    )
    
    subparsers = parser.add_subparsers(help='Mode of operation', dest='command', required=True)

    # 1. Run (CLI Mode)
    run_parser = subparsers.add_parser('run', help='Run the assistant in Command Line Interface mode')
    add_agent_args(run_parser)
    add_workspace_args(run_parser)

    # 2. GUI (Graphical Mode)
    gui_parser = subparsers.add_parser('gui', help='Launch the Graphical User Interface')
    add_gui_args(gui_parser)
    add_agent_args(gui_parser) 
    add_workspace_args(gui_parser)

    return parser

# ==============================================================================
# 
# VALIDATION & PARSING
#
# ==============================================================================

def validate_run_args(args):
    """
    Validates arguments for the CLI run mode.
    """
    args = args.copy()
    
    # Ensure work directory exists
    if not os.path.exists(args['work_dir']):
        print(f"Warning: Work directory '{args['work_dir']}' does not exist. Using current directory.")
        args['work_dir'] = "."

    # Normalize URLs
    if args.get('developer_base_url') and not args['developer_base_url'].startswith('http'):
         args['developer_base_url'] = f"http://{args['developer_base_url']}"

    return args

def validate_gui_args(args):
    """
    Validates arguments for the GUI mode.
    """
    args = args.copy()
    # GUI specific validation if needed
    return args

def parse_params():
    """
    Parses arguments, determines the command, and performs validation.
    """
    parser = get_main_parser()
    
    try:
        args = parser.parse_args()
        # Convert to dict for easier handling
        opts = vars(args)
        command = opts.pop('command')

        if command == 'run':
            opts = validate_run_args(opts)
        elif command == 'gui':
            opts = validate_gui_args(opts)
            
        return command, opts
        
    except Exception as e:
        print(f"Argument Parsing Error: {e}")
        sys.exit(1)