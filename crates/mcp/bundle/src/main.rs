//! Consolidated MCP runner for Coding-Assistants tool bridges.
//!
//! Usage:
//!   coding-assistants-mcp <tool> [args...]
//!
//! Subcommands correspond to each tool bridge:
//!   aseprite, blender, godot, krita, opentoonz, unity, unreal.

const USAGE: &str = "\
Usage: coding-assistants-mcp <tool> [args...]

Available tools:
  aseprite    - Aseprite batch-mode Lua bridge
  blender     - Blender TCP socket bridge
  godot       - Godot 4 editor TCP socket bridge
  krita       - Krita TCP socket bridge
  opentoonz   - OpenToonz scene inspection & render bridge
  unity       - Unity editor TCP socket bridge
  unreal      - Unreal Engine 5 editor TCP socket bridge
";

#[derive(Debug, PartialEq, Eq)]
pub enum ToolCommand {
    Aseprite,
    Blender,
    Godot,
    Krita,
    OpenToonz,
    Unity,
    Unreal,
}

impl ToolCommand {
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "aseprite" => Some(Self::Aseprite),
            "blender" => Some(Self::Blender),
            "godot" => Some(Self::Godot),
            "krita" => Some(Self::Krita),
            "opentoonz" => Some(Self::OpenToonz),
            "unity" => Some(Self::Unity),
            "unreal" => Some(Self::Unreal),
            _ => None,
        }
    }

    pub fn run(&self, args: &[String]) {
        match self {
            Self::Aseprite => mcp_aseprite::run(args),
            Self::Blender => mcp_blender::run(args),
            Self::Godot => mcp_godot::run(args),
            Self::Krita => mcp_krita::run(args),
            Self::OpenToonz => mcp_opentoonz::run(args),
            Self::Unity => mcp_unity::run(args),
            Self::Unreal => mcp_unreal::run(args),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" || args[0] == "help" {
        eprint!("{USAGE}");
        if args.is_empty() {
            std::process::exit(1);
        }
        return;
    }

    let tool_name = &args[0];
    let tool_args = &args[1..];

    match ToolCommand::parse(tool_name) {
        Some(cmd) => cmd.run(tool_args),
        None => {
            eprintln!("unknown MCP tool: {tool_name}\n\n{USAGE}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all_valid_tools() {
        assert_eq!(ToolCommand::parse("aseprite"), Some(ToolCommand::Aseprite));
        assert_eq!(ToolCommand::parse("blender"), Some(ToolCommand::Blender));
        assert_eq!(ToolCommand::parse("godot"), Some(ToolCommand::Godot));
        assert_eq!(ToolCommand::parse("krita"), Some(ToolCommand::Krita));
        assert_eq!(
            ToolCommand::parse("opentoonz"),
            Some(ToolCommand::OpenToonz)
        );
        assert_eq!(ToolCommand::parse("unity"), Some(ToolCommand::Unity));
        assert_eq!(ToolCommand::parse("unreal"), Some(ToolCommand::Unreal));
    }

    #[test]
    fn parse_unknown_tool() {
        assert_eq!(ToolCommand::parse("unknown"), None);
        assert_eq!(ToolCommand::parse(""), None);
    }
}
