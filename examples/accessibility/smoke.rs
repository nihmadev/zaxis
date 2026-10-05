//! The form without a window, a GPU or a screen reader: build it, check the accessibility
//! tree against `accesskit_consumer` (the model platform adapters are built on) and operate
//! it the way assistive technology does.
use crate::{
    model::{Model, Page, Plan},
    ui,
};
use accesskit_consumer::{NodeRef, Tree, TreeChangeHandler};
use zaxis::accesskit::{Action, ActionData, NodeId, Role, Toggled};
use zaxis::testing::AccessTree;
use zaxis::{winit::dpi::PhysicalSize, Context, Id};

struct Changes;
impl TreeChangeHandler for Changes {
    fn node_added(&mut self, _: &NodeRef) {}
    fn node_updated(&mut self, _: &NodeRef, _: &NodeRef) {}
    fn focus_moved(&mut self, _: Option<&NodeRef>, _: Option<&NodeRef>) {}
    fn node_removed(&mut self, _: &NodeRef) {}
}

struct Smoke {
    context: Context,
    model: Model,
    tree: AccessTree,
    consumer: Option<Tree>,
}

impl Smoke {
    /// A few passes: a request needs one to apply and one more to show everywhere.
    fn settle(&mut self) {
        for _ in 0..4 {
            self.context
                .run(|context| ui::show(context, &mut self.model));
            if self.tree.sync(&mut self.context).is_some() {
                let update = self.tree.last.clone().expect("an update was applied");
                match &mut self.consumer {
                    // An update the consumer rejects would panic a platform adapter.
                    Some(consumer) => consumer.update_and_process_changes(update, &mut Changes),
                    None => self.consumer = Some(Tree::new(update, true)),
                }
            }
            self.tree.validate();
        }
    }

    fn find(&self, role: Role, name: &str) -> Result<NodeId, String> {
        self.tree.find(role, name).ok_or_else(|| {
            let all = self.tree.all(role);
            let names: Vec<_> = all.iter().map(|id| self.tree.name(*id)).collect();
            format!("no {role:?} named {name:?} in the tree; there are {names:?}")
        })
    }

    fn act(&mut self, role: Role, name: &str, action: Action) -> Result<(), String> {
        self.act_with(role, name, action, None)
    }

    fn act_with(
        &mut self,
        role: Role,
        name: &str,
        action: Action,
        data: Option<ActionData>,
    ) -> Result<(), String> {
        let target = self.find(role, name)?;
        if !self.tree.act(&mut self.context, target, action, data) {
            return Err(format!("{role:?} {name:?} refused {action:?}"));
        }
        self.settle();
        Ok(())
    }
}

fn check(condition: bool, what: &str) -> Result<(), String> {
    condition.then_some(()).ok_or_else(|| what.to_owned())
}

pub fn run() -> Result<(), String> {
    let mut context = Context::new();
    context.set_viewport(PhysicalSize::new(900, 780), 1.0);
    let tree = AccessTree::attach(&mut context);
    let mut smoke = Smoke {
        context,
        model: Model::new(),
        tree,
        consumer: None,
    };
    smoke.settle();
    let problems = smoke.context.diagnostics();
    check(problems.is_empty(), &format!("diagnostics: {problems:?}"))?;
    for (role, name) in [
        (Role::TextInput, "Name"),
        (Role::TextInput, "Email"),
        (Role::MultilineTextInput, "Notes"),
        (Role::Image, "zaxis logo"),
        (Role::Button, "Refresh"),
        (Role::Link, "Documentation"),
        (Role::ComboBox, "Region"),
    ] {
        smoke.find(role, name)?;
    }

    smoke.act(Role::Button, "Refresh", Action::Click)?;
    check(
        smoke.model.refreshed == 1,
        "the icon button was not clicked once",
    )?;
    smoke.act(Role::CheckBox, "Subscribe to updates", Action::Click)?;
    check(!smoke.model.subscribe, "the checkbox did not toggle")?;
    let subscribe = smoke.find(Role::CheckBox, "Subscribe to updates")?;
    check(
        smoke.tree.node(subscribe).toggled() == Some(Toggled::False),
        "the tree does not show the checkbox off",
    )?;
    smoke.act(Role::RadioButton, "Team", Action::Click)?;
    check(smoke.model.plan == Plan::Team, "the plan did not change")?;
    let volume = Some(ActionData::NumericValue(72.0));
    smoke.act_with(Role::Slider, "Volume", Action::SetValue, volume)?;
    check(
        smoke.model.volume == 70.0,
        "the slider did not snap to its step",
    )?;
    let name = Some(ActionData::Value("Grace Hopper".into()));
    smoke.act_with(Role::TextInput, "Name", Action::SetValue, name)?;
    check(smoke.model.name == "Grace Hopper", "the name was not set")?;
    let email = Some(ActionData::Value("nobody".into()));
    smoke.act_with(Role::TextInput, "Email", Action::SetValue, email)?;
    let field = smoke.find(Role::TextInput, "Email")?;
    check(
        smoke.tree.node(field).invalid().is_some(),
        "a bad address is not marked invalid",
    )?;

    smoke.act(Role::Button, "Save", Action::Click)?;
    check(smoke.model.saved == 1, "Save did not run once")?;
    let announced = smoke.tree.all(Role::Status).into_iter().any(|id| {
        smoke.tree.name(id).contains("Profile saved") && smoke.tree.node(id).live().is_some()
    });
    check(announced, "the toast is not a live region")?;

    smoke.act(Role::Tab, "Data", Action::Click)?;
    check(smoke.model.page == Page::Data, "the tab did not switch")?;
    smoke.act(Role::ListBoxOption, "report.pdf", Action::Click)?;
    check(
        smoke.model.selected_files.len() == 1,
        "the list row was not selected",
    )?;
    smoke.act(Role::TreeItem, "src", Action::Collapse)?;
    check(
        !smoke.model.open_folders.contains(&Id::new("src")),
        "the tree node did not collapse",
    )?;
    check(
        smoke.tree.find(Role::TreeItem, "main.rs").is_none(),
        "a collapsed folder still lists its files",
    )?;

    smoke.act(Role::MenuItem, "Help", Action::Click)?;
    smoke.act(Role::MenuItem, "About", Action::Click)?;
    check(smoke.model.about, "the menu item did not open the dialog")?;
    let dialog = smoke.find(Role::Dialog, "About")?;
    check(
        smoke.tree.node(dialog).is_modal(),
        "the dialog is not modal",
    )?;
    check(
        smoke.tree.find(Role::Tab, "Data").is_none(),
        "the form behind the dialog is still in the tree",
    )?;
    smoke.act(Role::Button, "Close", Action::Click)?;
    check(!smoke.model.about, "the dialog did not close")?;
    smoke.find(Role::Tab, "Data")?;
    Ok(())
}
