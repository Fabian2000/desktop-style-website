# FabiScOS Calculator
# Simple calculator app

import fabiscos_ui as ui
import fabiscos_window as window
import fabiscos_state as state

# Styles
container_style = ui.style(
    background="#1e1e2e",
    height="100%",
    display="flex",
    flex_direction="column",
    padding="12px",
    box_sizing="border-box",
    gap="12px"
)

display_style = ui.style(
    background="#11111b",
    border="1px solid #313244",
    border_radius="8px",
    padding="16px",
    text_align="right",
    font_family="'Cascadia Code', 'Fira Code', monospace",
    font_size="28px",
    color="#cdd6f4",
    min_height="60px",
    display="flex",
    align_items="center",
    justify_content="flex-end",
    overflow="hidden"
)

btn_row_style = ui.style(
    display="flex",
    flex_direction="row",
    gap="8px",
    flex="1"
)

btn_style = ui.style(
    flex="1",
    background="#313244",
    border="none",
    border_radius="8px",
    color="#cdd6f4",
    font_size="20px",
    cursor="pointer",
    display="flex",
    align_items="center",
    justify_content="center"
)

btn_op_style = ui.style(
    flex="1",
    background="#89b4fa",
    border="none",
    border_radius="8px",
    color="#1e1e2e",
    font_size="20px",
    font_weight="bold",
    cursor="pointer",
    display="flex",
    align_items="center",
    justify_content="center"
)

btn_eq_style = ui.style(
    flex="1",
    background="#a6e3a1",
    border="none",
    border_radius="8px",
    color="#1e1e2e",
    font_size="20px",
    font_weight="bold",
    cursor="pointer",
    display="flex",
    align_items="center",
    justify_content="center"
)

btn_clear_style = ui.style(
    flex="1",
    background="#f38ba8",
    border="none",
    border_radius="8px",
    color="#1e1e2e",
    font_size="20px",
    font_weight="bold",
    cursor="pointer",
    display="flex",
    align_items="center",
    justify_content="center"
)

# State
_saved_display = state.get("display")
display_value = _saved_display if _saved_display else "0"
_saved_expr = state.get("expression")
expression = _saved_expr if _saved_expr else ""
_saved_new = state.get("new_number")
new_number = _saved_new == "true" if _saved_new else True

def save_state():
    state.set("display", display_value)
    state.set("expression", expression)
    state.set("new_number", "true" if new_number else "false")

def press_digit(d):
    global display_value, expression, new_number
    if new_number:
        display_value = d
        new_number = False
    else:
        if display_value == "0":
            display_value = d
        else:
            display_value += d
    expression += d
    save_state()
    render()

def press_op(op):
    global display_value, expression, new_number
    if expression and expression[-1] in "+-*/":
        expression = expression[:-1] + op
    else:
        expression += op
    new_number = True
    save_state()
    render()

def press_dot():
    global display_value, expression, new_number
    if new_number:
        display_value = "0."
        expression += "0."
        new_number = False
    elif "." not in display_value:
        display_value += "."
        expression += "."
    save_state()
    render()

def press_clear():
    global display_value, expression, new_number
    display_value = "0"
    expression = ""
    new_number = True
    save_state()
    render()

def press_backspace():
    global display_value, expression, new_number
    if expression:
        expression = expression[:-1]
        if display_value and len(display_value) > 1:
            display_value = display_value[:-1]
        else:
            display_value = "0"
            new_number = True
    save_state()
    render()

def press_equals():
    global display_value, expression, new_number
    if expression:
        try:
            # Clean expression for eval
            clean_expr = expression.replace("x", "*")
            result = eval(clean_expr)
            # Format result
            if isinstance(result, float):
                if result == int(result):
                    display_value = str(int(result))
                else:
                    display_value = str(round(result, 10))
            else:
                display_value = str(result)
            expression = display_value
            new_number = True
        except:
            display_value = "Error"
            expression = ""
            new_number = True
    save_state()
    render()

# Button handlers
def btn_0(): press_digit("0")
def btn_1(): press_digit("1")
def btn_2(): press_digit("2")
def btn_3(): press_digit("3")
def btn_4(): press_digit("4")
def btn_5(): press_digit("5")
def btn_6(): press_digit("6")
def btn_7(): press_digit("7")
def btn_8(): press_digit("8")
def btn_9(): press_digit("9")
def btn_dot(): press_dot()
def btn_add(): press_op("+")
def btn_sub(): press_op("-")
def btn_mul(): press_op("*")
def btn_div(): press_op("/")
def btn_eq(): press_equals()
def btn_c(): press_clear()
def btn_back(): press_backspace()

def render():
    # Display
    disp = ui.container([
        ui.label(display_value, style=ui.style(color="#cdd6f4", font_size="28px"))
    ], style=display_style)

    # Button rows
    row1 = ui.row([
        ui.button("C", style=btn_clear_style, on_click="btn_c"),
        ui.button("DEL", style=btn_style, on_click="btn_back"),
        ui.button("/", style=btn_op_style, on_click="btn_div"),
    ], style=btn_row_style)

    row2 = ui.row([
        ui.button("7", style=btn_style, on_click="btn_7"),
        ui.button("8", style=btn_style, on_click="btn_8"),
        ui.button("9", style=btn_style, on_click="btn_9"),
        ui.button("*", style=btn_op_style, on_click="btn_mul"),
    ], style=btn_row_style)

    row3 = ui.row([
        ui.button("4", style=btn_style, on_click="btn_4"),
        ui.button("5", style=btn_style, on_click="btn_5"),
        ui.button("6", style=btn_style, on_click="btn_6"),
        ui.button("-", style=btn_op_style, on_click="btn_sub"),
    ], style=btn_row_style)

    row4 = ui.row([
        ui.button("1", style=btn_style, on_click="btn_1"),
        ui.button("2", style=btn_style, on_click="btn_2"),
        ui.button("3", style=btn_style, on_click="btn_3"),
        ui.button("+", style=btn_op_style, on_click="btn_add"),
    ], style=btn_row_style)

    row5 = ui.row([
        ui.button("0", style=btn_style, on_click="btn_0"),
        ui.button(".", style=btn_style, on_click="btn_dot"),
        ui.button("=", style=btn_eq_style, on_click="btn_eq"),
    ], style=btn_row_style)

    html = ui.container([disp, row1, row2, row3, row4, row5], style=container_style)
    window.set_content(html)
    window.set_title("Calculator")

# Initial render
render()
