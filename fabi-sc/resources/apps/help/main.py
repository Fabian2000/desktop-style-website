# FabiScOS Help
# Documentation and Programming Guide

import fabiscos_ui as ui
import fabiscos_window as window
import fabiscos_state as state

# Styles
# Main container - like Terminal: height 100%, flex column
container_style = ui.style(
    background="#1a1a2e",
    height="100%",
    display="flex",
    flex_direction="row",
    box_sizing="border-box"
)

# Mobile container - column layout
mobile_container_style = ui.style(
    background="#1a1a2e",
    height="100%",
    display="flex",
    flex_direction="column",
    box_sizing="border-box"
)

sidebar_style = ui.style(
    background="#16213e",
    padding="8px",
    min_width="160px",
    max_width="160px",
    border_right="1px solid #0f3460",
    overflow_y="auto",
    display="flex",
    flex_direction="column",
    gap="4px",
    flex_shrink="0",
    height="100%"
)

# Mobile sidebar (full width, collapsible)
mobile_sidebar_style = ui.style(
    background="#16213e",
    padding="8px",
    border_bottom="1px solid #0f3460",
    display="flex",
    flex_direction="column",
    gap="4px"
)

# Mobile header with burger
mobile_header_style = ui.style(
    background="#16213e",
    padding="8px 12px",
    display="flex",
    align_items="center",
    justify_content="space-between",
    border_bottom="1px solid #0f3460"
)

burger_btn_style = ui.style(
    background="transparent",
    border="none",
    color="#94a3b8",
    font_size="18px",
    cursor="pointer",
    padding="4px 8px"
)

content_style = ui.style(
    flex="1",
    padding="16px",
    overflow_y="auto",
    color="#e0e0e0",
    line_height="1.5",
    background="#1a1a2e",
    min_height="0",
    word_wrap="break-word",
    overflow_wrap="break-word"
)

nav_btn_style = ui.style(
    background="transparent",
    border="1px solid #334155",
    color="#94a3b8",
    padding="6px 10px",
    cursor="pointer",
    border_radius="4px",
    font_size="12px"
)

nav_btn_active_style = ui.style(
    background="#0f3460",
    border="1px solid #60a5fa",
    color="#60a5fa",
    padding="6px 10px",
    cursor="pointer",
    border_radius="4px",
    font_size="12px"
)

title_style = ui.style(
    font_size="20px",
    font_weight="bold",
    color="#60a5fa",
    margin_bottom="12px"
)

mobile_title_style = ui.style(
    font_size="14px",
    font_weight="bold",
    color="#60a5fa",
    margin="0"
)

code_style = ui.style(
    background="#0d1117",
    padding="10px",
    border_radius="4px",
    font_family="'Cascadia Code', 'Fira Code', monospace",
    font_size="12px",
    color="#7dd3fc",
    overflow_x="auto",
    margin="8px 0",
    white_space="pre"
)

para_style = ui.style(
    margin_bottom="8px",
    color="#cbd5e1",
    font_size="13px"
)

# Content pages
pages = {
    "welcome": {
        "title": "Welcome to FabiScOS",
        "content": """FabiScOS is a virtual desktop OS running entirely in your browser.

Built with Rust, WebAssembly and Python.

Features:
  - Virtual filesystem (VFS) with persistence
  - Python apps running in browser
  - Desktop and mobile views
  - Window management (drag, resize, minimize)

Navigation:
  - Desktop: Click apps in taskbar
  - Mobile: Use Back, Home and Recents buttons"""
    },
    "filesystem": {
        "title": "Filesystem (VFS)",
        "content": """FabiScOS has a virtual filesystem stored in IndexedDB.

Directory structure:
  /home/
    Desktop/     - Desktop files
    Documents/   - Documents
    Pictures/    - Images
    apps/        - Your custom apps
    .system/     - System files (hidden)
      apps/      - System apps (Terminal, etc.)

Terminal commands:
  ls          - List directory
  cd <path>   - Change directory
  pwd         - Print working directory
  cat <file>  - Show file contents
  mkdir <dir> - Create directory
  touch <file>- Create file
  rm <file>   - Delete file
  cp <s> <d>  - Copy file
  mv <s> <d>  - Move file"""
    },
    "programming": {
        "title": "Programming Apps",
        "content": """Apps are written in Python and run in the browser.

App structure:
  /home/apps/my_app/
    metadata.json  - App info
    main.py        - Main code

metadata.json example:
  {
    "id": "my_app",
    "name": "My App",
    "version": "1.0.0",
    "icon": "icon.png",
    "entry": "main.py"
  }

Minimal main.py:
  import fabiscos_ui as ui
  import fabiscos_window as window

  window.set_content(
      ui.text("Hello World!")
  )
  window.set_title("My App")"""
    },
    "ui": {
        "title": "UI API",
        "content": """import fabiscos_ui as ui

Create styles:
  style = ui.style(color="#fff", padding="10px")

Widgets:
  ui.text(content, style=s)     - Multiline text
  ui.label(content, style=s)    - Single line
  ui.button(text, on_click="fn", style=s)
  ui.input(placeholder, on_submit="fn", style=s)
  ui.image(src, alt="", style=s)
  ui.divider(style=s)

Containers:
  ui.container([...], style=s)  - Generic
  ui.row([...], style=s)        - Horizontal flex
  ui.column([...], style=s)     - Vertical flex
  ui.spacer()                   - Flexible space
  ui.desktop_only([...])        - Hidden on mobile
  ui.mobile_only([...])         - Hidden on desktop

Style properties (snake_case):
  background, color, padding, margin
  font_size, font_family, font_weight
  border, border_radius
  display, flex, flex_direction
  width, height, overflow"""
    },
    "window": {
        "title": "Window API",
        "content": """import fabiscos_window as window

Window control:
  window.set_title(title)    - Set title
  window.set_content(html)   - Set UI
  window.close()             - Close window

Event handlers:
  def on_click_button():
      # Called when button clicked
      print("Button clicked!")
      render()

  def on_input(value):
      # Called on Enter in input
      print("Input:", value)
      render()

  def on_back():
      # Mobile back button
      window.close()

Button with handler:
  ui.button("Click Me", on_click="on_click_button")

Input with handler:
  ui.input("", on_submit="on_input")"""
    },
    "vfs": {
        "title": "VFS API",
        "content": """import fabiscos_vfs as vfs

Read/write files:
  vfs.read_text(path)        - Read text
  vfs.write(path, content)   - Write text
  vfs.exists(path)           - Exists?

Directories:
  vfs.list_dir(path)         - List contents
  vfs.mkdir(path)            - Create dir

Operations:
  vfs.remove(path)           - Delete
  vfs.copy(src, dst)         - Copy
  vfs.move(src, dst)         - Move

Working directory:
  vfs.cwd()                  - Get current
  vfs.set_cwd(path)          - Set current"""
    },
    "state": {
        "title": "State API",
        "content": """import fabiscos_state as state

State persists between app runs.

Store values:
  state.set(key, value)      - Store string
  state.get(key)             - Read string

Lists:
  state.set_list(key, list)  - Store list
  state.get_list(key)        - Read list

Example (Counter):
  count = state.get("count") or "0"
  count = int(count) + 1
  state.set("count", str(count))

Example (History):
  lines = state.get_list("output") or []
  lines.append("New line")
  state.set_list("output", lines)"""
    },
    "example": {
        "title": "Example App",
        "content": """Complete counter app:

  import fabiscos_ui as ui
  import fabiscos_window as window
  import fabiscos_state as state

  count = int(state.get("count") or "0")

  def on_plus():
      global count
      count += 1
      state.set("count", str(count))
      render()

  def on_minus():
      global count
      count -= 1
      state.set("count", str(count))
      render()

  def on_back():
      window.close()

  def render():
      window.set_content(
          ui.column([
              ui.label(f"Count: {count}",
                  style=ui.style(font_size="32px")),
              ui.row([
                  ui.button("-", on_click="on_minus"),
                  ui.button("+", on_click="on_plus")
              ])
          ], style=ui.style(padding="20px"))
      )
      window.set_title("Counter")

  render()"""
    }
}

nav_labels = [
    ("welcome", "Welcome"),
    ("filesystem", "Files"),
    ("programming", "Coding"),
    ("ui", "UI"),
    ("window", "Window"),
    ("vfs", "VFS"),
    ("state", "State"),
    ("example", "Example")
]

# Current page and mobile menu state
current_page = state.get("page") or "welcome"
menu_open = state.get("menu_open") == "1"

# Toggle mobile menu
def toggle_menu():
    global menu_open
    menu_open = not menu_open
    state.set("menu_open", "1" if menu_open else "0")
    render()

# Navigation functions - one for each page
def nav_welcome():
    global current_page, menu_open
    current_page = "welcome"
    menu_open = False
    state.set("page", current_page)
    state.set("menu_open", "0")
    render()

def nav_filesystem():
    global current_page, menu_open
    current_page = "filesystem"
    menu_open = False
    state.set("page", current_page)
    state.set("menu_open", "0")
    render()

def nav_programming():
    global current_page, menu_open
    current_page = "programming"
    menu_open = False
    state.set("page", current_page)
    state.set("menu_open", "0")
    render()

def nav_ui():
    global current_page, menu_open
    current_page = "ui"
    menu_open = False
    state.set("page", current_page)
    state.set("menu_open", "0")
    render()

def nav_window():
    global current_page, menu_open
    current_page = "window"
    menu_open = False
    state.set("page", current_page)
    state.set("menu_open", "0")
    render()

def nav_vfs():
    global current_page, menu_open
    current_page = "vfs"
    menu_open = False
    state.set("page", current_page)
    state.set("menu_open", "0")
    render()

def nav_state():
    global current_page, menu_open
    current_page = "state"
    menu_open = False
    state.set("page", current_page)
    state.set("menu_open", "0")
    render()

def nav_example():
    global current_page, menu_open
    current_page = "example"
    menu_open = False
    state.set("page", current_page)
    state.set("menu_open", "0")
    render()

# Map page keys to handler names
nav_handlers = {
    "welcome": "nav_welcome",
    "filesystem": "nav_filesystem",
    "programming": "nav_programming",
    "ui": "nav_ui",
    "window": "nav_window",
    "vfs": "nav_vfs",
    "state": "nav_state",
    "example": "nav_example"
}

def on_back():
    window.close()

def render():
    page = pages.get(current_page, pages["welcome"])

    # Build navigation buttons with click handlers
    nav_items = []
    for key, label in nav_labels:
        style = nav_btn_active_style if key == current_page else nav_btn_style
        handler = nav_handlers[key]
        nav_items.append(ui.button(label, style=style, on_click=handler))

    # Build content
    content_parts = [ui.label(page["title"], style=title_style)]

    # Simple text display with code formatting
    lines = page["content"].split("\n")
    text_block = []
    in_code = False

    for line in lines:
        # Detect code blocks (lines starting with 2+ spaces after non-code)
        is_code_line = line.startswith("  ") and line.strip()

        if is_code_line and not in_code:
            # Flush text
            if text_block:
                content_parts.append(ui.text("\n".join(text_block), style=para_style))
                text_block = []
            in_code = True
            text_block.append(line)
        elif not is_code_line and in_code and line.strip():
            # End code, start text
            content_parts.append(ui.text("\n".join(text_block), style=code_style))
            text_block = [line]
            in_code = False
        else:
            text_block.append(line)

    # Flush remaining
    if text_block:
        style = code_style if in_code else para_style
        content_parts.append(ui.text("\n".join(text_block), style=style))

    # Get current page title for mobile header
    page_title = next((label for key, label in nav_labels if key == current_page), "Help")

    # Desktop layout: direct container with sidebar and content (like Terminal)
    desktop_wrapper_style = ui.style(height="100%")
    desktop_html = ui.desktop_only([
        ui.container([
            ui.column(nav_items, style=sidebar_style),
            ui.container(content_parts, style=content_style)  # container statt column für normales Scrolling
        ], style=container_style)
    ], style=desktop_wrapper_style)

    # Mobile layout: header with burger, collapsible menu, content
    mobile_parts = []

    # Header with burger and title
    burger_icon_class = "fa-solid fa-xmark" if menu_open else "fa-solid fa-bars"
    mobile_parts.append(
        ui.row([
            ui.button("", icon=burger_icon_class, style=burger_btn_style, on_click="toggle_menu"),
            ui.label(page_title, style=mobile_title_style)
        ], style=mobile_header_style)
    )

    # Collapsible navigation (shown when menu_open)
    if menu_open:
        mobile_parts.append(ui.column(nav_items, style=mobile_sidebar_style))

    # Content area
    mobile_parts.append(ui.container(content_parts, style=content_style))

    mobile_html = ui.mobile_only([
        ui.column(mobile_parts, style=mobile_container_style)
    ])

    # Combine both layouts - needs height 100% to fill .app-ui
    wrapper_style = ui.style(height="100%")
    html = ui.container([desktop_html, mobile_html], style=wrapper_style)

    window.set_content(html)
    window.set_title("Help")

render()
