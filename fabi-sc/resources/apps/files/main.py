# FabiScOS File Explorer
# Browse and manage files in the virtual filesystem

import fabiscos_ui as ui
import fabiscos_vfs as vfs
import fabiscos_window as window
import fabiscos_state as state

# Styles
container_style = ui.style(
    background="#1e1e2e",
    height="100%",
    display="flex",
    flex_direction="row",
    box_sizing="border-box"
)

mobile_container_style = ui.style(
    background="#1e1e2e",
    height="100%",
    display="flex",
    flex_direction="column",
    box_sizing="border-box"
)

# Sidebar styles
sidebar_style = ui.style(
    background="#181825",
    min_width="160px",
    max_width="160px",
    border_right="1px solid #313244",
    overflow_y="auto",
    display="flex",
    flex_direction="column",
    padding="8px",
    gap="2px",
    flex_shrink="0",
    height="100%"
)

sidebar_item_style = ui.style(
    display="flex",
    flex_direction="row",
    align_items="center",
    padding="8px 10px",
    border_radius="4px",
    cursor="pointer",
    gap="8px",
    color="#cdd6f4",
    font_size="13px"
)

sidebar_item_active_style = ui.style(
    display="flex",
    flex_direction="row",
    align_items="center",
    padding="8px 10px",
    border_radius="4px",
    cursor="pointer",
    gap="8px",
    color="#cdd6f4",
    font_size="13px",
    background="#313244"
)

sidebar_label_style = ui.style(
    color="#6c7086",
    font_size="11px",
    padding="8px 10px 4px 10px",
    text_transform="uppercase"
)

# Main content area
content_style = ui.style(
    flex="1",
    display="flex",
    flex_direction="column",
    height="100%",
    min_width="0"
)

toolbar_style = ui.style(
    background="#181825",
    padding="8px 12px",
    display="flex",
    flex_direction="row",
    align_items="center",
    gap="8px",
    border_bottom="1px solid #313244"
)

# Mobile-specific styles
mobile_toolbar_style = ui.style(
    background="#181825",
    padding="8px 12px",
    display="flex",
    flex_direction="row",
    align_items="center",
    gap="8px",
    border_bottom="1px solid #313244"
)

mobile_bottom_bar_style = ui.style(
    background="#181825",
    padding="10px 16px",
    display="flex",
    flex_direction="row",
    align_items="center",
    justify_content="space-around",
    gap="8px",
    border_top="1px solid #313244",
    flex_shrink="0"
)

path_bar_style = ui.style(
    background="#11111b",
    border="1px solid #313244",
    border_radius="4px",
    padding="6px 12px",
    color="#cdd6f4",
    font_size="13px",
    flex="1"
)

btn_icon_style = ui.style(
    background="#313244",
    border="none",
    border_radius="4px",
    color="#cdd6f4",
    padding="8px 10px",
    cursor="pointer"
)

file_list_style = ui.style(
    flex="1",
    overflow_y="auto",
    padding="8px"
)

file_item_style = ui.style(
    display="flex",
    flex_direction="row",
    align_items="center",
    padding="8px 12px",
    border_radius="4px",
    cursor="pointer",
    gap="10px",
    margin_bottom="2px"
)

file_item_selected_style = ui.style(
    display="flex",
    flex_direction="row",
    align_items="center",
    padding="8px 12px",
    border_radius="4px",
    cursor="pointer",
    gap="10px",
    margin_bottom="2px",
    background="#45475a"
)

file_name_style = ui.style(
    color="#cdd6f4",
    flex="1",
    font_size="13px",
    overflow="hidden",
    text_overflow="ellipsis",
    white_space="nowrap"
)

empty_style = ui.style(
    color="#6c7086",
    text_align="center",
    padding="40px",
    font_size="14px"
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
    margin_bottom="16px"
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

modal_btn_primary_style = ui.style(
    background="#89b4fa",
    border="none",
    border_radius="4px",
    color="#1e1e2e",
    padding="8px 16px",
    cursor="pointer",
    font_weight="bold"
)

modal_btn_cancel_style = ui.style(
    background="#45475a",
    border="none",
    border_radius="4px",
    color="#cdd6f4",
    padding="8px 16px",
    cursor="pointer"
)

# Sidebar locations
sidebar_locations = [
    {"name": "Desktop", "path": "/home/Desktop", "icon": "fa-solid fa-desktop"},
    {"name": "Documents", "path": "/home/Documents", "icon": "fa-solid fa-file-lines"},
    {"name": "Downloads", "path": "/home/Downloads", "icon": "fa-solid fa-download"},
    {"name": "Pictures", "path": "/home/Pictures", "icon": "fa-solid fa-image"},
    {"name": "Music", "path": "/home/Music", "icon": "fa-solid fa-music"},
    {"name": "Videos", "path": "/home/Videos", "icon": "fa-solid fa-film"},
]

# State
_saved_path = state.get("current_path")
current_path = _saved_path if _saved_path else "/home/Desktop"
_saved_index = state.get("selected_index")
selected_index = int(_saved_index) if _saved_index else -1
_saved_modal = state.get("modal_type")
modal_type = _saved_modal if _saved_modal else None  # "new_file", "new_folder", "rename", "delete", "open_with"
entries = []
available_apps = []  # Apps that can open selected file

def save_state():
    state.set("current_path", current_path)
    state.set("selected_index", str(selected_index))
    state.set("modal_type", modal_type if modal_type else "")

def get_icon_for_file(name, is_dir):
    if is_dir:
        return "fa-solid fa-folder"
    ext = name.split(".")[-1].lower() if "." in name else ""
    icons = {
        "txt": "fa-solid fa-file-lines",
        "md": "fa-solid fa-file-lines",
        "py": "fa-solid fa-file-code",
        "js": "fa-solid fa-file-code",
        "json": "fa-solid fa-file-code",
        "html": "fa-solid fa-file-code",
        "css": "fa-solid fa-file-code",
        "png": "fa-solid fa-file-image",
        "jpg": "fa-solid fa-file-image",
        "jpeg": "fa-solid fa-file-image",
        "gif": "fa-solid fa-file-image",
        "webp": "fa-solid fa-file-image",
        "svg": "fa-solid fa-file-image",
        "pdf": "fa-solid fa-file-pdf",
        "zip": "fa-solid fa-file-zipper",
        "mp3": "fa-solid fa-file-audio",
        "wav": "fa-solid fa-file-audio",
        "mp4": "fa-solid fa-file-video",
    }
    return icons.get(ext, "fa-solid fa-file")

def navigate_to(path):
    global current_path, selected_index
    current_path = path
    selected_index = -1  # Reset selection when navigating
    save_state()
    render()

def go_up():
    global current_path
    if current_path != "/home":
        parts = current_path.rstrip("/").split("/")
        if len(parts) > 2:
            current_path = "/".join(parts[:-1])
        else:
            current_path = "/home"
        save_state()
        render()

def go_home():
    navigate_to("/home")

# Sidebar navigation handlers
def nav_desktop():
    navigate_to("/home/Desktop")
def nav_documents():
    navigate_to("/home/Documents")
def nav_downloads():
    navigate_to("/home/Downloads")
def nav_pictures():
    navigate_to("/home/Pictures")
def nav_music():
    navigate_to("/home/Music")
def nav_videos():
    navigate_to("/home/Videos")
def nav_home():
    navigate_to("/home")

sidebar_handlers = {
    "/home/Desktop": "nav_desktop",
    "/home/Documents": "nav_documents",
    "/home/Downloads": "nav_downloads",
    "/home/Pictures": "nav_pictures",
    "/home/Music": "nav_music",
    "/home/Videos": "nav_videos",
    "/home": "nav_home",
}

# Click handlers for items (0-29)
def click_0():
    handle_item_click(0)
def click_1():
    handle_item_click(1)
def click_2():
    handle_item_click(2)
def click_3():
    handle_item_click(3)
def click_4():
    handle_item_click(4)
def click_5():
    handle_item_click(5)
def click_6():
    handle_item_click(6)
def click_7():
    handle_item_click(7)
def click_8():
    handle_item_click(8)
def click_9():
    handle_item_click(9)
def click_10():
    handle_item_click(10)
def click_11():
    handle_item_click(11)
def click_12():
    handle_item_click(12)
def click_13():
    handle_item_click(13)
def click_14():
    handle_item_click(14)
def click_15():
    handle_item_click(15)
def click_16():
    handle_item_click(16)
def click_17():
    handle_item_click(17)
def click_18():
    handle_item_click(18)
def click_19():
    handle_item_click(19)
def click_20():
    handle_item_click(20)
def click_21():
    handle_item_click(21)
def click_22():
    handle_item_click(22)
def click_23():
    handle_item_click(23)
def click_24():
    handle_item_click(24)
def click_25():
    handle_item_click(25)
def click_26():
    handle_item_click(26)
def click_27():
    handle_item_click(27)
def click_28():
    handle_item_click(28)
def click_29():
    handle_item_click(29)

def handle_item_click(index):
    global selected_index, entries
    if index < 0 or index >= len(entries):
        return
    entry = entries[index]

    if index == selected_index:
        # Second click on same item
        if entry["is_dir"]:
            # Open folder
            new_path = current_path.rstrip("/") + "/" + entry["name"]
            navigate_to(new_path)
        else:
            # Open file with system handler
            open_selected_file()
    else:
        # First click: select item
        selected_index = index
        save_state()
        render()

def get_file_extension(filename):
    """Get file extension without dot"""
    if "." in filename:
        return filename.split(".")[-1].lower()
    return ""

def find_apps_for_extension(ext):
    """Find all apps that can handle this file extension"""
    apps = []
    apps_dir = "/home/.system/apps/"

    try:
        app_dirs = vfs.list_dir(apps_dir)
        for app_entry in app_dirs:
            if app_entry.get("type") != "directory":
                continue

            app_path = apps_dir + app_entry.get("name") + "/"
            metadata_path = app_path + "metadata.json"

            try:
                metadata_str = vfs.read_text(metadata_path)
                import json
                metadata = json.loads(metadata_str)

                # Check if app handles this extension
                file_handlers = metadata.get("file_handlers", [])
                if ext in file_handlers or "*" in file_handlers:
                    apps.append({
                        "id": metadata.get("id", ""),
                        "name": metadata.get("name", "Unknown"),
                        "icon": metadata.get("icon", ""),
                        "path": app_path
                    })
            except:
                pass
    except:
        pass

    return apps

def open_selected_file():
    """Open the selected file using system file handler"""
    global selected_index, entries
    if selected_index < 0 or selected_index >= len(entries):
        return

    filename = entries[selected_index]["name"]
    file_path = current_path.rstrip("/") + "/" + filename

    # Use system open_file - this will show the system "Open with" dialog
    import fabiscos_system as system
    system.open_file(file_path)

def open_with_app(app_index):
    """Open file with selected app"""
    global modal_type, selected_index, entries, available_apps

    if app_index < 0 or app_index >= len(available_apps):
        modal_type = None
        render()
        return

    app = available_apps[app_index]
    filename = entries[selected_index]["name"]
    file_path = current_path.rstrip("/") + "/" + filename

    # Store file path for the app to read
    state.set("open_file_path", file_path)

    # Launch the app via system
    import fabiscos_system as system
    system.launch_app(app["id"], file_path)

    modal_type = None
    render()

# Open with app handlers (0-9)
def open_app_0():
    open_with_app(0)
def open_app_1():
    open_with_app(1)
def open_app_2():
    open_with_app(2)
def open_app_3():
    open_with_app(3)
def open_app_4():
    open_with_app(4)

def show_new_file_modal():
    global modal_type
    modal_type = "new_file"
    save_state()
    render()

def show_new_folder_modal():
    global modal_type
    modal_type = "new_folder"
    save_state()
    render()

def show_rename_modal():
    global modal_type
    if selected_index >= 0:
        modal_type = "rename"
        save_state()
        render()

def show_delete_modal():
    global modal_type
    if selected_index >= 0:
        modal_type = "delete"
        save_state()
        render()

def close_modal():
    global modal_type
    modal_type = None
    save_state()
    render()

def on_input(value):
    """Called when Enter is pressed in any input field - routes to correct handler based on modal_type"""
    global modal_type
    if modal_type == "new_file":
        confirm_new_file(value)
    elif modal_type == "new_folder":
        confirm_new_folder(value)
    elif modal_type == "rename":
        confirm_rename(value)

def confirm_new_file(value):
    global modal_type
    print(f"[Files] confirm_new_file called with value: '{value}'")
    if value and value.strip():
        path = current_path.rstrip("/") + "/" + value.strip()
        print(f"[Files] Creating file at: {path}")
        try:
            vfs.write(path, "")
            print(f"[Files] File created successfully: {path}")
        except Exception as e:
            print(f"[Files] Error creating file: {e}")
    else:
        print("[Files] No filename provided, skipping file creation")
    modal_type = None
    save_state()
    render()

def confirm_new_folder(value):
    global modal_type
    if value:
        path = current_path.rstrip("/") + "/" + value
        try:
            vfs.mkdir(path)
        except Exception as e:
            print(f"Error creating folder: {e}")
    modal_type = None
    save_state()
    render()

def confirm_rename(value):
    global modal_type, selected_index, entries
    if value and selected_index >= 0 and selected_index < len(entries):
        old_name = entries[selected_index]["name"]
        old_path = current_path.rstrip("/") + "/" + old_name
        new_path = current_path.rstrip("/") + "/" + value
        try:
            vfs.move(old_path, new_path)
        except Exception as e:
            print(f"Error renaming: {e}")
    modal_type = None
    selected_index = -1
    save_state()
    render()

def remove_recursive(path):
    """Recursively remove directory and all contents"""
    try:
        entries_to_delete = vfs.list_dir(path)
        for entry in entries_to_delete:
            name = entry.get("name", "")
            if not name:
                continue
            entry_path = path.rstrip("/") + "/" + name
            if entry.get("type") == "directory":
                remove_recursive(entry_path)
            else:
                vfs.remove(entry_path)
        # Now remove the empty directory
        vfs.remove(path)
    except Exception as e:
        raise e

def confirm_delete():
    global modal_type, selected_index, entries
    if selected_index >= 0 and selected_index < len(entries):
        name = entries[selected_index]["name"]
        path = current_path.rstrip("/") + "/" + name
        try:
            # Check if it's a directory with contents
            is_dir = entries[selected_index]["is_dir"]
            if is_dir:
                dir_entries = vfs.list_dir(path)
                if dir_entries:
                    # Has contents - remove recursively
                    remove_recursive(path)
                else:
                    # Empty directory
                    vfs.remove(path)
            else:
                # Regular file
                vfs.remove(path)
        except Exception as e:
            print(f"Error deleting: {e}")
    modal_type = None
    selected_index = -1
    save_state()
    render()

def on_back():
    if current_path != "/home":
        go_up()
    else:
        window.close()

def build_sidebar():
    """Build the sidebar with quick access locations"""
    items = []

    # Home
    home_style = sidebar_item_active_style if current_path == "/home" else sidebar_item_style
    items.append(ui.container([
        ui.button("", icon="fa-solid fa-house", style=ui.style(background="transparent", border="none", color="#cdd6f4", padding="0", pointer_events="none")),
        ui.label("Home", style=ui.style(color="#cdd6f4", font_size="13px")),
    ], style=home_style, on_click="nav_home"))

    # Separator label
    items.append(ui.label("Places", style=sidebar_label_style))

    # Quick access locations
    for loc in sidebar_locations:
        is_active = current_path == loc["path"] or current_path.startswith(loc["path"] + "/")
        item_style = sidebar_item_active_style if is_active else sidebar_item_style
        handler = sidebar_handlers.get(loc["path"], "nav_home")
        items.append(ui.container([
            ui.button("", icon=loc["icon"], style=ui.style(background="transparent", border="none", color="#89b4fa", padding="0", pointer_events="none")),
            ui.label(loc["name"], style=ui.style(color="#cdd6f4", font_size="13px")),
        ], style=item_style, on_click=handler))

    return ui.column(items, style=sidebar_style)

def build_file_list():
    """Build the file list for current directory"""
    global entries

    entries = []
    try:
        raw_entries = vfs.list_dir(current_path)
        for e in raw_entries:
            name = e.get("name", "")
            ftype = e.get("type", "file")
            if name.startswith(".") and ".system" not in current_path:
                continue
            entries.append({
                "name": name,
                "is_dir": ftype == "directory"
            })
        entries.sort(key=lambda x: (not x["is_dir"], x["name"].lower()))
    except Exception as e:
        print(f"Error reading directory: {e}")

    if not entries:
        return ui.container([
            ui.label("This folder is empty", style=empty_style)
        ], style=file_list_style)

    file_items = []
    for i, entry in enumerate(entries):
        if i >= 30:
            break

        name = entry["name"]
        is_dir = entry["is_dir"]
        icon = get_icon_for_file(name, is_dir)
        icon_color = "#f9e2af" if is_dir else "#89b4fa"

        item_style = file_item_selected_style if i == selected_index else file_item_style
        handler = f"click_{i}"

        item = ui.container([
            ui.button("", icon=icon, style=ui.style(background="transparent", border="none", color=icon_color, padding="0", pointer_events="none")),
            ui.label(name, style=file_name_style),
        ], style=item_style, on_click=handler)

        file_items.append(item)

    return ui.column(file_items, style=file_list_style)

def build_toolbar():
    """Build the toolbar with navigation and actions (desktop)"""
    toolbar_items = [
        ui.button("", icon="fa-solid fa-arrow-up", style=btn_icon_style, on_click="go_up"),
        ui.button("", icon="fa-solid fa-house", style=btn_icon_style, on_click="go_home"),
        ui.label(current_path, style=path_bar_style),
        ui.button("", icon="fa-solid fa-file-circle-plus", style=btn_icon_style, on_click="show_new_file_modal"),
        ui.button("", icon="fa-solid fa-folder-plus", style=btn_icon_style, on_click="show_new_folder_modal"),
    ]

    if selected_index >= 0:
        toolbar_items.extend([
            ui.button("", icon="fa-solid fa-pen", style=btn_icon_style, on_click="show_rename_modal"),
            ui.button("", icon="fa-solid fa-trash", style=btn_icon_style, on_click="show_delete_modal"),
        ])

    return ui.container(toolbar_items, style=toolbar_style)

def build_mobile_toolbar():
    """Build mobile toolbar with just navigation and path"""
    toolbar_items = [
        ui.button("", icon="fa-solid fa-arrow-up", style=btn_icon_style, on_click="go_up"),
        ui.label(current_path, style=path_bar_style),
    ]
    return ui.container(toolbar_items, style=mobile_toolbar_style)

def build_mobile_bottom_bar():
    """Build mobile bottom bar with action buttons"""
    bottom_items = [
        ui.button("", icon="fa-solid fa-house", style=btn_icon_style, on_click="go_home"),
        ui.button("", icon="fa-solid fa-file-circle-plus", style=btn_icon_style, on_click="show_new_file_modal"),
        ui.button("", icon="fa-solid fa-folder-plus", style=btn_icon_style, on_click="show_new_folder_modal"),
    ]

    if selected_index >= 0:
        bottom_items.extend([
            ui.button("", icon="fa-solid fa-pen", style=btn_icon_style, on_click="show_rename_modal"),
            ui.button("", icon="fa-solid fa-trash", style=btn_icon_style, on_click="show_delete_modal"),
        ])

    return ui.container(bottom_items, style=mobile_bottom_bar_style)

def build_modal():
    """Build modal dialog if needed"""
    if not modal_type:
        return None

    modal_content = []

    # Style for hint text - needs display block to be on its own line
    hint_style = ui.style(color="#6c7086", font_size="12px", margin_bottom="8px", display="block")

    if modal_type == "new_file":
        modal_content.append(ui.label("Create New File", style=modal_title_style))
        modal_content.append(ui.label("Enter filename and press Enter:", style=hint_style))
        modal_content.append(ui.input("filename.txt", style=modal_input_style, name="modal_input", on_submit="confirm_new_file"))
        modal_content.append(ui.row([
            ui.button("Cancel", style=modal_btn_cancel_style, on_click="close_modal"),
        ], style=modal_btn_row_style))

    elif modal_type == "new_folder":
        modal_content.append(ui.label("Create New Folder", style=modal_title_style))
        modal_content.append(ui.label("Enter folder name and press Enter:", style=hint_style))
        modal_content.append(ui.input("New Folder", style=modal_input_style, name="modal_input", on_submit="confirm_new_folder"))
        modal_content.append(ui.row([
            ui.button("Cancel", style=modal_btn_cancel_style, on_click="close_modal"),
        ], style=modal_btn_row_style))

    elif modal_type == "rename":
        old_name = entries[selected_index]["name"] if selected_index >= 0 and selected_index < len(entries) else ""
        modal_content.append(ui.label("Rename", style=modal_title_style))
        modal_content.append(ui.label("Enter new name and press Enter:", style=hint_style))
        modal_content.append(ui.input(old_name, style=modal_input_style, name="modal_input", on_submit="confirm_rename"))
        modal_content.append(ui.row([
            ui.button("Cancel", style=modal_btn_cancel_style, on_click="close_modal"),
        ], style=modal_btn_row_style))

    elif modal_type == "delete":
        name = entries[selected_index]["name"] if selected_index >= 0 and selected_index < len(entries) else ""
        modal_content.append(ui.label("Delete", style=modal_title_style))
        modal_content.append(ui.text(f"Are you sure you want to delete '{name}'?", style=ui.style(
            color="#cdd6f4",
            margin_bottom="16px",
            display="block",
            overflow="hidden",
            text_overflow="ellipsis",
            white_space="nowrap"
        )))
        modal_content.append(ui.row([
            ui.button("Cancel", style=modal_btn_cancel_style, on_click="close_modal"),
            ui.button("Delete", style=modal_btn_primary_style, on_click="confirm_delete"),
        ], style=modal_btn_row_style))

    elif modal_type == "open_with":
        name = entries[selected_index]["name"] if selected_index >= 0 and selected_index < len(entries) else ""
        modal_content.append(ui.label("Open with", style=modal_title_style))
        modal_content.append(ui.label(f"Choose an app to open '{name}'", style=ui.style(
            color="#6c7086",
            margin_bottom="16px",
            font_size="13px",
            overflow="hidden",
            text_overflow="ellipsis",
            white_space="nowrap"
        )))

        # List available apps
        app_list_style = ui.style(
            display="flex",
            flex_direction="column",
            gap="4px",
            margin_bottom="16px"
        )
        app_btn_style = ui.style(
            display="flex",
            flex_direction="row",
            align_items="center",
            gap="10px",
            padding="10px 12px",
            background="#313244",
            border="none",
            border_radius="4px",
            color="#cdd6f4",
            cursor="pointer",
            font_size="13px",
            text_align="left"
        )

        app_items = []
        for i, app in enumerate(available_apps):
            if i >= 5:
                break
            handler = f"open_app_{i}"
            app_items.append(ui.container([
                ui.button("", icon="fa-solid fa-cube", style=ui.style(background="transparent", border="none", color="#89b4fa", padding="0", pointer_events="none")),
                ui.label(app["name"], style=ui.style(color="#cdd6f4", font_size="13px")),
            ], style=app_btn_style, on_click=handler))

        modal_content.append(ui.column(app_items, style=app_list_style))
        modal_content.append(ui.row([
            ui.button("Cancel", style=modal_btn_cancel_style, on_click="close_modal"),
        ], style=modal_btn_row_style))

    return ui.container([
        ui.container(modal_content, style=modal_style)
    ], style=modal_overlay_style)

def render():
    # Desktop layout: sidebar + content
    sidebar = build_sidebar()
    toolbar = build_toolbar()
    file_list = build_file_list()

    content_area = ui.container([toolbar, file_list], style=content_style)

    desktop_layout = ui.desktop_only([
        ui.container([sidebar, content_area], style=container_style)
    ], style=ui.style(height="100%"))

    # Mobile layout: toolbar at top, file list in middle, bottom bar for actions
    mobile_toolbar = build_mobile_toolbar()
    mobile_file_list = build_file_list()
    mobile_bottom_bar = build_mobile_bottom_bar()

    mobile_layout = ui.mobile_only([
        ui.container([mobile_toolbar, mobile_file_list, mobile_bottom_bar], style=mobile_container_style)
    ], style=ui.style(height="100%"))

    parts = [desktop_layout, mobile_layout]

    # Add modal if needed
    modal = build_modal()
    if modal:
        parts.append(modal)

    html = ui.container(parts, style=ui.style(height="100%"))
    window.set_content(html)
    window.set_title(f"Files - {current_path}")

# Initial render
render()
