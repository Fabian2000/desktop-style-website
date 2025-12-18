# FabiScOS Terminal
# System app for command-line access to the virtual filesystem
# Bash-like shell interpreter

import fabiscos_ui as ui
import fabiscos_vfs as vfs
import fabiscos_window as window
import fabiscos_system as system
import fabiscos_state as state

# ============================================================================
# STYLES
# ============================================================================

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
password_style = ui.merge_styles([
    mono,
    ui.style(background="transparent", border="none", outline="none", flex="1", color="#33ff33")
])

# ============================================================================
# SHELL STATE
# ============================================================================

class Shell:
    """Bash-like shell interpreter"""

    def __init__(self):
        # Load state from persistent storage
        _loaded = state.get_list("output_lines")
        if _loaded and isinstance(_loaded, list) and len(_loaded) > 0:
            self.output_lines = [str(line) for line in _loaded]  # Ensure all are strings
        else:
            self.output_lines = [
                "  ___       _     _ ___       ___  ___",
                " | __| __ _| |__ (_) __|__  / _ \\/ __|",
                " | _| / _` | '_ \\| \\__ | _|| (_) \\__ \\",
                " |_|  \\__,_|_.__/|_|___|___\\___/|___/",
                "",
                "Welcome to FabiScOS Terminal v0.2.0",
                "Type 'help' for available commands.",
                ""
            ]

        # Load cwd from persistent storage
        _saved_cwd = state.get("cwd")
        if _saved_cwd and isinstance(_saved_cwd, str):
            self.cwd = _saved_cwd
            try:
                vfs.set_cwd(self.cwd)
            except:
                self.cwd = "/home"
        else:
            try:
                self.cwd = vfs.cwd()
            except:
                self.cwd = "/home"

        # Environment variables
        self.env = {
            "USER": "user",
            "HOME": "/home",
            "PATH": "/bin:/usr/bin",
            "SHELL": "/bin/bash",
            "PWD": self.cwd,
            "HOSTNAME": "fabiscos"
        }

        # Command history
        _history = state.get_list("history")
        self.history = _history if _history and isinstance(_history, list) else []
        self.history_index = len(self.history)

        # Input mode (normal, password) - load from state
        _input_mode = state.get("input_mode")
        self.input_mode = _input_mode if _input_mode and isinstance(_input_mode, str) else "normal"

        _pending = state.get("pending_sudo_cmd")
        self.pending_sudo_cmd = _pending if _pending and isinstance(_pending, str) else None

        _attempts = state.get("sudo_attempts")
        try:
            self.sudo_attempts = int(_attempts) if _attempts is not None else 0
        except (ValueError, TypeError):
            self.sudo_attempts = 0

        # Aliases - load from state or use defaults (stored as JSON string)
        _saved_aliases = state.get("aliases")
        if _saved_aliases and isinstance(_saved_aliases, str):
            try:
                import json
                self.aliases = json.loads(_saved_aliases)
            except:
                self.aliases = {
                    "ll": "ls -l",
                    "la": "ls -a",
                    "..": "cd ..",
                    "~": "cd /home"
                }
        else:
            self.aliases = {
                "ll": "ls -l",
                "la": "ls -a",
                "..": "cd ..",
                "~": "cd /home"
            }

        # Last exit code
        self.last_exit_code = 0

        # Stdin for piped input
        self.stdin = None

        # Register all commands
        self.commands = {}
        self._register_builtin_commands()

    def _register_builtin_commands(self):
        """Register all built-in commands"""
        # Navigation & filesystem
        self.register("help", cmd_help, "Show available commands")
        self.register("ls", cmd_ls, "List directory contents")
        self.register("cd", cmd_cd, "Change directory")
        self.register("pwd", cmd_pwd, "Print working directory")
        self.register("cat", cmd_cat, "Show file contents")
        self.register("touch", cmd_touch, "Create empty file")
        self.register("mkdir", cmd_mkdir, "Create directory")
        self.register("rm", cmd_rm, "Remove file or directory")
        self.register("rmdir", cmd_rmdir, "Remove empty directory")
        self.register("cp", cmd_cp, "Copy file")
        self.register("mv", cmd_mv, "Move/rename file")

        # Text & output
        self.register("echo", cmd_echo, "Print text")
        self.register("clear", cmd_clear, "Clear screen")
        self.register("head", cmd_head, "Show first lines of file")
        self.register("tail", cmd_tail, "Show last lines of file")
        self.register("wc", cmd_wc, "Count lines, words, bytes")
        self.register("grep", cmd_grep, "Search for pattern in files")
        self.register("sort", cmd_sort, "Sort lines")
        self.register("uniq", cmd_uniq, "Remove duplicate lines")
        self.register("tr", cmd_tr, "Translate characters")
        self.register("cut", cmd_cut, "Cut fields from lines")
        self.register("tee", cmd_tee, "Write to file and stdout")

        # System info
        self.register("whoami", cmd_whoami, "Print current user")
        self.register("hostname", cmd_hostname, "Print hostname")
        self.register("uname", cmd_uname, "Print system info")
        self.register("date", cmd_date, "Print current date/time")
        self.register("uptime", cmd_uptime, "Show system uptime")
        self.register("env", cmd_env, "Print environment variables")
        self.register("export", cmd_export, "Set environment variable")

        # Process & misc
        self.register("history", cmd_history, "Show command history")
        self.register("alias", cmd_alias, "Show or set aliases")
        self.register("unalias", cmd_unalias, "Remove alias")
        self.register("which", cmd_which, "Show command location")
        self.register("type", cmd_type, "Show command type")
        self.register("true", cmd_true, "Return success")
        self.register("false", cmd_false, "Return failure")
        self.register("exit", cmd_exit, "Exit terminal")

        # Sudo
        self.register("sudo", cmd_sudo, "Execute as superuser")

    def register(self, name, func, description=""):
        """Register a command - makes extending easy"""
        self.commands[name] = {"func": func, "desc": description}

    def format_prompt(self):
        """Format the shell prompt"""
        if self.input_mode == "password":
            return "[sudo] password for user: "
        return f"{self.cwd} $ "

    def save_state(self):
        """Save terminal state to persistent storage"""
        try:
            state.set_list("output_lines", self.output_lines)
            state.set("cwd", self.cwd if self.cwd else "/home")
            state.set_list("history", self.history[-100:] if self.history else [])
            state.set("input_mode", self.input_mode if self.input_mode else "normal")
            state.set("pending_sudo_cmd", self.pending_sudo_cmd if self.pending_sudo_cmd else "")
            # IMPORTANT: state.set() only accepts strings!
            state.set("sudo_attempts", str(self.sudo_attempts if self.sudo_attempts else 0))
            # Aliases stored as JSON string
            import json
            state.set("aliases", json.dumps(self.aliases if self.aliases else {}))
        except Exception as e:
            # Silently fail - don't break terminal on state save errors
            pass

    def output(self, text):
        """Add a line to output"""
        # Ensure text is a string
        if text is None:
            text = ""
        self.output_lines.append(str(text))

    def resolve_path(self, p):
        """Resolve a path relative to cwd"""
        # Handle None or empty
        if not p:
            return self.cwd if self.cwd else "/home"

        p = str(p)  # Ensure string

        # Handle ~ for home
        if p.startswith("~"):
            p = self.env["HOME"] + p[1:]

        if not p.startswith("/"):
            p = f"{self.cwd}/{p}"

        # Normalize path
        parts_list = []
        for part in p.split("/"):
            if part == "..":
                if parts_list:
                    parts_list.pop()
            elif part and part != ".":
                parts_list.append(part)
        return "/" + "/".join(parts_list)

    def expand_variables(self, text):
        """Expand environment variables in text"""
        result = text
        for key, value in self.env.items():
            result = result.replace(f"${key}", value)
            result = result.replace(f"${{{key}}}", value)
        return result

    def expand_aliases(self, cmd):
        """Expand aliases in command"""
        parts = cmd.strip().split()
        if parts and parts[0] in self.aliases:
            return self.aliases[parts[0]] + " " + " ".join(parts[1:])
        return cmd

    def parse_command(self, cmd_line):
        """Parse command line into command and args, handling quotes"""
        args = []
        current = ""
        in_quotes = False
        quote_char = None

        for char in cmd_line:
            if char in '"\'':
                if not in_quotes:
                    in_quotes = True
                    quote_char = char
                elif char == quote_char:
                    in_quotes = False
                    quote_char = None
                else:
                    current += char
            elif char == ' ' and not in_quotes:
                if current:
                    args.append(current)
                    current = ""
            else:
                current += char

        if current:
            args.append(current)

        return args

    def execute(self, cmd_line):
        """Execute a command line"""
        # ALWAYS read fresh input_mode from state - script may be re-executed
        _mode = state.get("input_mode")
        self.input_mode = _mode if _mode and isinstance(_mode, str) else "normal"

        # Handle password input mode
        if self.input_mode == "password":
            self.handle_password_input(cmd_line)
            return

        # Add command to output
        self.output(f"{self.format_prompt()}{cmd_line}")

        # Skip empty commands
        if not cmd_line.strip():
            self.save_state()
            return

        # Add to history
        if cmd_line.strip() and (not self.history or self.history[-1] != cmd_line):
            self.history.append(cmd_line)
            self.history_index = len(self.history)

        # Expand aliases and variables
        cmd_line = self.expand_aliases(cmd_line)
        cmd_line = self.expand_variables(cmd_line)

        # Handle command chaining with && and ||
        if " && " in cmd_line:
            commands = cmd_line.split(" && ")
            for cmd in commands:
                result = self._execute_with_pipe(cmd.strip())
                self.last_exit_code = result
                if result != 0:
                    break
        elif " || " in cmd_line:
            commands = cmd_line.split(" || ")
            for cmd in commands:
                result = self._execute_with_pipe(cmd.strip())
                self.last_exit_code = result
                if result == 0:
                    break
        elif " ; " in cmd_line:
            commands = cmd_line.split(" ; ")
            for cmd in commands:
                self.last_exit_code = self._execute_with_pipe(cmd.strip())
        else:
            self.last_exit_code = self._execute_with_pipe(cmd_line)

        self.save_state()

    def _execute_with_pipe(self, cmd_line):
        """Execute command line with pipe support"""
        # Check for pipes
        if " | " in cmd_line:
            return self._execute_pipeline(cmd_line)
        else:
            return self._execute_with_redirect(cmd_line)

    def _execute_pipeline(self, cmd_line):
        """Execute a pipeline of commands"""
        commands = cmd_line.split(" | ")
        pipe_input = None

        for i, cmd in enumerate(commands):
            cmd = cmd.strip()
            is_last = (i == len(commands) - 1)

            # Capture output for piping
            old_output = self.output_lines[:]
            result = self._execute_with_redirect(cmd, pipe_input=pipe_input, capture=not is_last)

            if not is_last:
                # Get new lines as pipe input for next command
                new_lines = self.output_lines[len(old_output):]
                pipe_input = "\n".join(new_lines)
                # Remove captured output (don't show intermediate results)
                self.output_lines = old_output

            if result != 0:
                return result

        return 0

    def _execute_with_redirect(self, cmd_line, pipe_input=None, capture=False):
        """Execute command with redirection support"""
        redirect_out = None
        redirect_append = False
        redirect_in = None

        # Parse redirections (simple parsing, handles basic cases)
        # Output redirection: > or >>
        if " >> " in cmd_line:
            parts = cmd_line.split(" >> ", 1)
            cmd_line = parts[0].strip()
            redirect_out = parts[1].strip()
            redirect_append = True
        elif " > " in cmd_line:
            parts = cmd_line.split(" > ", 1)
            cmd_line = parts[0].strip()
            redirect_out = parts[1].strip()
            redirect_append = False

        # Input redirection: <
        if " < " in cmd_line:
            parts = cmd_line.split(" < ", 1)
            cmd_line = parts[0].strip()
            redirect_in = parts[1].strip()

        # If we have pipe input, use it instead of file input
        stdin_content = pipe_input
        if redirect_in and not stdin_content:
            try:
                path = self.resolve_path(redirect_in)
                stdin_content = vfs.read_text(path)
            except Exception as e:
                self.output(f"bash: {redirect_in}: {e}")
                return 1

        # Capture output if redirecting or piping
        old_output_len = len(self.output_lines)

        # Execute the command
        result = self._execute_single(cmd_line, stdin=stdin_content)

        # Handle output redirection
        if redirect_out:
            new_lines = self.output_lines[old_output_len:]
            content = "\n".join(new_lines)
            # Remove from display
            self.output_lines = self.output_lines[:old_output_len]

            try:
                path = self.resolve_path(redirect_out)
                if redirect_append:
                    existing = ""
                    try:
                        existing = vfs.read_text(path)
                    except:
                        pass
                    vfs.write(path, existing + content + "\n")
                else:
                    vfs.write(path, content)
            except Exception as e:
                self.output(f"bash: {redirect_out}: {e}")
                return 1

        return result

    def _execute_single(self, cmd_line, stdin=None):
        """Execute a single command"""
        args = self.parse_command(cmd_line)
        if not args:
            return 0

        command = args[0]
        cmd_args = args[1:]

        # Store stdin for commands that need it
        self.stdin = stdin

        # Check if command exists
        if command in self.commands:
            try:
                result = self.commands[command]["func"](self, cmd_args)
                return result if result is not None else 0
            except Exception as e:
                self.output(f"{command}: error: {e}")
                return 1
        else:
            self.output(f"bash: {command}: command not found")
            return 127

    def handle_password_input(self, password):
        """Handle password input for sudo"""
        # Don't echo password, just show the prompt was there
        self.output("[sudo] password for user: ")

        # ALWAYS read fresh from state - don't trust instance variable
        _attempts = state.get("sudo_attempts")
        try:
            current_attempts = int(_attempts) if _attempts else 0
        except (ValueError, TypeError):
            current_attempts = 0

        current_attempts += 1
        self.sudo_attempts = current_attempts

        # Always reject password - after 3 attempts, give up
        if current_attempts >= 3:
            self.output("sudo: 3 incorrect password attempts")
            # Reset everything
            self.input_mode = "normal"
            self.pending_sudo_cmd = None
            self.sudo_attempts = 0
            self.save_state()
            return
        else:
            self.output(f"Sorry, try again. (attempt {current_attempts}/3)")
            self.save_state()
            return

# ============================================================================
# COMMAND IMPLEMENTATIONS
# ============================================================================

def cmd_help(shell, args):
    """Show available commands"""
    shell.output("Available commands:")

    # Group commands by category
    categories = {
        "Navigation": ["cd", "pwd", "ls"],
        "Files": ["cat", "touch", "mkdir", "rm", "rmdir", "cp", "mv"],
        "Text Processing": ["head", "tail", "grep", "sort", "uniq", "wc", "cut", "tr", "tee"],
        "Output": ["echo", "clear"],
        "Search": ["which", "type"],
        "System": ["whoami", "hostname", "uname", "date", "uptime", "env", "export"],
        "Shell": ["history", "alias", "unalias", "exit"],
        "Other": ["sudo", "true", "false", "help"]
    }

    for category, cmds in categories.items():
        shell.output(f"\n  {category}:")
        for cmd in cmds:
            if cmd in shell.commands:
                desc = shell.commands[cmd]["desc"]
                shell.output(f"    {cmd:12} - {desc}")

    shell.output("")
    return 0

def cmd_ls(shell, args):
    """List directory contents"""
    show_all = "-a" in args
    show_long = "-l" in args
    args = [a for a in args if not a.startswith("-")]

    path = shell.resolve_path(args[0]) if args else shell.cwd

    try:
        entries = vfs.list_dir(path)
        if not entries:
            if show_all:
                shell.output("  ./  ../")
            else:
                shell.output("  (empty)")
        else:
            for e in entries:
                name = e.get("name", "?")
                if not show_all and name.startswith("."):
                    continue
                ftype = e.get("type", "file")
                if show_long:
                    # Simulated long format
                    perms = "drwxr-xr-x" if ftype == "directory" else "-rw-r--r--"
                    size = e.get("size", 0)
                    shell.output(f"  {perms} user user {size:>8} {name}{'/' if ftype == 'directory' else ''}")
                else:
                    if ftype == "directory":
                        shell.output(f"  {name}/")
                    else:
                        shell.output(f"  {name}")
        return 0
    except Exception as e:
        shell.output(f"ls: cannot access '{path}': {e}")
        return 1

def cmd_cd(shell, args):
    """Change directory"""
    if args:
        new_path = shell.resolve_path(args[0])
        if vfs.exists(new_path):
            shell.cwd = new_path
            shell.env["PWD"] = new_path
            vfs.set_cwd(new_path)
        else:
            shell.output(f"cd: {args[0]}: No such file or directory")
            return 1
    else:
        shell.cwd = shell.env["HOME"]
        shell.env["PWD"] = shell.cwd
        vfs.set_cwd(shell.cwd)
    return 0

def cmd_pwd(shell, args):
    """Print working directory"""
    shell.output(shell.cwd)
    return 0

def cmd_cat(shell, args):
    """Show file contents"""
    # If we have stdin and no args, output stdin
    if not args and shell.stdin:
        for line in shell.stdin.split("\n"):
            shell.output(line)
        return 0

    if not args:
        shell.output("cat: missing file operand")
        return 1

    for arg in args:
        path = shell.resolve_path(arg)
        try:
            content = vfs.read_text(path)
            for line in content.split("\n"):
                shell.output(line)
        except Exception as e:
            shell.output(f"cat: {arg}: {e}")
            return 1
    return 0

def cmd_touch(shell, args):
    """Create empty file"""
    if not args:
        shell.output("touch: missing file operand")
        return 1

    for arg in args:
        path = shell.resolve_path(arg)
        try:
            if not vfs.exists(path):
                vfs.write(path, "")
        except Exception as e:
            shell.output(f"touch: cannot touch '{arg}': {e}")
            return 1
    return 0

def cmd_mkdir(shell, args):
    """Create directory"""
    if not args:
        shell.output("mkdir: missing operand")
        return 1

    for arg in args:
        path = shell.resolve_path(arg)
        try:
            vfs.mkdir(path)
        except Exception as e:
            shell.output(f"mkdir: cannot create directory '{arg}': {e}")
            return 1
    return 0

def cmd_rm(shell, args):
    """Remove file or directory"""
    recursive = "-r" in args or "-rf" in args
    force = "-f" in args or "-rf" in args
    args = [a for a in args if not a.startswith("-")]

    if not args:
        shell.output("rm: missing operand")
        return 1

    for arg in args:
        path = shell.resolve_path(arg)
        try:
            if not vfs.exists(path):
                if not force:
                    shell.output(f"rm: cannot remove '{arg}': No such file or directory")
                    return 1
            else:
                vfs.remove(path)
        except Exception as e:
            shell.output(f"rm: cannot remove '{arg}': {e}")
            return 1
    return 0

def cmd_rmdir(shell, args):
    """Remove empty directory"""
    if not args:
        shell.output("rmdir: missing operand")
        return 1

    for arg in args:
        path = shell.resolve_path(arg)
        try:
            entries = vfs.list_dir(path)
            if entries:
                shell.output(f"rmdir: failed to remove '{arg}': Directory not empty")
                return 1
            vfs.remove(path)
        except Exception as e:
            shell.output(f"rmdir: failed to remove '{arg}': {e}")
            return 1
    return 0

def cmd_cp(shell, args):
    """Copy file"""
    if len(args) < 2:
        shell.output("cp: missing operand")
        shell.output("Usage: cp <source> <dest>")
        return 1

    src = shell.resolve_path(args[0])
    dst = shell.resolve_path(args[1])

    try:
        vfs.copy(src, dst)
        return 0
    except Exception as e:
        shell.output(f"cp: {e}")
        return 1

def cmd_mv(shell, args):
    """Move/rename file"""
    if len(args) < 2:
        shell.output("mv: missing operand")
        shell.output("Usage: mv <source> <dest>")
        return 1

    src = shell.resolve_path(args[0])
    dst = shell.resolve_path(args[1])

    try:
        vfs.move(src, dst)
        return 0
    except Exception as e:
        shell.output(f"mv: {e}")
        return 1

def cmd_echo(shell, args):
    """Print text"""
    # Handle -n flag (no newline - we ignore since we add lines)
    if args and args[0] == "-n":
        args = args[1:]

    text = " ".join(args)
    # Handle escape sequences
    text = text.replace("\\n", "\n").replace("\\t", "\t")

    for line in text.split("\n"):
        shell.output(line)
    return 0

def cmd_clear(shell, args):
    """Clear screen"""
    shell.output_lines.clear()
    return 0

def cmd_head(shell, args):
    """Show first lines of file"""
    n = 10
    files = []

    i = 0
    while i < len(args):
        if args[i] == "-n" and i + 1 < len(args):
            n = int(args[i + 1])
            i += 2
        elif args[i].startswith("-"):
            try:
                n = int(args[i][1:])
            except:
                pass
            i += 1
        else:
            files.append(args[i])
            i += 1

    # Handle stdin
    if not files and shell.stdin:
        lines = shell.stdin.split("\n")[:n]
        for line in lines:
            shell.output(line)
        return 0

    if not files:
        shell.output("head: missing file operand")
        return 1

    for f in files:
        path = shell.resolve_path(f)
        try:
            content = vfs.read_text(path)
            lines = content.split("\n")[:n]
            if len(files) > 1:
                shell.output(f"==> {f} <==")
            for line in lines:
                shell.output(line)
        except Exception as e:
            shell.output(f"head: {f}: {e}")
            return 1
    return 0

def cmd_tail(shell, args):
    """Show last lines of file"""
    n = 10
    files = []

    i = 0
    while i < len(args):
        if args[i] == "-n" and i + 1 < len(args):
            n = int(args[i + 1])
            i += 2
        elif args[i].startswith("-"):
            try:
                n = int(args[i][1:])
            except:
                pass
            i += 1
        else:
            files.append(args[i])
            i += 1

    # Handle stdin
    if not files and shell.stdin:
        lines = shell.stdin.split("\n")[-n:]
        for line in lines:
            shell.output(line)
        return 0

    if not files:
        shell.output("tail: missing file operand")
        return 1

    for f in files:
        path = shell.resolve_path(f)
        try:
            content = vfs.read_text(path)
            lines = content.split("\n")[-n:]
            if len(files) > 1:
                shell.output(f"==> {f} <==")
            for line in lines:
                shell.output(line)
        except Exception as e:
            shell.output(f"tail: {f}: {e}")
            return 1
    return 0

def cmd_wc(shell, args):
    """Count lines, words, bytes"""
    # Handle stdin
    if not args and shell.stdin:
        content = shell.stdin
        lines = len(content.split("\n"))
        words = len(content.split())
        bytes_count = len(content.encode('utf-8'))
        shell.output(f"  {lines:>6} {words:>6} {bytes_count:>6}")
        return 0

    if not args:
        shell.output("wc: missing file operand")
        return 1

    total_lines = 0
    total_words = 0
    total_bytes = 0

    for arg in args:
        path = shell.resolve_path(arg)
        try:
            content = vfs.read_text(path)
            lines = len(content.split("\n"))
            words = len(content.split())
            bytes_count = len(content.encode('utf-8'))
            shell.output(f"  {lines:>6} {words:>6} {bytes_count:>6} {arg}")
            total_lines += lines
            total_words += words
            total_bytes += bytes_count
        except Exception as e:
            shell.output(f"wc: {arg}: {e}")
            return 1

    if len(args) > 1:
        shell.output(f"  {total_lines:>6} {total_words:>6} {total_bytes:>6} total")
    return 0

def cmd_grep(shell, args):
    """Search for pattern in files"""
    if not args:
        shell.output("grep: missing operand")
        shell.output("Usage: grep <pattern> [file...]")
        return 1

    pattern = args[0]
    files = args[1:]
    found = False

    # Handle stdin (piped input)
    if not files and shell.stdin:
        for line in shell.stdin.split("\n"):
            if pattern in line:
                found = True
                shell.output(line)
        return 0 if found else 1

    if not files:
        shell.output("grep: missing file operand")
        return 1

    for f in files:
        path = shell.resolve_path(f)
        try:
            content = vfs.read_text(path)
            for i, line in enumerate(content.split("\n"), 1):
                if pattern in line:
                    found = True
                    if len(files) > 1:
                        shell.output(f"{f}:{line}")
                    else:
                        shell.output(line)
        except Exception as e:
            shell.output(f"grep: {f}: {e}")

    return 0 if found else 1

def cmd_sort(shell, args):
    """Sort lines"""
    reverse = "-r" in args
    numeric = "-n" in args
    args = [a for a in args if not a.startswith("-")]

    # Get content from stdin or file
    if shell.stdin:
        content = shell.stdin
    elif args:
        path = shell.resolve_path(args[0])
        try:
            content = vfs.read_text(path)
        except Exception as e:
            shell.output(f"sort: {args[0]}: {e}")
            return 1
    else:
        shell.output("sort: missing operand")
        return 1

    lines = content.split("\n")

    if numeric:
        def sort_key(x):
            try:
                return float(x.split()[0]) if x.split() else 0
            except:
                return 0
        lines.sort(key=sort_key, reverse=reverse)
    else:
        lines.sort(reverse=reverse)

    for line in lines:
        shell.output(line)
    return 0

def cmd_uniq(shell, args):
    """Remove duplicate lines"""
    count = "-c" in args
    args = [a for a in args if not a.startswith("-")]

    # Get content from stdin or file
    if shell.stdin:
        content = shell.stdin
    elif args:
        path = shell.resolve_path(args[0])
        try:
            content = vfs.read_text(path)
        except Exception as e:
            shell.output(f"uniq: {args[0]}: {e}")
            return 1
    else:
        shell.output("uniq: missing operand")
        return 1

    lines = content.split("\n")
    result = []
    prev = None
    cnt = 0

    for line in lines:
        if line == prev:
            cnt += 1
        else:
            if prev is not None:
                if count:
                    result.append(f"{cnt:>6} {prev}")
                else:
                    result.append(prev)
            prev = line
            cnt = 1

    if prev is not None:
        if count:
            result.append(f"{cnt:>6} {prev}")
        else:
            result.append(prev)

    for line in result:
        shell.output(line)
    return 0

def cmd_tr(shell, args):
    """Translate characters"""
    delete = "-d" in args
    args = [a for a in args if not a.startswith("-")]

    if not shell.stdin:
        shell.output("tr: no input")
        return 1

    if delete:
        if not args:
            shell.output("tr: missing operand")
            return 1
        chars_to_delete = args[0]
        result = shell.stdin
        for c in chars_to_delete:
            result = result.replace(c, "")
    else:
        if len(args) < 2:
            shell.output("tr: missing operand")
            shell.output("Usage: tr <set1> <set2>")
            return 1
        set1 = args[0]
        set2 = args[1]
        result = shell.stdin
        for i, c in enumerate(set1):
            if i < len(set2):
                result = result.replace(c, set2[i])

    for line in result.split("\n"):
        shell.output(line)
    return 0

def cmd_cut(shell, args):
    """Cut fields from lines"""
    delimiter = "\t"
    fields = None

    i = 0
    while i < len(args):
        if args[i] == "-d" and i + 1 < len(args):
            delimiter = args[i + 1]
            i += 2
        elif args[i] == "-f" and i + 1 < len(args):
            fields = args[i + 1]
            i += 2
        elif args[i].startswith("-d"):
            delimiter = args[i][2:]
            i += 1
        elif args[i].startswith("-f"):
            fields = args[i][2:]
            i += 1
        else:
            i += 1

    if not fields:
        shell.output("cut: you must specify a list of fields")
        return 1

    # Parse field spec (e.g., "1", "1,3", "1-3")
    field_indices = []
    for part in fields.split(","):
        if "-" in part:
            start, end = part.split("-")
            start = int(start) if start else 1
            end = int(end) if end else 999
            field_indices.extend(range(start, end + 1))
        else:
            field_indices.append(int(part))

    # Get content
    if shell.stdin:
        content = shell.stdin
    else:
        shell.output("cut: no input")
        return 1

    for line in content.split("\n"):
        parts = line.split(delimiter)
        selected = []
        for idx in field_indices:
            if 0 < idx <= len(parts):
                selected.append(parts[idx - 1])
        shell.output(delimiter.join(selected))
    return 0

def cmd_tee(shell, args):
    """Write to file and stdout"""
    append = "-a" in args
    args = [a for a in args if not a.startswith("-")]

    if not args:
        shell.output("tee: missing file operand")
        return 1

    if not shell.stdin:
        shell.output("tee: no input")
        return 1

    # Output to stdout
    for line in shell.stdin.split("\n"):
        shell.output(line)

    # Write to file(s)
    for f in args:
        path = shell.resolve_path(f)
        try:
            if append:
                existing = ""
                try:
                    existing = vfs.read_text(path)
                except:
                    pass
                vfs.write(path, existing + shell.stdin)
            else:
                vfs.write(path, shell.stdin)
        except Exception as e:
            shell.output(f"tee: {f}: {e}")
            return 1

    return 0

def cmd_whoami(shell, args):
    """Print current user"""
    shell.output(shell.env["USER"])
    return 0

def cmd_hostname(shell, args):
    """Print hostname"""
    shell.output(shell.env["HOSTNAME"])
    return 0

def cmd_uname(shell, args):
    """Print system info"""
    if "-a" in args:
        shell.output("FabiScOS 0.2.0 fabiscos 1.0.0 WASM FabiScOS")
    elif "-r" in args:
        shell.output("1.0.0")
    elif "-s" in args:
        shell.output("FabiScOS")
    else:
        shell.output("FabiScOS")
    return 0

def cmd_date(shell, args):
    """Print current date/time"""
    # Use JavaScript Date via system module if available
    try:
        shell.output(system.get_datetime())
    except:
        shell.output("Date not available")
    return 0

def cmd_uptime(shell, args):
    """Show system uptime"""
    shell.output(" up 0 days, 0:00, 1 user")
    return 0

def cmd_env(shell, args):
    """Print environment variables"""
    for key, value in sorted(shell.env.items()):
        shell.output(f"{key}={value}")
    return 0

def cmd_export(shell, args):
    """Set environment variable"""
    if not args:
        return cmd_env(shell, args)

    for arg in args:
        if "=" in arg:
            key, value = arg.split("=", 1)
            shell.env[key] = value
        else:
            shell.output(f"export: '{arg}': not a valid identifier")
            return 1
    return 0

def cmd_history(shell, args):
    """Show command history"""
    for i, cmd in enumerate(shell.history, 1):
        shell.output(f"  {i:>4}  {cmd}")
    return 0

def cmd_alias(shell, args):
    """Show or set aliases"""
    if not args:
        for name, value in sorted(shell.aliases.items()):
            shell.output(f"alias {name}='{value}'")
        return 0

    for arg in args:
        if "=" in arg:
            name, value = arg.split("=", 1)
            # Remove quotes if present
            value = value.strip("'\"")
            shell.aliases[name] = value
        else:
            if arg in shell.aliases:
                shell.output(f"alias {arg}='{shell.aliases[arg]}'")
            else:
                shell.output(f"alias: {arg}: not found")
                return 1
    return 0

def cmd_unalias(shell, args):
    """Remove alias"""
    if not args:
        shell.output("unalias: missing operand")
        return 1

    for arg in args:
        if arg in shell.aliases:
            del shell.aliases[arg]
        else:
            shell.output(f"unalias: {arg}: not found")
            return 1
    return 0

def cmd_which(shell, args):
    """Show command location"""
    if not args:
        shell.output("which: missing operand")
        return 1

    for arg in args:
        if arg in shell.commands:
            shell.output(f"/usr/bin/{arg}")
        else:
            shell.output(f"{arg}: not found")
            return 1
    return 0

def cmd_type(shell, args):
    """Show command type"""
    if not args:
        shell.output("type: missing operand")
        return 1

    for arg in args:
        if arg in shell.aliases:
            shell.output(f"{arg} is aliased to `{shell.aliases[arg]}'")
        elif arg in shell.commands:
            shell.output(f"{arg} is a shell builtin")
        else:
            shell.output(f"bash: type: {arg}: not found")
            return 1
    return 0

def cmd_true(shell, args):
    """Return success"""
    return 0

def cmd_false(shell, args):
    """Return failure"""
    return 1

def cmd_exit(shell, args):
    """Exit terminal"""
    window.close()
    return 0

def cmd_sudo(shell, args):
    """Execute as superuser"""
    if not args:
        shell.output("usage: sudo <command>")
        return 1

    # Store the pending command
    shell.pending_sudo_cmd = " ".join(args)
    shell.sudo_attempts = 0
    shell.input_mode = "password"

    return 0

# ============================================================================
# GLOBAL SHELL INSTANCE & UI
# ============================================================================

shell = Shell()

def on_input(value):
    """Called when Enter is pressed in the input field"""
    if value.strip() or shell.input_mode == "password":
        shell.execute(value)
    render()
    window.focus("input")
    # Scroll to bottom only after executing a command
    window.scroll_to_bottom("output")

def on_back():
    """Called when mobile back button is pressed"""
    window.close()

def focus_terminal():
    """Focus the input field when terminal is clicked"""
    window.focus("input")

def render():
    """Render the terminal UI"""
    # Keep only last 100 lines for performance
    if len(shell.output_lines) > 100:
        shell.output_lines = shell.output_lines[-100:]
        shell.save_state()

    # Build output text
    output_text = "\n".join(shell.output_lines)

    # Choose input style based on mode
    current_input_style = password_style if shell.input_mode == "password" else input_style
    input_type = "password" if shell.input_mode == "password" else "text"

    # Create terminal UI
    # on_click="focus_terminal" sets _skip_render=True to prevent scroll reset
    terminal_html = ui.container([
        ui.text(output_text, style=output_style, name="output"),
        ui.row([
            ui.label(shell.format_prompt(), style=prompt_style),
            ui.input("", style=current_input_style, on_submit="on_input", name="input")
        ], style=input_row_style)
    ], style=container_style, on_click="focus_terminal")

    window.set_content(terminal_html)
    window.set_title("Terminal")

# Initial render
render()
