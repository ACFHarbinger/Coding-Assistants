import os
import autogen
from dotenv import load_dotenv
from rich.console import Console
from rich.panel import Panel
from rich.prompt import Prompt

# Import our custom tools
from tools.file_system import FileTools

# Load environment variables
load_dotenv()

console = Console()

def init_model_env():
    console.print(Panel.fit("[bold blue]Multi-Model AI Coding Team[/bold blue]\n"
                            "Planner: OpenAI (API)\n"
                            "Developer: Llama3 (Local via Ollama)", border_style="blue"))

    # 1. Configuration
    # ----------------
    # Config for the Planner (Smart, API-based)
    planner_config = {
        "config_list": [{
            "model": "gpt-4o",  # Or gpt-4-turbo
            "api_key": os.environ.get("OPENAI_API_KEY")
        }],
        "temperature": 0.2
    }

    # Config for the Developer (Local, Cost-effective)
    # Ensure you have run `ollama serve` and `ollama run llama3` (or qwen2.5-coder, etc.)
    developer_config = {
        "config_list": [{
            "model": "llama3.1", # Ensure this matches your local model name
            "base_url": "http://localhost:11434/v1",
            "api_key": "ollama", # Required placeholder
            "price": [0, 0], # It's free!
        }],
        "temperature": 0.5,
        "timeout": 120 # Local models can be slower
    }

    # 2. Define Agents
    # ----------------
    
    # The Admin (You) - Executes tools and gives feedback
    user_proxy = autogen.UserProxyAgent(
        name="Admin",
        human_input_mode="NEVER", # "ALWAYS" asks you before every step, "NEVER" runs fully auto until limit
        max_consecutive_auto_reply=10,
        is_termination_msg=lambda x: x.get("content", "").rstrip().endswith("TERMINATE"),
        code_execution_config={
            "work_dir": "workspace", # Code execution happens here
            "use_docker": False
        }
    )

    # The Planner - Breaks down the task
    planner = autogen.AssistantAgent(
        name="Planner",
        system_message="""You are a Senior Software Architect.
        1. Analyze the user's request.
        2. Ask the Developer to explore the codebase using `list_files` or `read_file` if context is missing.
        3. Create a step-by-step plan for the Developer to implement changes.
        4. Review the Developer's work.
        5. If the plan is complete and verified, reply "TERMINATE".""",
        llm_config=planner_config,
    )

    # The Developer - Writes code and reads files
    developer = autogen.AssistantAgent(
        name="Developer",
        system_message="""You are a Python Developer.
        You have access to the file system through tools.
        1. When asked, explore files to understand existing code.
        2. Write clean, documented code.
        3. Use `write_file` to save your code to disk.
        4. Report back to the Planner when a step is done.""",
        llm_config=developer_config,
    )

    # 3. Register Tools
    # -----------------
    # We initialize the tools on the current directory (or a specific subdirectory)
    work_dir = os.path.abspath(".") # Defaults to current directory
    file_tools = FileTools(root_dir=work_dir)

    # Helper function to register tools easily
    def register_tool(func, description):
        autogen.agentchat.register_function(
            func,
            caller=developer,  # Developer calls the tool
            executor=user_proxy, # User Proxy executes the tool (locally)
            name=func.__name__,
            description=description,
        )

    register_tool(file_tools.list_files, "List files in a directory. Args: directory, recursive (bool)")
    register_tool(file_tools.read_file, "Read file content. Args: filepath")
    register_tool(file_tools.write_file, "Write content to a file. Args: filepath, content")
    register_tool(file_tools.search_files, "Search for a keyword in files. Args: keyword, directory")

    # 4. Start the Workflow
    # ---------------------
    
    # Create the group chat
    groupchat = autogen.GroupChat(
        agents=[user_proxy, planner, developer],
        messages=[],
        max_round=20
    )
    
    manager = autogen.GroupChatManager(groupchat=groupchat, llm_config=planner_config)

    # Get user task
    task = Prompt.ask("[bold green]What would you like the team to do?[/bold green]")
    
    console.print(f"\n[italic]Starting task in context: {work_dir}[/italic]\n")

    # Start the conversation
    user_proxy.initiate_chat(
        manager,
        message=task
    )

if __name__ == "__main__":
    init_model_env()