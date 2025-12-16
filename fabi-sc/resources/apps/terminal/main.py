# FabiScOS Terminal
# System app for command-line access to the virtual filesystem

from fabiscos import ui, vfs, window, system

# Terminal state
cwd = "/home"
history = []
output_lines = ["Welcome to FabiScOS Terminal", "Type 'help' for available commands", ""]

def execute(cmd):
    """Execute a terminal command"""
    global cwd, output_lines

    cmd = cmd.strip()
    if not cmd:
        return

    # Add to history
    history.append(cmd)
    output_lines.append(f"{cwd}$ {cmd}")

    parts = cmd.split()
    command = parts[0].lower()
    args = parts[1:] if len(parts) > 1 else []

    try:
        if command == "help":
            output_lines.extend([
                "Available commands:",
                "  ls [path]     - List directory contents",
                "  cd <path>     - Change directory",
                "  pwd           - Print working directory",
                "  cat <file>    - Display file contents",
                "  mkdir <name>  - Create directory",
                "  rm <path>     - Remove file or directory",
                "  cp <src> <dst>- Copy file",
                "  mv <src> <dst>- Move/rename file",
                "  touch <file>  - Create empty file",
                "  clear         - Clear terminal",
                "  apps          - List installed apps",
                "  date          - Show current date",
                "  time          - Show current time",
                "  help          - Show this help",
                ""
            ])

        elif command == "ls":
            path = args[0] if args else cwd
            if not path.startswith("/"):
                path = f"{cwd}/{path}"

            entries = vfs.list_dir(path)
            for entry in entries:
                name = entry["name"]
                if entry["is_dir"]:
                    output_lines.append(f"  {name}/")
                else:
                    size = entry.get("size", 0)
                    output_lines.append(f"  {name}  ({size} bytes)")
            output_lines.append("")

        elif command == "cd":
            if not args:
                cwd = "/home"
            else:
                new_path = args[0]
                if not new_path.startswith("/"):
                    new_path = f"{cwd}/{new_path}"

                # Normalize path
                parts = []
                for part in new_path.split("/"):
                    if part == "..":
                        if parts:
                            parts.pop()
                    elif part and part != ".":
                        parts.append(part)
                new_path = "/" + "/".join(parts) if parts else "/"

                if vfs.exists(new_path):
                    cwd = new_path
                else:
                    output_lines.append(f"cd: no such directory: {new_path}")
            output_lines.append("")

        elif command == "pwd":
            output_lines.append(cwd)
            output_lines.append("")

        elif command == "cat":
            if not args:
                output_lines.append("cat: missing filename")
            else:
                path = args[0]
                if not path.startswith("/"):
                    path = f"{cwd}/{path}"

                content = vfs.read_text(path)
                output_lines.append(content)
            output_lines.append("")

        elif command == "mkdir":
            if not args:
                output_lines.append("mkdir: missing directory name")
            else:
                path = args[0]
                if not path.startswith("/"):
                    path = f"{cwd}/{path}"
                vfs.mkdir(path)
                output_lines.append(f"Created directory: {path}")
            output_lines.append("")

        elif command == "rm":
            if not args:
                output_lines.append("rm: missing path")
            else:
                path = args[0]
                if not path.startswith("/"):
                    path = f"{cwd}/{path}"
                vfs.remove(path)
                output_lines.append(f"Removed: {path}")
            output_lines.append("")

        elif command == "touch":
            if not args:
                output_lines.append("touch: missing filename")
            else:
                path = args[0]
                if not path.startswith("/"):
                    path = f"{cwd}/{path}"
                vfs.write(path, "")
                output_lines.append(f"Created: {path}")
            output_lines.append("")

        elif command == "clear":
            output_lines.clear()

        elif command == "apps":
            output_lines.append("Installed apps:")
            # Will be implemented when app listing is available
            output_lines.append("  (app listing not yet implemented)")
            output_lines.append("")

        elif command == "date":
            output_lines.append(system.date())
            output_lines.append("")

        elif command == "time":
            output_lines.append(system.time())
            output_lines.append("")

        else:
            output_lines.append(f"Unknown command: {command}")
            output_lines.append("Type 'help' for available commands")
            output_lines.append("")

    except Exception as e:
        output_lines.append(f"Error: {e}")
        output_lines.append("")

    render()

def render():
    """Render the terminal UI"""
    # Keep only last 50 lines for performance
    visible_output = output_lines[-50:]

    window.set_content(
        ui.column(
            ui.label("\n".join(visible_output)),
            ui.row(
                ui.label(f"{cwd}$ "),
                ui.input(placeholder="", on_submit=execute)
            )
        )
    )

# Initial render
render()
