mod clashsrvctl_tab;
mod profile_tab;
mod profile_input;

pub use clashsrvctl_tab::ClashSrvCtlTab;
pub use profile_tab::ProfileTab;


pub trait CommonTab {
    fn draw<B: ratatui::backend::Backend>(&mut self, f: &mut ratatui::Frame<B>, area: ratatui::layout::Rect);
    // This should be impled, but rustc won't recognize it
    fn event(&mut self, ev: &crossterm::event::Event) -> Result<super::EventState, ()>;
    // Desprate HashMap<_,Box<dyn CommonTab>>
    // fn as_any(&self) -> &dyn std::any::Any;
    // just return &self
}

pub enum Tabs {
    ProfileTab(std::cell::RefCell<ProfileTab>),
    ClashsrvctlTab(std::cell::RefCell<ClashSrvCtlTab>),
}