use zaxis::{Id, TreeModel, TreeNode};
pub struct Files;
impl Files {
    pub fn name(&self, id: Id) -> &'static str {
        if id == Id::new("src") {
            "src"
        } else {
            ["main.rs", "layout.rs", "input.rs"]
                .into_iter()
                .find(|name| Id::new(name) == id)
                .unwrap_or("src")
        }
    }
}
impl TreeModel for Files {
    fn revision(&self) -> u64 {
        1
    }
    fn roots(&self) -> impl Iterator<Item = Id> {
        [Id::new("src")].into_iter()
    }
    fn children(&self, node: Id) -> impl Iterator<Item = Id> {
        ["main.rs", "layout.rs", "input.rs"]
            .into_iter()
            .filter(move |_| node == Id::new("src"))
            .map(Id::new)
    }
    fn node(&self, node: Id) -> Option<TreeNode<'_>> {
        Some(if node == Id::new("src") {
            TreeNode::branch("src")
        } else {
            TreeNode::leaf(self.name(node))
        })
    }
    fn parent(&self, node: Id) -> Option<Id> {
        (node != Id::new("src")).then_some(Id::new("src"))
    }
}
