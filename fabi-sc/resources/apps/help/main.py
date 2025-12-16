# FabiScOS Help
# Documentation and Programming Guide

import fabiscos_ui as ui
import fabiscos_window as window
import fabiscos_state as state

# Styles
container_style = ui.style(
    background="#1a1a2e",
    height="100%",
    display="flex",
    flex_direction="column",
    box_sizing="border-box"
)

sidebar_style = ui.style(
    background="#16213e",
    padding="12px",
    min_width="180px",
    border_right="1px solid #0f3460"
)

content_style = ui.style(
    flex="1",
    padding="20px",
    overflow="auto",
    color="#e0e0e0",
    line_height="1.6"
)

nav_btn_style = ui.style(
    background="transparent",
    border="none",
    color="#94a3b8",
    padding="10px 12px",
    text_align="left",
    width="100%",
    cursor="pointer",
    border_radius="6px",
    margin_bottom="4px"
)

nav_btn_active_style = ui.style(
    background="#0f3460",
    border="none",
    color="#60a5fa",
    padding="10px 12px",
    text_align="left",
    width="100%",
    cursor="pointer",
    border_radius="6px",
    margin_bottom="4px"
)

title_style = ui.style(
    font_size="24px",
    font_weight="bold",
    color="#60a5fa",
    margin_bottom="16px"
)

subtitle_style = ui.style(
    font_size="18px",
    font_weight="bold",
    color="#818cf8",
    margin_top="20px",
    margin_bottom="12px"
)

code_style = ui.style(
    background="#0d1117",
    padding="12px",
    border_radius="6px",
    font_family="'Cascadia Code', 'Fira Code', monospace",
    font_size="13px",
    color="#7dd3fc",
    overflow_x="auto",
    margin="12px 0"
)

para_style = ui.style(
    margin_bottom="12px",
    color="#cbd5e1"
)

# Content pages
pages = {
    "welcome": {
        "title": "Willkommen bei FabiScOS",
        "content": """
FabiScOS ist ein virtuelles Desktop-Betriebssystem das komplett im Browser lauft.

Es wurde mit Rust, WebAssembly und Python entwickelt.

Features:
- Virtuelles Dateisystem (VFS) mit Persistenz
- Python-Apps die im Browser laufen
- Desktop- und Mobile-Ansicht
- Fenster-Management (Drag, Resize, Minimize)

Navigation:
- Desktop: Klicke auf Apps in der Taskbar
- Mobile: Nutze Back, Home und Recents Buttons
"""
    },
    "filesystem": {
        "title": "Dateisystem (VFS)",
        "content": """
FabiScOS hat ein virtuelles Dateisystem das in IndexedDB gespeichert wird.

Ordnerstruktur:
/home/
  Desktop/     - Dateien auf dem Desktop
  Documents/   - Dokumente
  Pictures/    - Bilder
  apps/        - Deine eigenen Apps
  .system/     - System-Dateien (versteckt)
    apps/      - System-Apps (Terminal, etc.)

Terminal-Befehle:
- ls          - Ordnerinhalt anzeigen
- cd <path>   - Ordner wechseln
- pwd         - Aktueller Pfad
- cat <file>  - Datei anzeigen
- mkdir <dir> - Ordner erstellen
- touch <file>- Datei erstellen
- rm <file>   - Datei loschen
- cp <s> <d>  - Datei kopieren
- mv <s> <d>  - Datei verschieben
"""
    },
    "programming": {
        "title": "Apps Programmieren",
        "content": """
Apps werden in Python geschrieben und laufen im Browser.

App-Struktur:
/home/apps/meine_app/
  metadata.json  - App-Infos
  main.py        - Hauptcode

metadata.json Beispiel:
{
  "id": "meine_app",
  "name": "Meine App",
  "version": "1.0.0",
  "icon": "fa-solid fa-star",
  "entry": "main.py"
}

Minimales main.py:
import fabiscos_ui as ui
import fabiscos_window as window

window.set_content(
    ui.text("Hallo Welt!")
)
window.set_title("Meine App")
"""
    },
    "ui_api": {
        "title": "UI API",
        "content": """
import fabiscos_ui as ui

Styles erstellen:
style = ui.style(color="#fff", padding="10px")

Widgets:
ui.text(content, style=s)      - Mehrzeiliger Text
ui.label(content, style=s)     - Einzeiliger Text
ui.button(text, style=s)       - Button
ui.input(placeholder, style=s, on_submit="fn")
ui.image(src, alt="", style=s)
ui.divider(style=s)

Container:
ui.container([...], style=s)   - Generischer Container
ui.row([...], style=s)         - Horizontal (flexbox)
ui.column([...], style=s)      - Vertikal (flexbox)
ui.spacer()                    - Flexibler Abstand

Style-Properties (snake_case):
background, color, padding, margin
font_size, font_family, font_weight
border, border_radius
display, flex, flex_direction
width, height, min_width, max_width
"""
    },
    "window_api": {
        "title": "Window API",
        "content": """
import fabiscos_window as window

Fenster-Kontrolle:
window.set_title(title)     - Titel setzen
window.set_content(html)    - UI setzen
window.close()              - Fenster schliessen

Event-Handler definieren:
def on_input(value):
    # Wird aufgerufen bei Enter im Input
    print("Eingabe:", value)
    render()

def on_back():
    # Mobile Back-Button
    window.close()

Input mit Handler:
ui.input("", on_submit="on_input")
"""
    },
    "vfs_api": {
        "title": "VFS API",
        "content": """
import fabiscos_vfs as vfs

Dateien lesen/schreiben:
vfs.read_text(path)         - Text lesen
vfs.write(path, content)    - Text schreiben
vfs.exists(path)            - Existiert?

Ordner:
vfs.list_dir(path)          - Inhalt auflisten
vfs.mkdir(path)             - Ordner erstellen

Operationen:
vfs.remove(path)            - Loschen
vfs.copy(src, dst)          - Kopieren
vfs.move(src, dst)          - Verschieben

Arbeitsverzeichnis:
vfs.cwd()                   - Aktueller Pfad
vfs.set_cwd(path)           - Pfad setzen
"""
    },
    "state_api": {
        "title": "State API",
        "content": """
import fabiscos_state as state

State bleibt zwischen App-Ausfuhrungen erhalten.

Werte speichern:
state.set(key, value)       - String speichern
state.get(key)              - String lesen

Listen:
state.set_list(key, list)   - Liste speichern
state.get_list(key)         - Liste lesen

Beispiel (Counter):
count = state.get("count") or "0"
count = int(count) + 1
state.set("count", str(count))

Beispiel (Terminal History):
lines = state.get_list("output") or []
lines.append("Neue Zeile")
state.set_list("output", lines)
"""
    },
    "example": {
        "title": "Beispiel-App",
        "content": """
Komplette Counter-App:

import fabiscos_ui as ui
import fabiscos_window as window
import fabiscos_state as state

# State laden
count = int(state.get("count") or "0")

def on_input(value):
    global count
    if value == "+":
        count += 1
    elif value == "-":
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
                ui.input("+ oder -",
                    on_submit="on_input")
            ])
        ], style=ui.style(padding="20px"))
    )
    window.set_title("Counter")

render()
"""
    }
}

# Current page
current_page = state.get("page") or "welcome"

def on_input(value):
    global current_page
    if value in pages:
        current_page = value
        state.set("page", value)
        render()

def on_back():
    window.close()

def render():
    page = pages.get(current_page, pages["welcome"])

    # Build navigation
    nav_items = []
    nav_labels = {
        "welcome": "Willkommen",
        "filesystem": "Dateisystem",
        "programming": "Programmieren",
        "ui_api": "UI API",
        "window_api": "Window API",
        "vfs_api": "VFS API",
        "state_api": "State API",
        "example": "Beispiel-App"
    }

    for key, label in nav_labels.items():
        style = nav_btn_active_style if key == current_page else nav_btn_style
        # Use hidden input for navigation
        nav_items.append(ui.button(label, style=style))

    # Build content
    content_parts = [ui.label(page["title"], style=title_style)]

    # Split content into paragraphs and code blocks
    lines = page["content"].strip().split("\n")
    current_text = []
    in_code = False

    for line in lines:
        if not in_code and (line.startswith("  ") or line.startswith("{")):
            # Flush text
            if current_text:
                content_parts.append(ui.text("\n".join(current_text), style=para_style))
                current_text = []
            in_code = True
            current_text.append(line)
        elif in_code and not line.startswith("  ") and not line.startswith("}") and line.strip() and not line.startswith("{"):
            # End code block
            content_parts.append(ui.text("\n".join(current_text), style=code_style))
            current_text = [line]
            in_code = False
        else:
            current_text.append(line)

    # Flush remaining
    if current_text:
        style = code_style if in_code else para_style
        content_parts.append(ui.text("\n".join(current_text), style=style))

    # Hidden input for navigation
    nav_input = ui.input("Seite...", style=ui.style(
        position="absolute",
        top="-100px"
    ), on_submit="on_input")

    html = ui.row([
        ui.column(nav_items, style=sidebar_style),
        ui.column(content_parts, style=content_style),
        nav_input
    ], style=container_style)

    window.set_content(html)
    window.set_title("Help - " + page["title"])

render()
