# GPUI Kit: Details

Detail sections moved out of `SKILL.md` to keep it short.

## Component Catalog

Import paths are relative to `gpui_kit::component::`, so `input::{Input, InputState}`
means `use gpui_kit::component::input::{Input, InputState};`. For the full API
fetch the component's `.md` doc.

### Input & Form

| Component     | Import                                          | Notes                                        |
| ------------- | ----------------------------------------------- | -------------------------------------------- |
| `Input`       | `input::{Input, InputState}`                    | Stateful. Text, password, mask, validation   |
| `Textarea`    | `input::{Textarea, TextareaState}`              | Stateful. Multi-line text                    |
| `Editor`      | `input::{Editor, EditorState}`                  | Stateful. Code editor, `tree-sitter` feature |
| `NumberInput` | `input::{NumberInput, NumberInputEvent}`        | Stateful. Numeric with step                  |
| `OtpInput`    | `input::OtpInput`                               | Stateful. One-time password                  |
| `SpeechButton` | `speech::{SpeechButton, SpeechState}`          | Stateful. Dictation, `speech` feature        |
| `Select`      | `select::{Select, SelectState}`                 | Stateful. Dropdown picker                    |
| `Combobox`    | `combobox::{Combobox, ComboboxState}`           | Stateful. Searchable select                  |
| `Checkbox`    | `checkbox::Checkbox`                            | Stateless. `on_click` receives `&bool`       |
| `Switch`      | `switch::Switch`                                | Stateless. Toggle                            |
| `Radio`       | `radio::{Radio, RadioGroup}`                    | Stateless.                                   |
| `Slider`      | `slider::{Slider, SliderState}`                 | Stateful.                                    |
| `Toggle`      | `button::Toggle`                                | Stateless.                                   |
| `Rating`      | `rating::Rating`                                | Stateless.                                   |
| `Stepper`     | `stepper::Stepper`                              | Stateless. Multi-step progress               |
| `ColorPicker` | `color_picker::{ColorPicker, ColorPickerState}` | Stateful.                                    |
| `DatePicker`  | `date_picker::{DatePicker, DatePickerState}`    | Stateful.                                    |
| `TimeField`   | `time_field::{TimeField, TimeFieldState}`       | Stateful. Time of day, 24/12-hour            |
| `Calendar`    | `calendar::{Calendar, CalendarState}`           | Stateful. Inline month view                  |
| `Form`        | `form::{v_form, h_form, field}`                 | Layout container for form fields             |

### Display & Feedback

| Component   | Import                                    | Notes                                 |
| ----------- | ----------------------------------------- | ------------------------------------- |
| `Button`    | `button::{Button, ButtonGroup}`           | Stateless. Primary UI action          |
| `Icon`      | `{Icon, IconName}`                        | Stateless. Lucide icons               |
| `Badge`     | `badge::Badge`                            | Stateless.                            |
| `Tag`       | `tag::Tag`                                | Stateless. Closable tags              |
| `Avatar`    | `avatar::Avatar`                          | Stateless.                            |
| `Label`     | `label::Label`                            | Stateless. Form label                 |
| `Kbd`       | `kbd::Kbd`                                | Stateless. Keyboard key display       |
| `Alert`     | `alert::Alert`                            | Stateless. Info/success/warning/error |
| `Spinner`   | `spinner::Spinner`                        | Stateless. Loading indicator          |
| `Skeleton`  | `skeleton::Skeleton`                      | Stateless. Loading placeholder        |
| `Shimmer`   | `shimmer::{ShimmerText, ShimmerStyle}`    | Stateless. Streaming-text shimmer     |
| `Marker`    | `marker::{Marker, MarkerVariant}`         | Stateless. Inline status marker       |
| `Progress`  | `progress::{Progress, ProgressCircle}`    | Stateless.                            |
| `Tooltip`   | `tooltip::Tooltip`                        | Via `.tooltip()` on elements          |
| `HoverCard` | `hover_card::{HoverCard, HoverCardState}` | Stateful.                             |
| `Clipboard` | `clipboard::Clipboard`                    | Stateless. Copy button                |
| `TextView`  | `text::TextView`                          | `TextView::markdown(id, text)`, HTML too |
| Image       | `gpui_kit::{img, ImageSource, ObjectFit}` | GPUI's `img()` element                |

### Overlay & Popups

| Component        | Import                                            | Notes                                    |
| ---------------- | ------------------------------------------------- | ---------------------------------------- |
| `Dialog`         | `dialog::Dialog` + `WindowExt`                    | Via `window.open_dialog(...)`            |
| `AlertDialog`    | `WindowExt`                                       | Via `window.open_alert_dialog(...)`      |
| `Sheet`          | `sheet::Sheet` + `WindowExt`                      | Side panel, via `window.open_sheet(...)` |
| `Notification`   | `notification::Notification` + `WindowExt`        | Via `window.push_notification(...)`      |
| `Popover`        | `popover::Popover`                                | Floating overlay                         |
| `Menu`           | `menu::{PopupMenu, DropdownMenu}`                 | Context menus                            |
| `DropdownButton` | `button::DropdownButton`                          | Button with dropdown menu                |
| `Command`        | `command::{Command, CommandState, CommandGroup}`  | Stateful. Command palette                |
| Focus trap       | `FocusTrapElement`                                | `.focus_trap(id, &handle)` on a container; `Dialog` and `Sheet` have it built in |

### Navigation & Layout

| Component         | Import                                                                   | Notes                     |
| ----------------- | ------------------------------------------------------------------------ | ------------------------- |
| `Tabs` / `TabBar` | `tab::{Tab, TabBar}`                                                     | Tabbed interface          |
| `Sidebar`         | `sidebar::{Sidebar, SidebarMenu, ...}`                                   | App navigation panel      |
| `TitleBar`        | `TitleBar`                                                               | Window title bar          |
| `StatusBar`       | `status_bar::StatusBar`                                                  | Window status bar         |
| `Breadcrumb`      | `breadcrumb::Breadcrumb`                                                 | Navigation breadcrumb     |
| `Pagination`      | `pagination::Pagination`                                                 | Page navigation           |
| `Accordion`       | `accordion::Accordion`                                                   | Collapsible sections      |
| `Collapsible`     | `collapsible::Collapsible`                                               | Single collapsible        |
| `GroupBox`        | `group_box::GroupBox`                                                    | Labeled container         |
| `Resizable`       | `resizable::{h_resizable, v_resizable, resizable_panel, ResizableState}` | Draggable split panes     |
| `ResizeHandle`    | `resizable::{resize_handle, HandleEdge}`                                 | Standalone resize edge    |
| `Scrollbar`       | `scroll::Scrollbar`                                                      | Custom scrollbar          |

### Data Display

| Component         | Import                                          | Notes                         |
| ----------------- | ----------------------------------------------- | ----------------------------- |
| `DataTable`       | `table::{DataTable, TableState, TableDelegate}` | Stateful. Full-featured table |
| `Diff`            | `diff::{Diff, DiffState, DiffFile}`           | Retained state, `RenderOnce` element. Readonly unified/Git diff display of one or more files, merge conflicts and source files; application supplies the patch |
| `Table`           | `table::{Table, ...}`                           | Simpler table                 |
| `VirtualList`     | `{v_virtual_list, h_virtual_list}`              | High-perf large lists         |
| `List`            | `list::{List, ListState, ListDelegate}`         | Stateful. Searchable list     |
| `Tree`            | `tree::{Tree, TreeState, TreeItem, TreeEntry}`  | Stateful. Hierarchy           |
| `DescriptionList` | `description_list::DescriptionList`             | Key-value pairs               |
| `Settings`        | `setting::Settings`                             | Settings panel                |

### Chat & Messaging

| Component         | Import                                                   | Notes                                 |
| ----------------- | -------------------------------------------------------- | ------------------------------------- |
| `Message`         | `message::{Message, MessageContent, MessageAlignment}`   | Stateless. Chat message row           |
| `Bubble`          | `bubble::{Bubble, BubbleContent, BubbleVariant}`         | Stateless. Message bubble             |
| `Attachment`      | `attachment::{Attachment, AttachmentContent, ...}`       | Stateless. File/media attachment card |
| `MessageScroller` | `message_scroller::{MessageScroller, MessageScrollerState}` | Stateful. Auto-scrolling message list |

### Charts

| Component | Import                                                          | Notes                          |
| --------- | --------------------------------------------------------------- | ------------------------------ |
| `Chart`   | `chart::{AreaChart, BarChart, LineChart, PieChart, RadarChart}` | Bar, line, area, pie charts    |
| `Plot`    | `plot::Plot`                                                    | `#[derive(IntoPlot)]` for data |
