# egui Accessibility Improvements for AT-SPI

This document describes missing AccessKit properties in egui that prevent AT-SPI interfaces from working correctly on Linux.

## Background

egui uses [AccessKit](https://github.com/AccessKit/accesskit) to provide cross-platform accessibility support. AccessKit translates accessibility information to platform-specific APIs:
- Windows: UI Automation
- macOS: NSAccessibility
- Linux: AT-SPI2 via D-Bus

While egui's AccessKit integration works well for basic operations (click, set_text), several AT-SPI interfaces are not functional because egui doesn't provide the required data to AccessKit.

## Issues Found

### 1. Value Interface (Slider, DragValue)

**Symptom**: AT-SPI Value interface methods (`get_value`, `set_value`) fail with "Unknown property 'CurrentValue'"

**Root Cause**: `set_numeric_value()` is never called on the AccessKit builder.

AccessKit's AT-SPI adapter checks `supports_value()` which requires `numeric_value` to be set:

```rust
// accesskit_atspi_common/src/node.rs
fn supports_value(&self) -> bool {
    self.current_value().is_some()  // calls numeric_value()
}
```

**Current Code** (`crates/egui/src/widgets/slider.rs`):
```rust
builder.set_min_numeric_value(*self.range.start());
builder.set_max_numeric_value(*self.range.end());
// Missing: builder.set_numeric_value(current_value);
```

**Fix Required**:

In `slider.rs`, add after the min/max settings:
```rust
builder.set_numeric_value(get());
```

In `drag_value.rs`, add:
```rust
builder.set_numeric_value(value);
```

---

### 2. Component Interface (Bounds)

**Symptom**: AT-SPI Component interface methods (`get_bounds`, `focus_element`, `scroll_to_element`) fail with "Unknown method 'GetExtents'"

**Root Cause**: `fill_accesskit_node_common` calls `set_bounds()` correctly. However, the issue may be:
1. Bounds are set in local coordinates but AT-SPI expects screen/window coordinates
2. The transform is not being applied correctly
3. Timing issue - bounds set after node is committed

AccessKit's AT-SPI adapter checks:
```rust
// accesskit_atspi_common/src/node.rs
fn supports_component(&self) -> bool {
    self.0.raw_bounds().is_some() || self.is_root()
}

// raw_bounds() simply returns the stored bounds:
pub fn raw_bounds(&self) -> Option<Rect> {
    self.data().bounds()
}
```

**Current Code** (`crates/egui/src/response.rs`):
```rust
builder.set_bounds(accesskit::Rect {
    x0: self.rect.min.x.into(),
    y0: self.rect.min.y.into(),
    x1: self.rect.max.x.into(),
    y1: self.rect.max.y.into(),
});
```

**Potential Issues**:

1. **Transform not set**: AccessKit may need `set_transform()` to convert local bounds to screen coordinates
2. **Bounds in wrong coordinate space**: The rect might need to be in screen coordinates, not widget-local

**Investigation Needed**:
- Debug whether `bounds()` returns `Some` or `None` in the AccessKit tree
- Check if `set_transform()` is being called on parent nodes
- Verify coordinate system expectations

---

### 3. Selection Interface (ComboBox)

**Symptom**: AT-SPI Selection interface methods (`select_item`, `get_selected_count`, etc.) fail with "Unknown property 'NselectedChildren'"

**Root Cause**: ComboBox has **no AccessKit integration** for selection.

AccessKit's AT-SPI adapter checks:
```rust
// accesskit_atspi_common/src/node.rs
fn supports_selection(&self) -> bool {
    self.0.is_container_with_selectable_children()
}
```

**Current Code** (`crates/egui/src/containers/combo_box.rs`):
- No AccessKit code present
- Only basic `WidgetInfo` with `WidgetType::ComboBox`

**Fix Required**:

Add AccessKit integration to ComboBox:
```rust
// In ComboBox show method, when building the popup:
builder.set_children_selectable();  // or equivalent method

// For each selectable item:
item_builder.set_selected(is_selected);
```

Note: This may require significant refactoring as ComboBox currently doesn't track its items in a way that's compatible with AccessKit's selection model.

---

### 4. Text Interface (TextEdit)

**Symptom**: AT-SPI Text interface methods (`get_text`, `get_caret_position`, `get_text_selection`) fail with "Unknown property 'CharacterCount'"

**Root Cause**: `supports_text_ranges()` returns false because TextEdit doesn't create `Role::TextRun` child nodes.

AccessKit's `supports_text_ranges()` requires **both** conditions:
```rust
// accesskit/consumer/src/text.rs
pub fn supports_text_ranges(&self) -> bool {
    (self.is_text_input()
        || matches!(self.role(), Role::Label | Role::Document | Role::Terminal))
        && self.text_runs().next().is_some()  // <-- Requires TextRun children!
}
```

**Current Code** (`crates/egui/src/text_selection/accesskit_text.rs`):
- Sets text-related properties on the TextEdit node itself
- Does NOT create `Role::TextRun` child nodes for the text content

**Fix Required**:

egui needs to create child nodes with `Role::TextRun` for each text segment in the TextEdit:

```rust
// For each text run in the galley:
let text_run_builder = NodeBuilder::new(Role::TextRun);
text_run_builder.set_value(run_text);
text_run_builder.set_character_lengths(...);
text_run_builder.set_character_positions(...);
// ... add as child of TextEdit node
```

This is a more significant change as it requires restructuring how text accessibility is handled.

---

## Summary Table

| AT-SPI Interface | Affected Tools | Root Cause | Fix Complexity |
|------------------|----------------|------------|----------------|
| Value | `get_value`, `set_value` | Missing `set_numeric_value()` in Slider/DragValue | **Easy** - Add one line |
| Component | `get_bounds`, `focus_element`, `scroll_to_element` | Bounds set but possibly wrong coordinates or missing transform | **Medium** - Need debugging |
| Selection | `select_item`, `deselect_item`, `get_selected_count`, `select_all`, `clear_selection` | ComboBox has no AccessKit integration | **Hard** - Major refactoring |
| Text | `get_text`, `get_text_selection`, `set_text_selection`, `get_caret_position`, `set_caret_position` | Missing `Role::TextRun` child nodes | **Hard** - Restructure text accessibility |

## Recommended Fix Order

1. **Value Interface** (Easy Win)
   - Add `set_numeric_value()` to Slider and DragValue
   - Immediate benefit for screen readers and automation tools

2. **Component Interface** (Debug First)
   - Need to determine why bounds aren't exposed via AT-SPI
   - May be a simple coordinate/transform fix

3. **Text Interface** (Significant Work)
   - Requires adding TextRun child nodes
   - Consider if this is necessary for egui's use cases

4. **Selection Interface** (Major Work)
   - Requires rethinking ComboBox architecture
   - May need to track items in accessibility tree

## Testing

To verify fixes work correctly, use AT-SPI tools:

```bash
# Install AT-SPI debugging tools
sudo apt install accerciser at-spi2-core

# Run Accerciser to inspect accessibility tree
accerciser

# Or use command-line tools
# List accessible applications
busctl --user call org.a11y.atspi.Registry /org/a11y/atspi/accessible/root org.a11y.atspi.Accessible GetChildren
```

## Related Links

- [egui PR #2294 - Implement accessibility APIs via AccessKit](https://github.com/emilk/egui/pull/2294)
- [egui Issue #167 - Accessibility (A11y)](https://github.com/emilk/egui/issues/167)
- [AccessKit Repository](https://github.com/AccessKit/accesskit)
- [AT-SPI Documentation](https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/)
