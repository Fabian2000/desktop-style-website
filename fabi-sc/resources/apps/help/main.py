# FabiScOS Help
# Complete API Documentation

import fabiscos_ui as ui
import fabiscos_window as window
import fabiscos_state as state

# ============ Styles ============
container_style = ui.style(
    background="#1a1a2e",
    height="100%",
    display="flex",
    flex_direction="row",
    box_sizing="border-box"
)

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
    min_width="140px",
    max_width="140px",
    border_right="1px solid #0f3460",
    overflow_y="auto",
    display="flex",
    flex_direction="column",
    gap="2px",
    flex_shrink="0",
    height="100%"
)

mobile_sidebar_style = ui.style(
    background="#16213e",
    padding="8px",
    border_bottom="1px solid #0f3460",
    display="flex",
    flex_direction="column",
    gap="4px"
)

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
    overflow_x="hidden",
    color="#e0e0e0",
    line_height="1.5",
    background="#1a1a2e",
    min_height="0",
    width="100%",
    box_sizing="border-box",
    word_wrap="break-word",
    overflow_wrap="break-word"
)

nav_btn_style = ui.style(
    background="transparent",
    border="1px solid #334155",
    color="#94a3b8",
    padding="4px 8px",
    cursor="pointer",
    border_radius="4px",
    font_size="11px",
    text_align="left"
)

nav_btn_active_style = ui.style(
    background="#0f3460",
    border="1px solid #60a5fa",
    color="#60a5fa",
    padding="4px 8px",
    cursor="pointer",
    border_radius="4px",
    font_size="11px",
    text_align="left"
)

section_style = ui.style(
    background="#0f3460",
    color="#94a3b8",
    padding="4px 8px",
    font_size="10px",
    font_weight="bold",
    margin_top="8px",
    margin_bottom="4px",
    border_radius="2px"
)

title_style = ui.style(
    font_size="18px",
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
    font_size="11px",
    color="#7dd3fc",
    margin="8px 0",
    white_space="pre-wrap",
    word_break="break-word"
)

para_style = ui.style(
    margin_bottom="8px",
    color="#cbd5e1",
    font_size="12px"
)

# ============ Content Pages ============
pages = {
    "welcome": {
        "title": "Welcome to FabiScOS",
        "content": """FabiScOS is a virtual desktop operating system running entirely in your browser.

Built with Rust, WebAssembly and Python.

Features:
  - Virtual filesystem (VFS) with IndexedDB persistence
  - Python apps with full API access
  - Desktop and mobile responsive views
  - Window management (drag, resize, minimize, maximize)
  - Session management across browser tabs

Available APIs:
  Core:
  - fabiscos_ui - UI widgets and styling
  - fabiscos_window - Window control
  - fabiscos_vfs - Virtual filesystem
  - fabiscos_csv - CSV parsing/writing
  - fabiscos_archive - ZIP file handling
  - fabiscos_state - Persistent state storage

  Utility:
  - fabiscos_time - Time and date functions
  - fabiscos_random - Random number generation
  - fabiscos_base64 - Base64 encoding/decoding
  - fabiscos_hash - Cryptographic hashes
  - fabiscos_crypto - AES-256 encryption
  - fabiscos_http - HTTP requests
  - fabiscos_notify - Notifications"""
    },
    "filesystem": {
        "title": "Filesystem Structure",
        "content": """FabiScOS uses a virtual filesystem stored in IndexedDB.

Directory structure:
  /home/
    Desktop/       - Desktop shortcuts
    Documents/     - User documents
    Pictures/      - Images
    Music/         - Audio files
    Videos/        - Video files
    Downloads/     - Downloaded files
    apps/          - User-created apps
    .system/       - System files (protected)
      apps/        - System apps (Terminal, Help, etc.)
    .Trash/        - Deleted files (auto-cleanup after 5 days)

Protected paths:
  - /home/.system/ cannot be modified by apps
  - System apps are read-only
  - .Trash directory itself cannot be deleted

Path rules:
  - All paths start with /home/
  - Maximum path length: 4096 characters
  - Maximum filename: 255 characters
  - Forbidden: < > : " | ? * and control characters"""
    },
    "programming": {
        "title": "Creating Apps",
        "content": """Apps are Python scripts that run in the browser via RustPython.

App structure:
  /home/apps/my_app/
    metadata.json  - App configuration
    main.py        - Entry point
    icon.png       - App icon (optional)

metadata.json:
  {
    "id": "my_app",
    "name": "My App",
    "version": "1.0.0",
    "author": "Your Name",
    "description": "What the app does",
    "icon": "icon.png",
    "entry": "main.py",
    "min_width": 400,
    "min_height": 300
  }

Minimal main.py:
  import fabiscos_ui as ui
  import fabiscos_window as window

  window.set_content(ui.text("Hello World!"))
  window.set_title("My App")

Event-driven pattern:
  import fabiscos_ui as ui
  import fabiscos_window as window

  def on_click():
      print("Button clicked!")
      render()

  def on_back():
      window.close()

  def render():
      window.set_content(
          ui.button("Click Me", on_click="on_click")
      )
      window.set_title("My App")

  render()"""
    },
    "ui": {
        "title": "UI API",
        "content": """import fabiscos_ui as ui

Styling:
  style = ui.style(
      color="#ffffff",
      background="#1a1a2e",
      padding="10px",
      margin="5px",
      border="1px solid #333",
      border_radius="4px",
      font_size="14px",
      font_weight="bold",
      display="flex",
      flex_direction="column",
      gap="8px",
      width="100%",
      height="auto",
      overflow="auto"
  )

Text widgets:
  ui.text(content, style=s)      - Multiline text (pre-wrap)
  ui.label(content, style=s)     - Single line text

Interactive widgets:
  ui.button(text, on_click="fn", icon="fa-class", style=s)
  ui.input(placeholder, value="", on_change="fn",
           on_submit="fn", style=s)
  ui.textarea(placeholder, value="", rows=4,
              on_change="fn", style=s)
  ui.checkbox(label, checked=False, on_change="fn", style=s)
  ui.radio(label, name="group", value="v",
           checked=False, on_change="fn", style=s)
  ui.select(options=["a","b"], value="a",
            on_change="fn", style=s)

Media:
  ui.image(src, alt="", style=s)
  ui.divider(style=s)

Containers:
  ui.container([child1, child2], style=s)
  ui.row([...], style=s)         - display: flex
  ui.column([...], style=s)      - flex-direction: column
  ui.spacer()                    - flex: 1

Responsive:
  ui.desktop_only([...], style=s)
  ui.mobile_only([...], style=s)"""
    },
    "window": {
        "title": "Window API",
        "content": """import fabiscos_window as window

Window control:
  window.set_title(title)      - Set window title
  window.set_content(html)     - Set UI content
  window.close()               - Close the window

Event handlers are Python functions called by name:
  def on_button_click():
      print("Clicked!")
      render()

  def on_input_change(value):
      print("New value:", value)

  def on_input_submit(value):
      print("Submitted:", value)

  def on_back():
      # Mobile back button pressed
      window.close()

Usage with widgets:
  ui.button("Save", on_click="on_button_click")
  ui.input("", on_change="on_input_change",
           on_submit="on_input_submit")

Re-rendering pattern:
  def render():
      window.set_content(build_ui())
      window.set_title("My App")

  # Call render() after state changes
  def on_increment():
      global count
      count += 1
      render()"""
    },
    "files": {
        "title": "Files API",
        "content": """VFS - Virtual Filesystem
  import fabiscos_vfs as vfs

Reading files:
  vfs.read_text(path)          - Read as string
  vfs.read_bytes(path)         - Read as bytes
  vfs.exists(path)             - Check if exists
  vfs.stat(path)               - Get file info dict

Writing files:
  vfs.write(path, content)     - Write text
  vfs.write_bytes(path, data)  - Write bytes
  vfs.append(path, content)    - Append text

Directories:
  vfs.list_dir(path)           - List contents
  vfs.mkdir(path)              - Create directory
  vfs.mkdir_p(path)            - Create with parents

File operations:
  vfs.remove(path)             - Delete file/dir
  vfs.copy(src, dst)           - Copy file
  vfs.move(src, dst)           - Move/rename file

CSV - Comma-Separated Values
  import fabiscos_csv as csv

Parsing:
  csv.parse(content)           - Parse to list of lists
  csv.parse_dict(content)      - Parse with headers

Writing:
  csv.stringify(rows)          - List of lists to CSV
  csv.stringify_with_headers(headers, rows)

Example:
  data = csv.parse_dict(vfs.read_text("/home/data.csv"))
  for row in data:
      print(row["name"], row["value"])

Archive - ZIP Files
  import fabiscos_archive as archive

Operations:
  archive.list_zip(path)       - List ZIP contents
  archive.read_from_zip(zip_path, file_name)
  archive.unzip(zip_path, dest_dir)
  archive.zip(files, zip_path)

Example:
  # Create backup
  archive.zip(["/home/Documents"], "/home/backup.zip")

  # Extract
  archive.unzip("/home/backup.zip", "/home/extracted")"""
    },
    "state": {
        "title": "State API",
        "content": """import fabiscos_state as state

State persists between app runs (per window).

String values:
  state.set(key, value)        - Store string
  state.get(key)               - Get string or None
  state.get(key, default)      - Get with default

List values:
  state.set_list(key, list)    - Store list of strings
  state.get_list(key)          - Get list or empty []

Clear state:
  state.clear()                - Remove all state

Counter example:
  count = int(state.get("count", "0"))

  def on_plus():
      global count
      count += 1
      state.set("count", str(count))
      render()

History example:
  history = state.get_list("history")

  def add_entry(text):
      history.append(text)
      state.set_list("history", history)
      render()

Note: All values are stored as strings.
Use int(), float(), json.loads() for conversion."""
    },
    "time": {
        "title": "Time API",
        "content": """import fabiscos_time as time

Current time:
  time.now()                   - Milliseconds since epoch
  time.monotonic()             - High-res monotonic time (ms)
  time.iso_now()               - Current time as ISO string

Create timestamp:
  time.timestamp(year, month, day, hour, min, sec)
    - month is 1-12
    - Returns milliseconds since epoch

Format timestamps:
  time.format_time(ms)         - "14:30:00" (localized)
  time.format_date(ms)         - "19.12.2024" (localized)
  time.format_iso(ms)          - ISO 8601 string

Example - Measure elapsed time:
  start = time.monotonic()
  # ... do something ...
  elapsed = time.monotonic() - start
  print(f"Took {elapsed:.2f}ms")

Example - Display current time:
  now = time.now()
  print(time.format_time(now))
  print(time.format_date(now))

Example - Create specific date:
  christmas = time.timestamp(2024, 12, 25, 0, 0, 0)
  print(time.format_date(christmas))"""
    },
    "random": {
        "title": "Random API",
        "content": """import fabiscos_random as random

Integers:
  random.randint(min, max)     - Random int [min, max] inclusive

Floats:
  random.random()              - Random float [0.0, 1.0)
  random.uniform(min, max)     - Random float [min, max)

Secure random:
  random.random_bytes(count)   - Cryptographic random bytes
  random.uuid4()               - Random UUID v4 string

Lists:
  random.choice(items)         - Random element from list
  random.shuffle(items)        - Shuffled copy of list

Examples:
  # Dice roll
  dice = random.randint(1, 6)

  # Coin flip
  result = random.choice(["heads", "tails"])

  # Shuffle cards
  cards = ["A", "K", "Q", "J"]
  shuffled = random.shuffle(cards)

  # Generate password
  chars = "abcdefghijklmnopqrstuvwxyz0123456789"
  password = "".join([random.choice(list(chars))
                      for _ in range(12)])

  # Unique ID
  id = random.uuid4()
  # "a1b2c3d4-e5f6-4789-8abc-def012345678" """
    },
    "encoding": {
        "title": "Encoding & Crypto",
        "content": """Base64
  import fabiscos_base64 as base64

Standard Base64:
  base64.encode(bytes)         - Bytes to base64 string
  base64.decode(string)        - Base64 to bytes
  base64.encode_str(text)      - Text to base64
  base64.decode_str(b64)       - Base64 to text

URL-safe Base64:
  base64.encode_url(bytes)     - No +, /, or = chars
  base64.decode_url(string)

Hash Functions
  import fabiscos_hash as hash

Available hashes (return hex):
  hash.md5_str(text)           - MD5 (not secure!)
  hash.sha1_str(text)          - SHA-1 (not secure!)
  hash.sha256_str(text)        - SHA-256 (recommended)
  hash.sha512_str(text)        - SHA-512

Bytes variants:
  hash.md5(bytes), hash.sha1(bytes), etc.

HMAC:
  hash.hmac_sha256_str(key, data)
  hash.hmac_sha256(key, data)

AES Encryption
  import fabiscos_crypto as crypto

Encrypt:
  crypto.encrypt_str(text, password)
  crypto.encrypt(bytes, password)
    - Returns base64 string (IV prepended)

Decrypt:
  crypto.decrypt_str(base64, password)
  crypto.decrypt(base64, password)
    - Raises error on wrong password

Example:
  # Encode data
  b64 = base64.encode_str("Hello!")

  # Hash password
  h = hash.sha256_str("secret")

  # Encrypt sensitive data
  enc = crypto.encrypt_str("API_KEY=xyz", "password")
  vfs.write("/home/secrets.enc", enc)

  # Decrypt
  try:
      data = crypto.decrypt_str(enc, "password")
  except:
      print("Wrong password!")

Security notes:
  - MD5/SHA1: checksums only, not security
  - Use SHA-256+ for security purposes
  - AES uses simple SHA-256 key derivation"""
    },
    "http": {
        "title": "HTTP API",
        "content": """import fabiscos_http as http

Simple requests:
  http.get(url)
    - GET request
    - Returns: {status, ok, text, error?}

  http.post(url, body, content_type=None)
    - POST with body
    - content_type: e.g. "application/json"

Generic request:
  http.fetch(url, method, body=None, headers_json=None)
    - Any HTTP method
    - headers_json: '{"Header": "value"}'

Response dict:
  {
    "status": 200,       # HTTP status code
    "ok": True,          # status 200-299
    "text": "...",       # Response body
    "error": "..."       # Error message if failed
  }

Examples:
  # GET request
  resp = http.get("https://api.example.com/data")
  if resp["ok"]:
      print(resp["text"])

  # POST JSON
  import json
  data = json.dumps({"name": "Test"})
  resp = http.post(
      "https://api.example.com/create",
      data,
      "application/json")

  # Custom headers
  resp = http.fetch(
      "https://api.example.com/auth",
      "GET",
      None,
      '{"Authorization": "Bearer token123"}')

Note: Subject to CORS restrictions.
Only works with APIs that allow browser requests."""
    },
    "notify": {
        "title": "Notify API",
        "content": """import fabiscos_notify as notify

  notify.notify(title, message=None)

Examples:
  # Simple notification
  notify.notify("Download complete!")

  # With message
  notify.notify(
      "Error",
      "Could not save file. Disk may be full.")

  # Success message
  notify.notify(
      "Saved",
      f"Document saved to {path}")"""
    },
    "example": {
        "title": "Complete Example",
        "content": """Full counter app with persistence:

  import fabiscos_ui as ui
  import fabiscos_window as window
  import fabiscos_state as state

  # Load saved count
  count = int(state.get("count") or "0")

  # Styles
  container = ui.style(
      padding="20px",
      display="flex",
      flex_direction="column",
      align_items="center",
      gap="16px"
  )
  count_style = ui.style(
      font_size="48px",
      font_weight="bold",
      color="#60a5fa"
  )
  btn_style = ui.style(
      padding="12px 24px",
      font_size="18px",
      border_radius="8px"
  )

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

  def on_reset():
      global count
      count = 0
      state.set("count", "0")
      render()

  def on_back():
      window.close()

  def render():
      window.set_content(
          ui.column([
              ui.label(str(count), style=count_style),
              ui.row([
                  ui.button("-", on_click="on_minus",
                            style=btn_style),
                  ui.button("Reset", on_click="on_reset",
                            style=btn_style),
                  ui.button("+", on_click="on_plus",
                            style=btn_style)
              ], style=ui.style(gap="8px"))
          ], style=container)
      )
      window.set_title(f"Counter: {count}")

  render()"""
    }
}

# Navigation structure with sections
nav_structure = [
    ("section", "Getting Started"),
    ("welcome", "Welcome"),
    ("filesystem", "Filesystem"),
    ("programming", "Creating Apps"),
    ("section", "Core APIs"),
    ("ui", "UI"),
    ("window", "Window"),
    ("files", "Files"),
    ("state", "State"),
    ("section", "Utility APIs"),
    ("time", "Time"),
    ("random", "Random"),
    ("encoding", "Encoding & Crypto"),
    ("http", "HTTP"),
    ("notify", "Notify"),
    ("section", "Examples"),
    ("example", "Full Example")
]

# Current state
current_page = state.get("page") or "welcome"
menu_open = state.get("menu_open") == "1"

def toggle_menu():
    global menu_open
    menu_open = not menu_open
    state.set("menu_open", "1" if menu_open else "0")
    render()

def nav_to(page):
    global current_page, menu_open
    current_page = page
    menu_open = False
    state.set("page", current_page)
    state.set("menu_open", "0")
    render()

# Navigation handlers
def nav_welcome(): nav_to("welcome")
def nav_filesystem(): nav_to("filesystem")
def nav_programming(): nav_to("programming")
def nav_ui(): nav_to("ui")
def nav_window(): nav_to("window")
def nav_files(): nav_to("files")
def nav_state(): nav_to("state")
def nav_time(): nav_to("time")
def nav_random(): nav_to("random")
def nav_encoding(): nav_to("encoding")
def nav_http(): nav_to("http")
def nav_notify(): nav_to("notify")
def nav_example(): nav_to("example")

nav_handlers = {
    "welcome": "nav_welcome",
    "filesystem": "nav_filesystem",
    "programming": "nav_programming",
    "ui": "nav_ui",
    "window": "nav_window",
    "files": "nav_files",
    "state": "nav_state",
    "time": "nav_time",
    "random": "nav_random",
    "encoding": "nav_encoding",
    "http": "nav_http",
    "notify": "nav_notify",
    "example": "nav_example"
}

def on_back():
    window.close()

def render():
    page = pages.get(current_page, pages["welcome"])

    # Build navigation
    nav_items = []
    for item in nav_structure:
        if item[0] == "section":
            nav_items.append(ui.label(item[1], style=section_style))
        else:
            key, label = item
            style = nav_btn_active_style if key == current_page else nav_btn_style
            handler = nav_handlers[key]
            nav_items.append(ui.button(label, style=style, on_click=handler))

    # Build content
    content_parts = [ui.label(page["title"], style=title_style)]

    lines = page["content"].split("\n")
    text_block = []
    in_code = False

    for line in lines:
        is_code_line = line.startswith("  ") and line.strip()

        if is_code_line and not in_code:
            if text_block:
                content_parts.append(ui.text("\n".join(text_block), style=para_style))
                text_block = []
            in_code = True
            text_block.append(line)
        elif not is_code_line and in_code and line.strip():
            content_parts.append(ui.text("\n".join(text_block), style=code_style))
            text_block = [line]
            in_code = False
        else:
            text_block.append(line)

    if text_block:
        style = code_style if in_code else para_style
        content_parts.append(ui.text("\n".join(text_block), style=style))

    page_title = next((label for key, label in nav_structure
                       if key == current_page), "Help")

    # Desktop layout
    desktop_html = ui.desktop_only([
        ui.container([
            ui.column(nav_items, style=sidebar_style),
            ui.container(content_parts, style=content_style)
        ], style=container_style)
    ], style=ui.style(height="100%"))

    # Mobile layout
    mobile_parts = []
    burger_icon = "fa-solid fa-xmark" if menu_open else "fa-solid fa-bars"
    mobile_parts.append(
        ui.row([
            ui.button("", icon=burger_icon, style=burger_btn_style, on_click="toggle_menu"),
            ui.label(page_title, style=mobile_title_style)
        ], style=mobile_header_style)
    )

    if menu_open:
        mobile_parts.append(ui.column(nav_items, style=mobile_sidebar_style))

    mobile_parts.append(ui.container(content_parts, style=content_style))

    mobile_html = ui.mobile_only([
        ui.column(mobile_parts, style=mobile_container_style)
    ], style=ui.style(height="100%"))

    html = ui.container([desktop_html, mobile_html], style=ui.style(height="100%"))

    window.set_content(html)
    window.set_title("Help")

render()
