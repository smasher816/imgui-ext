mod support;

use imgui::ImString;

#[derive(imgui_ext::Gui, Debug)]
pub struct Example {
    #[imgui(combobox(label = "choose one", selected = "1"))]
    vec: [ImString; 3],
}

impl<'a> Default for Example {
    fn default() -> Self {
        Self {
            vec: [
                ImString::new("Foo"), 
                ImString::new("Bar"), 
                ImString::new("Baz")
            ],
        }
    }
}

fn main() {
    support::demo().run_debug::<Example, _>(|_, _| {});
}
