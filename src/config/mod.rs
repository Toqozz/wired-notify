use serde::Deserialize;
use crate::types::tree::{ Node, Tree };

#[derive(Debug, Deserialize)]
pub struct Config {
    pub max_notifications: u32,
    pub gap: i32,
    pub notification: NotificationConfig,
    pub shortcuts: ShortcutsConfig,
}

#[derive(Debug, Deserialize)]
pub struct NotificationConfig {
    pub root: LayoutBlock,

    pub font: String,

    //pub layout: Tree<LayoutElement>,

    //pub summary: TextArea,
    //pub body: TextArea,

    // Geometry.
    pub width: u32,
    pub height: u32,            // Base height.  NOTE: notification windows will generally be resized, ignoring this value.
    pub x: i32,
    pub y: i32,

    pub border_width: f64,

    pub background_color: Color,
    pub border_color: Color,
    pub text_color: Color,

    pub timeout: f32,           // Default timeout.

    pub scroll_speed: f32,
    pub bounce: bool,

    // Undecided...
    //bounce_margin: u32,
    //rounding: u32,
}

/*
#[derive(Debug, Deserialize)]
pub struct TextArea {
    pub anchor: FieldType,
    pub anchor_position: AnchorPosition,

    pub font: String,

    pub color: Color,

    pub width: f64,
    pub max_lines: f64,

    pub offset: Offset,
    pub padding: Padding,
}
*/

#[derive(Debug, Deserialize)]
pub struct ShortcutsConfig {
    pub notification_close: u32,
    pub notification_closeall: u32,
    pub notification_pause: u32,
    pub notification_url: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Padding {
    pub left: f64,
    pub right: f64,
    pub top: f64,
    pub bottom: f64,
}

#[derive(Debug, Deserialize)]
pub struct Offset {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize)]
pub enum FieldType {
    Root,
    Icon,
    Summary,
    Body,
}

#[derive(Debug, Deserialize, Clone)]
pub enum AnchorPosition {
    TL,
    TR,
    BL,
    BR,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Color {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

#[derive(Debug, Deserialize)]
pub struct LayoutBlock {
    pub field: FieldType,
    pub hook: AnchorPosition,
    pub offset: Offset,
    pub padding: Padding,
    pub children: Vec<LayoutBlock>,
}

/*
// Tree structure.
pub struct LayoutElement {
    pub hook: AnchorPosition,
}


pub fn construct_layouts(root_block: &LayoutBlock) -> Tree<LayoutElement> {
    let root_data = LayoutElement {
        hook: AnchorPosition::TL,
    };

    //let mut root = Node::new(root_data);
    let mut tree = Tree::new();
    let root = tree.insert_root(root_data);

    fn descend(tree: &mut Tree<LayoutElement>, block: &LayoutBlock, parent: usize) {
        for child in &block.children {
            let data = LayoutElement { hook: child.hook.clone() };
            let node = tree.insert(data, parent);
            descend(tree, child, node);
        }
    }

    descend(&mut tree, root_block, root);

    tree
}
*/
