# FabiScOS Terminal
# System app for command-line access to the virtual filesystem

import fabiscos_ui as ui
import fabiscos_vfs as vfs
import fabiscos_window as window
import fabiscos_system as system
import fabiscos_state as state

# Terminal styles - all CSS properties can be passed to ui.style()
mono = ui.style(
    font_family="'Cascadia Code', 'Fira Code', 'Consolas', monospace",
    font_size="13px",
    color="#33ff33"
)
container_style = ui.style(
    background="#0d0d0d",
    padding="12px",
    height="100%",
    display="flex",
    flex_direction="column",
    box_sizing="border-box"
)
output_style = ui.merge_styles([
    mono,
    ui.style(margin="0", flex="1", overflow="auto")
])
input_row_style = ui.style(
    background="#0d0d0d",
    padding="0",
    margin="0",
    display="flex",
    align_items="center"
)
prompt_style = mono
input_style = ui.merge_styles([
    mono,
    ui.style(background="transparent", border="none", outline="none", flex="1")
])

# Load state from persistent storage
_loaded = state.get_list("output_lines")
if _loaded and len(_loaded) > 0:
    output_lines = _loaded
else:
    output_lines = [
        "  ___       _     _ ___       ___  ___",
        " | __| __ _| |__ (_) __|__  / _ \\/ __|",
        " | _| / _` | '_ \\| \\__ | _|| (_) \\__ \\",
        " |_|  \\__,_|_.__/|_|___|___\\___/|___/",
        "",
        "Welcome to FabiScOS Terminal v0.1.0",
        "Type 'help' for available commands.",
        ""
    ]

cwd = vfs.cwd()

def format_prompt():
    return f"{cwd} $ "

def save_state():
    """Save terminal state to persistent storage"""
    state.set_list("output_lines", output_lines)

def execute(cmd):
    """Execute a terminal command"""
    global cwd, output_lines

    # Add command to output
    output_lines.append(f"{format_prompt()}{cmd}")

    parts = cmd.strip().split()
    if not parts:
        save_state()
        return

    command = parts[0]
    args = parts[1:]

    def resolve_path(p):
        """Resolve a path relative to cwd"""
        if not p.startswith("/"):
            p = f"{cwd}/{p}"
        # Normalize path
        parts_list = []
        for part in p.split("/"):
            if part == "..":
                if parts_list:
                    parts_list.pop()
            elif part and part != ".":
                parts_list.append(part)
        return "/" + "/".join(parts_list)

    if command == "help":
        output_lines.append("Available commands:")
        output_lines.append("  help     - Show this help")
        output_lines.append("  ls       - List directory contents")
        output_lines.append("  cd       - Change directory")
        output_lines.append("  pwd      - Print working directory")
        output_lines.append("  cat      - Show file contents")
        output_lines.append("  touch    - Create empty file")
        output_lines.append("  mkdir    - Create directory")
        output_lines.append("  rm       - Remove file")
        output_lines.append("  rmdir    - Remove empty directory")
        output_lines.append("  cp       - Copy file")
        output_lines.append("  mv       - Move/rename file")
        output_lines.append("  clear    - Clear screen")
        output_lines.append("  echo     - Print text")

    elif command == "ls":
        path = resolve_path(args[0]) if args else cwd
        try:
            entries = vfs.list_dir(path)
            if not entries:
                output_lines.append("  (empty)")
            else:
                for e in entries:
                    name = e.get("name", "?")
                    ftype = e.get("type", "file")
                    if ftype == "directory":
                        output_lines.append(f"  {name}/")
                    else:
                        output_lines.append(f"  {name}")
        except Exception as e:
            output_lines.append(f"ls: {e}")

    elif command == "cd":
        if args:
            new_path = resolve_path(args[0])
            if vfs.exists(new_path):
                cwd = new_path
                vfs.set_cwd(cwd)
            else:
                output_lines.append(f"cd: {args[0]}: No such directory")
        else:
            cwd = "/home"
            vfs.set_cwd(cwd)

    elif command == "pwd":
        output_lines.append(cwd)

    elif command == "cat":
        if args:
            path = resolve_path(args[0])
            try:
                content = vfs.read_text(path)
                for line in content.split("\n"):
                    output_lines.append(line)
            except Exception as e:
                output_lines.append(f"cat: {args[0]}: {e}")
        else:
            output_lines.append("cat: missing file operand")

    elif command == "touch":
        if args:
            path = resolve_path(args[0])
            try:
                if not vfs.exists(path):
                    vfs.write(path, "")
            except Exception as e:
                output_lines.append(f"touch: {args[0]}: {e}")
        else:
            output_lines.append("touch: missing file operand")

    elif command == "mkdir":
        if args:
            path = resolve_path(args[0])
            try:
                vfs.mkdir(path)
            except Exception as e:
                output_lines.append(f"mkdir: {args[0]}: {e}")
        else:
            output_lines.append("mkdir: missing operand")

    elif command == "rm":
        if args:
            path = resolve_path(args[0])
            try:
                vfs.remove(path)
            except Exception as e:
                output_lines.append(f"rm: {args[0]}: {e}")
        else:
            output_lines.append("rm: missing operand")

    elif command == "rmdir":
        if args:
            path = resolve_path(args[0])
            try:
                # Check if directory is empty
                entries = vfs.list_dir(path)
                if entries:
                    output_lines.append(f"rmdir: {args[0]}: Directory not empty")
                else:
                    vfs.remove(path)
            except Exception as e:
                output_lines.append(f"rmdir: {args[0]}: {e}")
        else:
            output_lines.append("rmdir: missing operand")

    elif command == "cp":
        if len(args) >= 2:
            src = resolve_path(args[0])
            dst = resolve_path(args[1])
            try:
                vfs.copy(src, dst)
            except Exception as e:
                output_lines.append(f"cp: {e}")
        else:
            output_lines.append("cp: missing operand")
            output_lines.append("Usage: cp <source> <dest>")

    elif command == "mv":
        if len(args) >= 2:
            src = resolve_path(args[0])
            dst = resolve_path(args[1])
            try:
                vfs.move(src, dst)
            except Exception as e:
                output_lines.append(f"mv: {e}")
        else:
            output_lines.append("mv: missing operand")
            output_lines.append("Usage: mv <source> <dest>")

    elif command == "clear":
        output_lines.clear()

    elif command == "echo":
        output_lines.append(" ".join(args))

    else:
        output_lines.append(f"bash: {command}: command not found")

    # Save state after every command
    save_state()

def on_input(value):
    """Called when Enter is pressed in the input field"""
    if value.strip():
        execute(value)
    render()

def render():
    """Render the terminal UI"""
    global output_lines
    # Keep only last 100 lines for performance
    if len(output_lines) > 100:
        output_lines = output_lines[-100:]
        save_state()

    # Build output text
    output_text = "\n".join(output_lines)

    # Create terminal UI with proper styling
    terminal_html = ui.container([
        ui.text(output_text, style=output_style),
        ui.row([
            ui.label(format_prompt(), style=prompt_style),
            ui.input("", style=input_style, on_submit="execute")
        ], style=input_row_style)
    ], style=container_style)

    window.set_content(terminal_html)
    window.set_title("Terminal")

# Initial render
render()
