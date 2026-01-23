# egui-mcp Roadmap

## Current Status (Phase 5 Complete)

All initially planned features have been implemented.

### Implemented Tools

| Tool | Description | Method |
|------|-------------|--------|
| `get_ui_tree` | Get the complete UI tree | AT-SPI |
| `find_by_label` | Search elements by label (substring) | AT-SPI |
| `find_by_label_exact` | Search elements by label (exact) | AT-SPI |
| `find_by_role` | Search elements by role | AT-SPI |
| `get_element` | Get element by ID | AT-SPI |
| `click_element` | Click element by ID | AT-SPI Action |
| `set_text` | Set text input content | AT-SPI EditableText |
| `click_at` | Click at coordinates | IPC |
| `keyboard_input` | Send keyboard input | IPC |
| `scroll` | Scroll at coordinates | IPC |
| `take_screenshot` | Capture screenshot | IPC |
| `ping` | Server health check | - |
| `check_connection` | App connection check | IPC |

---

## Phase 6: Enhanced Interactions (In Progress)

Features inspired by Playwright and Chrome DevTools MCP.

### Priority 1: Mouse Operations ✅

| Tool | Description | Method | Reference | Status |
|------|-------------|--------|-----------|--------|
| `hover` | Move mouse to element/coordinates | IPC (move_mouse) | Playwright hover() | ✅ Done |
| `double_click` | Double click at coordinates | IPC | Playwright dblclick() | ✅ Done |
| `drag` | Drag from point A to point B | IPC (drag) | Playwright dragTo() | ✅ Done |
| `drag_element` | Drag element to target | AT-SPI + IPC | Playwright dragTo() | ⏳ Planned |

### Priority 2: Element Information (AT-SPI Component)

| Tool | Description | Method | Reference |
|------|-------------|--------|-----------|
| `get_bounds` | Get element bounding box | AT-SPI Component | Playwright boundingBox() |
| `focus_element` | Focus element by ID | AT-SPI Component | Playwright focus() |
| `scroll_to_element` | Scroll element into view | AT-SPI Component | Playwright scrollIntoViewIfNeeded() |
| `element_at_point` | Find element at coordinates | AT-SPI Component | - |

### Priority 3: Value Operations (AT-SPI Value)

For sliders, progress bars, spinboxes, etc.

| Tool | Description | Method | Reference |
|------|-------------|--------|-----------|
| `get_value` | Get current value | AT-SPI Value | - |
| `set_value` | Set value (slider, etc.) | AT-SPI Value | Playwright fill() for inputs |
| `get_value_range` | Get min/max/increment | AT-SPI Value | - |

### Priority 4: Selection Operations (AT-SPI Selection)

For lists, combo boxes, menus, etc.

| Tool | Description | Method | Reference |
|------|-------------|--------|-----------|
| `select_item` | Select item by index | AT-SPI Selection | Playwright selectOption() |
| `deselect_item` | Deselect item by index | AT-SPI Selection | - |
| `get_selected_items` | Get selected items | AT-SPI Selection | - |
| `select_all` | Select all items | AT-SPI Selection | - |
| `clear_selection` | Clear all selections | AT-SPI Selection | - |

### Priority 5: Text Operations (AT-SPI Text)

Enhanced text handling beyond EditableText.

| Tool | Description | Method | Reference |
|------|-------------|--------|-----------|
| `get_text` | Get text content | AT-SPI Text | Playwright textContent() |
| `get_text_selection` | Get selected text range | AT-SPI Text | - |
| `set_text_selection` | Select text range | AT-SPI Text | Playwright selectText() |
| `get_caret_position` | Get cursor position | AT-SPI Text | - |
| `set_caret_position` | Set cursor position | AT-SPI Text | - |

---

## Phase 7: Advanced Features (Future)

### Wait/Polling Operations

| Tool | Description | Method |
|------|-------------|--------|
| `wait_for_element` | Wait for element to appear/disappear | Polling AT-SPI |
| `wait_for_state` | Wait for element state change | Polling AT-SPI |

### State Queries

| Tool | Description | Method |
|------|-------------|--------|
| `is_visible` | Check if element is visible | AT-SPI State |
| `is_enabled` | Check if element is enabled | AT-SPI State |
| `is_focused` | Check if element has focus | AT-SPI State |
| `is_checked` | Check toggle/checkbox state | AT-SPI State |

### Screenshot Enhancements

| Tool | Description | Method |
|------|-------------|--------|
| `screenshot_element` | Screenshot specific element | IPC + bounds |
| `screenshot_region` | Screenshot specific region | IPC |

---

## Implementation Notes

### AT-SPI Interfaces Available

From `atspi-proxies` crate:

- **Accessible** - Base interface (name, role, state, children)
- **Action** - Click, activate (implemented)
- **Component** - Position, size, focus, scroll
- **EditableText** - Set text content (implemented)
- **Text** - Read text, selections, caret
- **Value** - Numeric values (sliders, etc.)
- **Selection** - List/combo selection
- **Table** / **TableCell** - Table navigation
- **Image** - Image description
- **Document** - Document properties
- **Hyperlink** / **Hypertext** - Link handling

### IPC Methods Available (Not Yet Exposed)

From `ipc_client.rs`:

- `move_mouse(x, y)` - Already implemented
- `drag(start_x, start_y, end_x, end_y, button)` - Already implemented

### Protocol Extensions Needed

For new IPC features:

```rust
// egui-mcp-protocol Request enum additions
DoubleClick { x: f32, y: f32, button: MouseButton },
```

---

## References

- [Playwright Actions](https://playwright.dev/docs/input)
- [Chrome DevTools MCP](https://github.com/AhmedBasem20/chrome-devtools-mcp)
- [AT-SPI Documentation](https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/)
- [atspi-proxies Rust crate](https://docs.rs/atspi-proxies)
