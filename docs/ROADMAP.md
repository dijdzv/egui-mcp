# egui-mcp Roadmap

## Current Status (Phase 6 Complete)

All Phase 6 enhanced interactions have been implemented.

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
| `double_click` | Double click at coordinates | IPC |
| `keyboard_input` | Send keyboard input | IPC |
| `scroll` | Scroll at coordinates | IPC |
| `hover` | Move mouse to coordinates | IPC |
| `drag` | Drag between coordinates | IPC |
| `drag_element` | Drag element to target | AT-SPI + IPC |
| `take_screenshot` | Capture screenshot | IPC |
| `ping` | Server health check | - |
| `check_connection` | App connection check | IPC |
| `get_bounds` | Get element bounding box | AT-SPI Component |
| `focus_element` | Focus element by ID | AT-SPI Component |
| `scroll_to_element` | Scroll element into view | AT-SPI Component |
| `get_value` | Get slider/progress value | AT-SPI Value |
| `set_value` | Set slider value | AT-SPI Value |
| `select_item` | Select item by index | AT-SPI Selection |
| `deselect_item` | Deselect item by index | AT-SPI Selection |
| `get_selected_count` | Get selected items count | AT-SPI Selection |
| `select_all` | Select all items | AT-SPI Selection |
| `clear_selection` | Clear all selections | AT-SPI Selection |
| `get_text` | Get text content | AT-SPI Text |
| `get_text_selection` | Get selected text range | AT-SPI Text |
| `set_text_selection` | Set text selection | AT-SPI Text |
| `get_caret_position` | Get cursor position | AT-SPI Text |
| `set_caret_position` | Set cursor position | AT-SPI Text |

---

## Phase 6: Enhanced Interactions (Complete)

Features inspired by Playwright and Chrome DevTools MCP.

### Priority 1: Mouse Operations ✅

| Tool | Description | Method | Reference | Status |
|------|-------------|--------|-----------|--------|
| `hover` | Move mouse to element/coordinates | IPC (move_mouse) | Playwright hover() | ✅ Done |
| `double_click` | Double click at coordinates | IPC | Playwright dblclick() | ✅ Done |
| `drag` | Drag from point A to point B | IPC (drag) | Playwright dragTo() | ✅ Done |
| `drag_element` | Drag element to target | AT-SPI + IPC | Playwright dragTo() | ✅ Done |

### Priority 2: Element Information (AT-SPI Component) ✅

| Tool | Description | Method | Reference | Status |
|------|-------------|--------|-----------|--------|
| `get_bounds` | Get element bounding box | AT-SPI Component | Playwright boundingBox() | ✅ Done |
| `focus_element` | Focus element by ID | AT-SPI Component | Playwright focus() | ✅ Done |
| `scroll_to_element` | Scroll element into view | AT-SPI Component | Playwright scrollIntoViewIfNeeded() | ✅ Done |

### Priority 3: Value Operations (AT-SPI Value) ✅

For sliders, progress bars, spinboxes, etc.

| Tool | Description | Method | Reference | Status |
|------|-------------|--------|-----------|--------|
| `get_value` | Get current value (includes min/max/increment) | AT-SPI Value | - | ✅ Done |
| `set_value` | Set value (slider, etc.) | AT-SPI Value | Playwright fill() for inputs | ✅ Done |

### Priority 4: Selection Operations (AT-SPI Selection) ✅

For lists, combo boxes, menus, etc.

| Tool | Description | Method | Reference | Status |
|------|-------------|--------|-----------|--------|
| `select_item` | Select item by index | AT-SPI Selection | Playwright selectOption() | ✅ Done |
| `deselect_item` | Deselect item by index | AT-SPI Selection | - | ✅ Done |
| `get_selected_count` | Get count of selected items | AT-SPI Selection | - | ✅ Done |
| `select_all` | Select all items | AT-SPI Selection | - | ✅ Done |
| `clear_selection` | Clear all selections | AT-SPI Selection | - | ✅ Done |

### Priority 5: Text Operations (AT-SPI Text) ✅

Enhanced text handling beyond EditableText.

| Tool | Description | Method | Reference | Status |
|------|-------------|--------|-----------|--------|
| `get_text` | Get text content (includes length, caret) | AT-SPI Text | Playwright textContent() | ✅ Done |
| `get_text_selection` | Get selected text range | AT-SPI Text | - | ✅ Done |
| `set_text_selection` | Select text range | AT-SPI Text | Playwright selectText() | ✅ Done |
| `get_caret_position` | Get cursor position | AT-SPI Text | - | ✅ Done |
| `set_caret_position` | Set cursor position | AT-SPI Text | - | ✅ Done |

---

## Known Limitations

### Working Interfaces

The following AT-SPI interfaces are now working:

| Interface | Tools | Status | Notes |
|-----------|-------|--------|-------|
| Action | `click_element` | ✅ Working | |
| Component | `get_bounds`, `focus_element`, `scroll_to_element`, `drag_element` | ✅ Working | |
| Text (read) | `get_text`, `get_caret_position` | ✅ Working | Read-only operations |
| Text (selection) | `get_text_selection`, `set_text_selection` | ✅ Working | atspi-proxies workaround (see below) |
| Value | `get_value`, `set_value` | ✅ Working | Works in egui 0.33+ |
| Selection (partial) | `get_selected_count` | ✅ Working | ComboBox uses name property |
| Text (caret) | `set_caret_position` | ✅ Working | Requires focus first |

### Not Working (Limitation)

| Interface | Tools Affected | Issue | Workaround |
|-----------|---------------|-------|------------|
| EditableText | `set_text` | AccessKit doesn't implement EditableText interface | IPC `keyboard_input` |
| Selection | `select_item`, `deselect_item` | egui ComboBox items not registered as children | IPC `click_at` + `keyboard_input` |

### Not Needed

| Tools | Reason |
|-------|--------|
| `select_all`, `clear_selection` | egui only has ComboBox and RadioGroup (single selection) |

> **Note**: See [egui-accessibility-investigation.md](egui-accessibility-investigation.md) for detailed analysis of each limitation.

### Workarounds and Future Fixes

1. **Text Selection**: Currently uses D-Bus method call workaround for atspi-proxies 0.9.0 bug (method name case mismatch). This workaround will become unnecessary when [egui PR #7850](https://github.com/emilk/egui/pull/7850) merges (updates atspi to 0.28.0).

2. **EditableText**: AT-SPI `Action.DoAction(index)` cannot pass arguments, so AccessKit's `ReplaceSelectedText` cannot be invoked via AT-SPI. Use IPC-based `keyboard_input` as the working alternative.

3. **Selection (ComboBox)**: egui's ComboBox popup architecture puts items in a separate window, not as children of the ComboBox. `get_selected_count` works by checking the ComboBox's name property. For selecting items, use IPC-based `click_at` + `keyboard_input`.

### Related Code (AccessKit)

From `accesskit_atspi_common/src/node.rs`:
```rust
fn supports_component(&self) -> bool {
    self.0.raw_bounds().is_some() || self.is_root()
}
fn supports_value(&self) -> bool {
    self.current_value().is_some()  // calls numeric_value()
}
fn supports_text(&self) -> bool {
    self.0.supports_text_ranges()
}
fn supports_selection(&self) -> bool {
    self.0.is_container_with_selectable_children()
}
```

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

### IPC Methods (All Exposed)

From `ipc_client.rs` - all methods now exposed as MCP tools:

- `move_mouse(x, y)` - exposed as `hover`
- `drag(start_x, start_y, end_x, end_y, button)` - exposed as `drag`
- `double_click(x, y, button)` - exposed as `double_click`

---

## References

- [Playwright Actions](https://playwright.dev/docs/input)
- [Chrome DevTools MCP](https://github.com/AhmedBasem20/chrome-devtools-mcp)
- [AT-SPI Documentation](https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/)
- [atspi-proxies Rust crate](https://docs.rs/atspi-proxies)
