//! Roles and small state enums of the accessibility layer. They are zaxis' own types, so a
//! new major version of AccessKit does not change the public API.

/// What a widget is to assistive technology.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum AccessRole {
    /// A container without a meaning of its own.
    #[default]
    Group,
    Window,
    Pane,
    Region,
    Dialog,
    AlertDialog,
    Label,
    Paragraph,
    Heading,
    Link,
    Image,
    Button,
    CheckBox,
    Switch,
    RadioGroup,
    RadioButton,
    Slider,
    SpinButton,
    TextInput,
    MultilineTextInput,
    ComboBox,
    ListBox,
    ListBoxOption,
    List,
    ListItem,
    Tree,
    TreeItem,
    Table,
    Grid,
    Row,
    ColumnHeader,
    RowHeader,
    Cell,
    TabList,
    Tab,
    TabPanel,
    MenuBar,
    Menu,
    MenuItem,
    MenuItemCheckBox,
    MenuItemRadio,
    Tooltip,
    Status,
    Alert,
    ScrollView,
    ScrollBar,
    Splitter,
    ProgressIndicator,
    ColorWell,
    Toolbar,
    TitleBar,
    Canvas,
}

impl AccessRole {
    /// Roles a screen reader announces by name: a node of such a role without a label is
    /// reported through [`DiagnosticKind::MissingAccessibleName`](crate::DiagnosticKind).
    pub(crate) fn needs_name(self) -> bool {
        matches!(
            self,
            Self::Button
                | Self::CheckBox
                | Self::Switch
                | Self::RadioButton
                | Self::Slider
                | Self::SpinButton
                | Self::TextInput
                | Self::MultilineTextInput
                | Self::ComboBox
                | Self::Link
                | Self::Image
                | Self::Tab
                | Self::MenuItem
                | Self::MenuItemCheckBox
                | Self::MenuItemRadio
                | Self::ColorWell
                | Self::Dialog
                | Self::AlertDialog
        )
    }

    /// Roles whose text is their value rather than a label.
    pub(crate) fn is_text(self) -> bool {
        matches!(self, Self::Label | Self::Paragraph | Self::Heading)
    }
}

/// State of a two- or three-state control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessToggled {
    False,
    True,
    Mixed,
}

impl From<bool> for AccessToggled {
    fn from(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }
}

/// How urgently a change of a live region is announced.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessLive {
    /// Announced when the screen reader is idle.
    Polite,
    /// Interrupts the current announcement.
    Assertive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessOrientation {
    Horizontal,
    Vertical,
}

/// An action a node accepts from assistive technology; see [`AccessNode::action`].
///
/// [`AccessNode::action`]: super::AccessNode::action
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum AccessActionKind {
    Click,
    Focus,
    Expand,
    Collapse,
    Increment,
    Decrement,
    SetValue,
    ReplaceSelectedText,
    SetTextSelection,
    ScrollIntoView,
    Scroll,
    ShowContextMenu,
    ShowTooltip,
}

/// One request from assistive technology, delivered to the node it targets on the next pass.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum AccessAction {
    Click,
    Expand,
    Collapse,
    Increment,
    Decrement,
    /// A new value as text, for text fields and combo boxes.
    SetValue(String),
    /// A new value for a range control. May be out of range or not finite.
    SetNumericValue(f64),
    ReplaceSelectedText(String),
    /// Byte offsets into the node's value; each is a grapheme boundary.
    SetTextSelection {
        anchor: usize,
        focus: usize,
    },
    ScrollIntoView,
    /// Scroll by this many pages (`±1.0`) or lines (`±0.1`) on each axis.
    ScrollBy(crate::Vec2),
    SetScrollOffset(crate::Vec2),
    ShowContextMenu,
    ShowTooltip,
    HideTooltip,
}
