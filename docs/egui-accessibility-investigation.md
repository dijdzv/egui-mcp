# egui Accessibility Investigation

This document describes AT-SPI accessibility behavior in egui and documents our investigation findings.

## Background

egui uses [AccessKit](https://github.com/AccessKit/accesskit) to provide cross-platform accessibility support. AccessKit translates accessibility information to platform-specific APIs:
- Windows: UI Automation
- macOS: NSAccessibility
- Linux: AT-SPI2 via D-Bus

---

## Current Status

| Interface | Tools | Status | Notes |
|-----------|-------|--------|-------|
| Action | `click_element` | ✅ Working | - |
| Component | `get_bounds`, `focus_element`, `scroll_to_element` | ✅ Working | - |
| State | `is_visible`, `is_enabled`, `is_focused`, `is_checked` | ✅ Working | - |
| Text (read) | `get_text`, `get_caret_position` | ✅ Working | - |
| Text (selection) | `get_text_selection`, `set_text_selection` | ✅ Working | See [atspi-proxies-issue.md](atspi-proxies-issue.md) |
| Text (write) | `set_caret_position` | ✅ Working | Requires focus first |
| Value | `get_value`, `set_value` | ✅ Working | Works in egui 0.33+ |
| Selection (read) | `get_selected_count` | ✅ Working | ComboBox uses name property |
| Selection (write) | `select_item`, `deselect_item` | ⛔ egui architecture | IPC `click_at` + `keyboard_input` |
| Selection (bulk) | `select_all`, `clear_selection` | ➖ Not needed | egui only has single selection |
| EditableText | `set_text` | ⛔ AccessKit limitation | IPC `keyboard_input` |

---

## State Interface - WORKING

**Status**: ✅ Works out of the box

The State interface provides boolean flags about element state. We query these via `AccessibleProxy::get_state()` which returns a `StateSet`.

### Implemented Tools

| Tool | State Flags Checked | Notes |
|------|---------------------|-------|
| `is_visible` | `Visible`, `Showing` | Returns true if either flag is set |
| `is_enabled` | `Enabled` | Disabled elements have this flag unset |
| `is_focused` | `Focused` | Requires element to have keyboard focus |
| `is_checked` | `Checked`, `Pressed`, `Checkable` | Returns `Some(true)` if checked, `Some(false)` if checkable but unchecked, `None` if not checkable |

### How State is Set in egui

From `accesskit_atspi_common/src/node.rs`, states are derived from AccessKit node properties:

```rust
// State flags from AccessKit
state.insert(State::Visible);  // if node has bounds
state.insert(State::Showing);  // if visible and not clipped
state.insert(State::Enabled);  // if not disabled
state.insert(State::Focused);  // if focused
state.insert(State::Checked);  // if toggled == Some(true)
state.insert(State::Checkable); // if toggled.is_some()
```

---

## Value Interface (Slider, DragValue) - WORKING

**Status**: ✅ Works in egui 0.33+

**Investigation**: Initially we thought a fork fix was needed, but investigation revealed that egui 0.33.0 already correctly implements the Value interface:

1. `WidgetInfo::slider()` sets `value: Some(value)`
2. `fill_accesskit_node_from_widget_info()` calls `builder.set_numeric_value(value)`

The Value interface works out of the box with egui 0.33+.

---

## EditableText Interface - NOT FIXABLE IN EGUI

**Symptom**: `set_text` fails with "Unknown interface 'org.a11y.atspi.EditableText'"

**Root Cause**: **AccessKit itself does not implement the AT-SPI EditableText interface**.

### Investigation Results

1. **AT-SPI EditableText Interface** (`atspi-proxies/editable_text.rs`):
   - `SetTextContents(text)` - Replace entire text
   - `InsertText(position, text, length)` - Insert text at position
   - `DeleteText(start, end)` - Delete text range
   - `CopyText`, `CutText`, `PasteText` - Clipboard operations

2. **AccessKit Support**:
   - `Action::ReplaceSelectedText` exists for text replacement
   - **However**, `accesskit_atspi_common/src/node.rs` `interfaces()` method does NOT include `Interface::EditableText`
   - The EditableText interface is never exposed via AT-SPI

3. **Why AT-SPI Cannot Use AccessKit's Approach**:
   - AT-SPI's `Action.DoAction(index)` method takes **no arguments** - it only accepts an action index
   - AccessKit's `Action::ReplaceSelectedText` requires **data** (the replacement text)
   - There is no way to pass the replacement text through AT-SPI's Action interface
   - Therefore, even though AccessKit supports `ReplaceSelectedText`, it cannot be invoked via AT-SPI

4. **Conclusion**:
   - Fixing egui alone cannot enable EditableText
   - AccessKit's AT-SPI adapter (`accesskit_atspi_common`) needs modification to implement `EditableText` interface
   - This is **out of scope** for egui fixes

**Workaround**: Use IPC-based keyboard simulation (`click_at` + `keyboard_input`) - already implemented in egui-mcp-server

---

## Selection Interface (ComboBox) - NOT FIXABLE IN EGUI (Architecture)

**Symptom**: Selection methods fail because ComboBox has no child items visible to AT-SPI.

### Investigation Results

1. **Selection interface IS supported** by AccessKit
2. **ComboBox DOES have Selection interface** (`['Accessible', 'Action', 'Component', 'Selection']`)
3. **Problem**: ComboBox has 0 children in AT-SPI tree
4. **`get_n_selected_children()` works** (returns 0)

The real issue is egui's ComboBox popup architecture:
- ComboBox items are in a separate popup window
- Popup appears only when ComboBox is opened
- Items are NOT registered as ComboBox children in AccessKit
- egui's immediate mode GUI makes parent-child relationships difficult

```rust
// accesskit_atspi_common/src/node.rs
pub fn select_child(&self, child_index: usize) -> Result<bool> {
    // This requires filtered_children(filter).nth(child_index)
    // But ComboBox has no children!
}
```

### get_selected_count Workaround

For ComboBox, we check the `name` property instead of using the Selection interface:

```rust
// ComboBox: check if there's a selected value (stored in name property)
if role == atspi_common::Role::ComboBox {
    let name: String = accessible_proxy.name().await.unwrap_or_default();
    // If name is not empty, something is selected
    return Ok(if name.is_empty() { 0 } else { 1 });
}
```

**Status**: ❌ Cannot be easily fixed (requires architectural changes to egui)
**Workaround**: Use IPC-based `click_at` + `keyboard_input` to interact with ComboBox

---

## Summary

| Issue | Status | Notes |
|-------|--------|-------|
| State Interface | ✅ Working | Visible, Enabled, Focused, Checked |
| Value Interface | ✅ Working | egui 0.33+ |
| EditableText | ⛔ AccessKit limitation | Use IPC workaround |
| Text Selection | ✅ Fixed | See [atspi-proxies-issue.md](atspi-proxies-issue.md) |
| Selection (read) | ✅ Fixed | ComboBox uses name property |
| Selection (write) | ⛔ egui architecture | Use IPC workaround |

---

## Testing

```bash
# Run demo app
just demo

# Test AT-SPI tools via MCP
# Or use Python:
python3 -c "
import gi
gi.require_version('Atspi', '2.0')
from gi.repository import Atspi

desktop = Atspi.get_desktop(0)
# Navigate to element and test
"

# Or use Accerciser
sudo apt install accerciser
accerciser
```

---

## Related Links

- [egui PR #2294 - Implement accessibility APIs via AccessKit](https://github.com/emilk/egui/pull/2294)
- [egui PR #7850 - Update atspi to 0.28.0](https://github.com/emilk/egui/pull/7850) - Fixes atspi-proxies method name bug
- [egui Issue #167 - Accessibility (A11y)](https://github.com/emilk/egui/issues/167)
- [AccessKit Repository](https://github.com/AccessKit/accesskit)
- [AT-SPI Documentation](https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/)
- [atspi-proxies issue](atspi-proxies-issue.md) - D-Bus proxy and method name bugs
