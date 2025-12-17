# FabiScOS Text Editor
# Simple text editor for viewing and editing files

import fabiscos_ui as ui
import fabiscos_vfs as vfs
import fabiscos_window as window
import fabiscos_state as state

# Styles
container_style = ui.style(
    background="#1e1e2e",
    height="100%",
    display="flex",
    flex_direction="column",
    box_sizing="border-box"
)

toolbar_style = ui.style(
    background="#181825",
    padding="8px 12px",
    display="flex",
    flex_direction="row",
    align_items="center",
    gap="8px",
    border_bottom="1px solid #313244",
    flex_shrink="0"
)

btn_style = ui.style(
    background="#313244",
    border="none",
    border_radius="4px",
    color="#cdd6f4",
    padding="8px 12px",
    cursor="pointer",
    font_size="13px"
)

btn_primary_style = ui.style(
    background="#89b4fa",
    border="none",
    border_radius="4px",
    color="#1e1e2e",
    padding="8px 12px",
    cursor="pointer",
    font_size="13px",
    font_weight="bold"
)

filename_style = ui.style(
    color="#6c7086",
    font_size="13px",
    flex="1",
    overflow="hidden",
    text_overflow="ellipsis",
    white_space="nowrap"
)

editor_style = ui.style(
    flex="1",
    background="#11111b",
    border="none",
    color="#cdd6f4",
    font_family="'Cascadia Code', 'Fira Code', 'Consolas', monospace",
    font_size="14px",
    padding="12px",
    resize="none",
    outline="none",
    overflow="auto",
    white_space="pre",
    box_sizing="border-box"
)

status_bar_style = ui.style(
    background="#181825",
    padding="4px 12px",
    border_top="1px solid #313244",
    color="#6c7086",
    font_size="12px",
    display="flex",
    justify_content="space-between",
    flex_shrink="0"
)

# Modal styles
modal_overlay_style = ui.style(
    position="fixed",
    top="0",
    left="0",
    right="0",
    bottom="0",
    background="rgba(0,0,0,0.5)",
    display="flex",
    align_items="center",
    justify_content="center",
    z_index="1000"
)

modal_style = ui.style(
    background="#1e1e2e",
    border="1px solid #313244",
    border_radius="8px",
    padding="20px",
    min_width="300px",
    max_width="400px"
)

modal_title_style = ui.style(
    color="#cdd6f4",
    font_size="16px",
    font_weight="bold",
    margin_bottom="16px",
    display="block"
)

modal_input_style = ui.style(
    background="#11111b",
    border="1px solid #313244",
    border_radius="4px",
    padding="8px 12px",
    color="#cdd6f4",
    font_size="14px",
    width="100%",
    margin_bottom="16px",
    box_sizing="border-box"
)

modal_btn_row_style = ui.style(
    display="flex",
    justify_content="flex-end",
    gap="8px"
)

# State
_saved_path = state.get("file_path")
current_file = _saved_path if _saved_path else None
_saved_content = state.get("content")
content = _saved_content if _saved_content else ""
_saved_modified = state.get("modified")
modified = _saved_modified == "true" if _saved_modified else False
modal_type = None  # "save_as", "unsaved_warning"

def save_state():
    state.set("file_path", current_file if current_file else "")
    state.set("content", content)
    state.set("modified", "true" if modified else "false")

def load_file(path):
    global current_file, content, modified
    try:
        content = vfs.read_text(path)
        current_file = path
        modified = False
        save_state()
    except Exception as e:
        print(f"[Editor] Error loading file: {e}")
        # Still set the path and show empty content on error
        current_file = path
        content = f"Error loading file: {e}"
        modified = False
    render()

def save_file():
    global modified
    if current_file:
        try:
            vfs.write(current_file, content)
            modified = False
            save_state()
            render()
        except Exception as e:
            print(f"[Editor] Error saving file: {e}")
    else:
        # No file path - open Save As dialog
        show_save_as()

def save_file_as(path):
    global current_file, modified
    if path and path.strip():
        try:
            vfs.write(path.strip(), content)
            current_file = path.strip()
            modified = False
            save_state()
            render()
        except Exception as e:
            print(f"[Editor] Error saving file: {e}")

def new_file():
    global current_file, content, modified
    current_file = None
    content = ""
    modified = False
    save_state()
    render()

def show_save_as():
    global modal_type
    modal_type = "save_as"
    render()

def close_modal():
    global modal_type
    modal_type = None
    render()

def on_input(value):
    """Called when Enter is pressed in modal input"""
    global modal_type
    if modal_type == "save_as":
        save_file_as(value)
        modal_type = None
        render()

def on_text_change(value):
    """Called when text content changes"""
    global content, modified
    content = value
    modified = True
    save_state()

def on_back():
    window.close()

def get_filename():
    if current_file:
        return current_file.split("/")[-1]
    return "Untitled"

def get_line_count():
    return len(content.split("\n"))

def build_modal():
    if not modal_type:
        return None

    modal_content = []
    hint_style = ui.style(color="#6c7086", font_size="12px", margin_bottom="8px", display="block")

    if modal_type == "save_as":
        default_path = current_file if current_file else "/home/Desktop/untitled.txt"
        modal_content.append(ui.label("Save As", style=modal_title_style))
        modal_content.append(ui.label("Enter file path and press Enter:", style=hint_style))
        modal_content.append(ui.input(default_path, style=modal_input_style, name="modal_input", on_submit="save_file_as"))
        modal_content.append(ui.row([
            ui.button("Cancel", style=btn_style, on_click="close_modal"),
        ], style=modal_btn_row_style))

    return ui.container([
        ui.container(modal_content, style=modal_style)
    ], style=modal_overlay_style)

def render():
    # Toolbar
    toolbar_items = [
        ui.button("New", icon="fa-solid fa-file", style=btn_style, on_click="new_file"),
        ui.button("Save", icon="fa-solid fa-floppy-disk", style=btn_primary_style if modified else btn_style, on_click="save_file"),
        ui.button("Save As", icon="fa-solid fa-file-export", style=btn_style, on_click="show_save_as"),
    ]

    filename_display = get_filename()
    if modified:
        filename_display += " *"
    toolbar_items.append(ui.label(filename_display, style=filename_style))

    toolbar = ui.container(toolbar_items, style=toolbar_style)

    # Editor area - textarea for editing
    editor = ui.textarea(content, style=editor_style, name="editor", on_change="on_text_change")

    # Status bar
    line_count = get_line_count()
    char_count = len(content)
    status_left = f"{line_count} lines, {char_count} characters"
    status_right = current_file if current_file else "New file"

    status_bar = ui.container([
        ui.label(status_left, style=ui.style(color="#6c7086", font_size="12px")),
        ui.label(status_right, style=ui.style(color="#6c7086", font_size="12px")),
    ], style=status_bar_style)

    # Main layout
    parts = [toolbar, editor, status_bar]

    # Add modal if needed
    modal = build_modal()
    if modal:
        parts.append(modal)

    html = ui.container(parts, style=container_style)
    window.set_content(html)

    title = "Editor"
    if current_file:
        title = f"Editor - {get_filename()}"
    if modified:
        title += " *"
    window.set_title(title)

# Check if this is a change event (textarea typing) - don't re-render
if '__change_handler__' in dir():
    # Change event - handler was already called, don't render
    pass
# Check if we should load a file (passed via args)
# __args__ is set by the system when launching with "Open with"
elif '__args__' in dir() and __args__ and len(__args__) > 0:
    # First arg is the file path
    _file_to_open = __args__[0]
    # Only load on first run (check if we already have this file loaded)
    if current_file != _file_to_open:
        load_file(_file_to_open)
    else:
        render()
else:
    # Initial render
    render()
