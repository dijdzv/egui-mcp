# egui-mcp Roadmap

## Current Status (Phase 8 Complete)

All Phase 7 advanced features and Phase 8 testing/debugging features have been implemented.

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
| `is_visible` | Check if element is visible | AT-SPI State |
| `is_enabled` | Check if element is enabled | AT-SPI State |
| `is_focused` | Check if element has focus | AT-SPI State |
| `is_checked` | Check toggle/checkbox state | AT-SPI State |
| `screenshot_element` | Screenshot specific element | AT-SPI + IPC |
| `screenshot_region` | Screenshot specific region | IPC |
| `wait_for_element` | Wait for element to appear/disappear | Polling AT-SPI |
| `wait_for_state` | Wait for element state change | Polling AT-SPI |
| `compare_screenshots` | Compare two screenshots | Server (image-compare) |
| `diff_screenshots` | Generate visual diff image | Server (image-compare) |
| `highlight_element` | Highlight element with colored border | AT-SPI + IPC |
| `clear_highlights` | Remove all highlights | IPC |
| `save_snapshot` | Save current UI tree state | AT-SPI |
| `load_snapshot` | Load a saved snapshot | Memory |
| `diff_snapshots` | Compare two saved snapshots | Memory |
| `diff_current` | Compare current state with snapshot | AT-SPI + Memory |
| `get_logs` | Get recent log entries | IPC |
| `clear_logs` | Clear the log buffer | IPC |
| `get_frame_stats` | Get FPS and frame timing | IPC |
| `start_perf_recording` | Start recording performance | IPC |
| `get_perf_report` | Get performance report | IPC |

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
| State | `is_visible`, `is_enabled`, `is_focused`, `is_checked` | ✅ Working | |
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

## Phase 7: Advanced Features (Complete)

### State Queries ✅

| Tool | Description | Method | Status |
|------|-------------|--------|--------|
| `is_visible` | Check if element is visible | AT-SPI State | ✅ Done |
| `is_enabled` | Check if element is enabled | AT-SPI State | ✅ Done |
| `is_focused` | Check if element has focus | AT-SPI State | ✅ Done |
| `is_checked` | Check toggle/checkbox state | AT-SPI State | ✅ Done |

### Screenshot Enhancements ✅

| Tool | Description | Method | Status |
|------|-------------|--------|--------|
| `screenshot_element` | Screenshot specific element | AT-SPI + IPC (crop) | ✅ Done |
| `screenshot_region` | Screenshot specific region | IPC (crop) | ✅ Done |

### Wait/Polling Operations ✅

| Tool | Description | Method | Status |
|------|-------------|--------|--------|
| `wait_for_element` | Wait for element to appear/disappear | Polling AT-SPI | ✅ Done |
| `wait_for_state` | Wait for element state change | Polling AT-SPI | ✅ Done |

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

## Phase 8: Testing & Debugging Features (Future)

Inspired by [Playwright MCP](https://github.com/microsoft/playwright-mcp) and [Chrome DevTools MCP](https://github.com/ChromeDevTools/chrome-devtools-mcp).

### 8.1 Visual Regression Testing ✅

Compare screenshots to detect UI changes.

| Tool | Description | Parameters | Status |
|------|-------------|------------|--------|
| `compare_screenshots` | Compare two screenshots and return similarity score | `base64_a`, `base64_b`, `algorithm` | ✅ Done |
| `diff_screenshots` | Generate a visual diff image highlighting differences | `base64_a`, `base64_b` | ✅ Done |

**Implementation Details:**

- **Crate**: [`image-compare`](https://crates.io/crates/image-compare) (v0.4+)
- **Algorithms**: hybrid (default), MSSIM (structural), RMS (pixel-wise)
- **Location**: `egui-mcp-server` (pure server-side image processing)
- **Output**: Similarity score (0.0-1.0), diff image as base64 PNG

```rust
// Example implementation
use image_compare::{rgba_hybrid_compare, Algorithm};

pub fn compare_images(img_a: &[u8], img_b: &[u8]) -> Result<f64, Error> {
    let a = image::load_from_memory(img_a)?;
    let b = image::load_from_memory(img_b)?;
    let result = rgba_hybrid_compare(&a.to_rgba8(), &b.to_rgba8())?;
    Ok(result.score)
}
```

**Feasibility**: ✅ High - Pure Rust, no external dependencies

---

### 8.2 Element Highlight ✅

Visually highlight elements for debugging.

| Tool | Description | Parameters | Status |
|------|-------------|------------|--------|
| `highlight_element` | Draw highlight overlay on element | `id`, `color`, `duration_ms` | ✅ Done |
| `clear_highlights` | Remove all highlights | - | ✅ Done |

**Implementation Details:**

- **Method**: IPC request to egui client to draw overlay via `Context::debug_painter()`
- **Server**: Gets element bounds via AT-SPI, sends highlight request via IPC
- **Client**: Stores highlights and draws them in `draw_highlights()` helper
- **Protocol**: `Request::HighlightElement { x, y, width, height, color, duration_ms }`, `Request::ClearHighlights`

**Usage in egui app:**

```rust
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ... your UI code ...

        // Draw highlights at the end
        let highlights = self.runtime.block_on(self.mcp_client.get_highlights());
        egui_mcp_client::draw_highlights(ctx, &highlights);
    }
}
```

**Feasibility**: ✅ Done - egui debug_painter works well

---

### 8.3 Snapshot Diff ✅

Compare UI tree states to detect changes.

| Tool | Description | Parameters | Status |
|------|-------------|------------|--------|
| `save_snapshot` | Save current UI tree state | `name` | ✅ Done |
| `load_snapshot` | Load a saved snapshot | `name` | ✅ Done |
| `diff_snapshots` | Compare two snapshots | `name_a`, `name_b` | ✅ Done |
| `diff_current` | Compare current state with saved snapshot | `name` | ✅ Done |

**Implementation Details:**

- **Method**: JSON serialization of `UiTree`, then structural diff
- **Crate**: [`similar`](https://crates.io/crates/similar) or custom tree diff
- **Storage**: In-memory HashMap or file-based (`/tmp/egui-mcp-snapshots/`)
- **Output**: List of added/removed/modified nodes

```rust
#[derive(Debug)]
enum TreeDiff {
    Added { node: NodeInfo },
    Removed { node: NodeInfo },
    Modified { id: u64, field: String, old: String, new: String },
}

fn diff_trees(a: &UiTree, b: &UiTree) -> Vec<TreeDiff> {
    let a_map: HashMap<u64, &NodeInfo> = a.nodes.iter().map(|n| (n.id, n)).collect();
    let b_map: HashMap<u64, &NodeInfo> = b.nodes.iter().map(|n| (n.id, n)).collect();

    let mut diffs = vec![];
    // Find added/removed/modified...
    diffs
}
```

**Feasibility**: ✅ High - Pure data manipulation, no external dependencies required

---

### 8.4 Performance Metrics ✅

Measure rendering performance.

| Tool | Description | Parameters | Status |
|------|-------------|------------|--------|
| `get_frame_stats` | Get FPS and frame timing | - | ✅ Done |
| `start_perf_recording` | Start recording performance data | `duration_ms` | ✅ Done |
| `get_perf_report` | Get recorded performance report | - | ✅ Done |

**Usage in egui app:**

```rust
impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // ... your UI code ...

        // Record frame for performance metrics (1 line only!)
        self.runtime.block_on(self.mcp_client.record_frame_auto());
    }
}
```

**MCP Tool Output Example:**

```json
// get_frame_stats
{"fps": 65.5, "frame_time_ms": 15.2, "frame_time_min_ms": 10.5, "frame_time_max_ms": 25.0, "sample_count": 120}

// get_perf_report (after recording)
{"duration_ms": 5000, "total_frames": 300, "avg_fps": 60.0, "p95_frame_time_ms": 18.0, "p99_frame_time_ms": 22.0}
```

---

### 8.5 Console/Log Access ✅

Access application logs via MCP.

| Tool | Description | Parameters | Status |
|------|-------------|------------|--------|
| `get_logs` | Get recent log entries | `level`, `limit` | ✅ Done |
| `clear_logs` | Clear log buffer | - | ✅ Done |

**Usage in egui app:**

```rust
use egui_mcp_client::{McpClient, McpLogLayer};
use tracing_subscriber::prelude::*;

fn main() {
    // Set up MCP log layer (replaces tracing_subscriber::fmt::init())
    let (mcp_layer, log_buffer) = McpLogLayer::new(1000);

    tracing_subscriber::registry()
        .with(mcp_layer)                           // Capture logs for MCP
        .with(tracing_subscriber::fmt::layer())    // Also log to stdout
        .init();

    // Pass log buffer to MCP client
    let mcp_client = McpClient::new().with_log_buffer_sync(log_buffer);
    // ... run egui app
}
```

**MCP Tool Output Example:**

```json
// get_logs with limit=3
{
  "count": 3,
  "entries": [
    {"level": "INFO", "target": "my_app", "message": "Starting...", "timestamp_ms": 1234567890},
    {"level": "DEBUG", "target": "my_app::ui", "message": "Button clicked", "timestamp_ms": 1234567891},
    {"level": "WARN", "target": "my_app", "message": "Connection slow", "timestamp_ms": 1234567892}
  ]
}
```

**Note**: Only captures logs from the egui app process (not system logs)

---

### Phase 8 Summary

| Feature | Tools | Status | Dependencies |
|---------|-------|--------|--------------|
| Visual Regression | 2 | ✅ Done | `image-compare` (in server) |
| Element Highlight | 2 | ✅ Done | IPC protocol addition |
| Snapshot Diff | 4 | ✅ Done | None |
| Performance Metrics | 3 | ✅ Done | Client changes |
| Console/Log Access | 2 | ✅ Done | `tracing-subscriber` (in client) |

**Phase 8 Complete!** All 13 tools implemented.

---

## References

- [Playwright MCP](https://github.com/microsoft/playwright-mcp) - Browser automation MCP
- [Chrome DevTools MCP](https://github.com/ChromeDevTools/chrome-devtools-mcp) - Chrome debugging MCP
- [Playwright Actions](https://playwright.dev/docs/input)
- [AT-SPI Documentation](https://www.freedesktop.org/wiki/Accessibility/AT-SPI2/)
- [atspi-proxies Rust crate](https://docs.rs/atspi-proxies)
- [image-compare crate](https://crates.io/crates/image-compare) - Image comparison algorithms
- [tracing-subscriber](https://docs.rs/tracing-subscriber) - Composable logging layers
